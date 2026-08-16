# Establishing the Windows logon for a local account

How a completed browser sign-in becomes a real interactive Windows session,
why that still involves a password, and why the password is now stored rather
than regenerated on every logon.

## Why there is a password at all

Once the browser reports a successful sign-in, the provider has to hand
winlogon something LSA will accept. Two passwordless routes exist on paper.
Both are shut.

**S4U cannot do it.** `MSV1_0_S4U_LOGON` mints a token without credentials,
which is genuinely useful for launching a process as some account, but it
cannot establish *this* logon:

1. A credential provider does not perform the logon. `GetSerialization`
   returns a credential blob and an auth-package id; winlogon hands those to
   LSA and LSA creates the session. `CREDENTIAL_PROVIDER_CREDENTIAL_SERIALIZATION`
   has no field for a token you already made.
2. The token S4U produces lives in the calling process. It is not, and cannot
   become, the interactive desktop session winlogon is about to set up.
3. An S4U logon is identity-only — no credential material in the session. Even
   if it could be injected, the resulting desktop would have no DPAPI master
   key, so Credential Manager, EFS and saved browser and wifi passwords would
   all be broken.

**A custom LSA authentication package cannot be shipped.** This is the
mechanism that genuinely does passwordless logon: register an authentication
package, have the provider look that package up instead of `Negotiate` (one
line, in `syscalls.rs::negotiate_package`), pack an arbitrary blob such as the
authentik token, and let the package's `LsaApLogonUser` validate it and call
`CreateLogonSession`.

It was built and abandoned. `ee/wcp/ak_lsa` in the C++ tree, deleted in
`57d1bd55` — recoverable with `git show 57d1bd55~1:ee/wcp/ak_lsa/Main.cpp`. It
implemented `SpLsaModeInitialize`, `SpInitialize`, `LsaApLogonUser` and
`CreateLogonSession`; `PrepareToken.cpp` assembled the token by hand and
`PrepareProfile.cpp` built the session profile. It compiled, but was never
registered under `Authentication Packages` and never called from the logon
path.

It was abandoned because of Microsoft's signing requirements for LSA plugins.
Under LSA protection, LSASS runs as a protected process and refuses to load
plugins that are not Microsoft-signed; an EV certificate does not satisfy
this. That gate is getting tighter, not looser — LSA protection is default-on
for new Windows 11 installs from 22H2 onward.

**Treat this as closed.** It is not a risk to be managed carefully, it is a
capability a third party cannot ship. Do not restart it without first
establishing that Microsoft will sign the package.

For domain-joined machines there is a real passwordless answer — authentik
issues a short-lived certificate and Windows performs PKINIT/smartcard-style
logon, broadly the mechanism behind Windows Hello for Business. It needs PKI
and KDC trust, and does nothing for standalone machines. For local accounts,
a password it is; the design question is only how it is managed.

## `reset` versus `change`

Everything below turns on this distinction.

`NetUserSetInfo` level 1003 is an **administrative reset**. It does not need
the old password, which is exactly why it is dangerous: the user's DPAPI
master key is encrypted with the old password, and nothing can re-encrypt it,
so it is orphaned. This is the operation Windows warns "might cause
irreversible loss of information" for. EFS-encrypted files, personal
certificates and everything in Credential Manager become unreadable.

`NetUserChangePassword` is a **self-service change**. Supplying the old
password lets LSA re-encrypt the master key, so nothing is lost. It also works
on an expired password, which is the normal "your password has expired, change
it now" flow.

The provider originally called `reset` on every single sign-in. The loss was
therefore not a one-off risk but a guaranteed, repeating one.

## What happens now

`credential.rs::Connect`, for a local account:

1. Load the stored password for this account's SID.
2. If there is one, check it with a network `LogonUserW`:
   - **valid** — reuse it, no account modification at all. The common path.
   - **expired** — we know it, so `change` it for a fresh one and use that.
   - **rejected** — it was changed out of band; fall through to (3).
   - **inconclusive** — reuse it anyway. See below.
3. Otherwise generate a password, `reset` the account to it, and store it.

`GetSerialization` packs whichever password that produced, records it, and
`ReportResult` rotates it with `change` once the logon has actually succeeded
(`STATUS_SUCCESS`). Rotation is best-effort: a failure leaves the stored
password valid and the next sign-in works.

So `reset` — the lossy call — now runs on first use and on out-of-band
change, and nowhere else.

### Why an inconclusive check reuses the password

`LogonUserW` failing does not mean the password is wrong. A policy denying
network logons, a locked-out or disabled account, an unreachable SAM: all of
these produce errors that say nothing about the credential. Only
`ERROR_LOGON_FAILURE` is treated as a rejection, and only
`ERROR_PASSWORD_EXPIRED` / `ERROR_PASSWORD_MUST_CHANGE` as expiry.

Treating anything else as "wrong password" would trigger a reset, destroying
the master key on a guess — precisely the damage this design exists to
prevent. Reusing a password that turns out to be wrong costs one failed logon,
after which the account state has changed and the check has a definite answer.

## Where the secret lives

Windows Credential Manager, via `ak-platform-keyring`'s `WindowsStore`:

- **SYSTEM's vault.** The provider is loaded into LogonUI, which runs as
  SYSTEM. Generic credentials are per-logon-account, so this is SYSTEM's own
  vault and no interactive user can read it.
- **`CRED_PERSIST_LOCAL_MACHINE`.** The keyring's default is Enterprise, which
  roams with the profile on a domain. A machine-local account password is
  meaningless anywhere else, so roaming it is pure exposure.
- **Keyed by SID**, under service `io.goauthentik.agent.wcp-account-password`,
  so renaming the account does not orphan the entry.
- `WindowsStore` is constructed directly rather than through
  `ak_platform_keyring::store()`, because that resolves to the in-memory store
  under `debug_assertions` and would silently lose the password between
  LogonUI processes in every development build.

A consequence worth knowing: under `CPUS_CREDUI` the provider runs as the
interactive user, not SYSTEM, and therefore sees a different vault. It will
find no stored password and fall back to the reset path.

## Known limits

- **Minimum password age.** A policy setting one makes
  `NetUserChangePassword` fail with `NERR_PasswordTooRecent` on every logon.
  Rotation quietly stops happening; the stored password stays valid and
  sign-in is unaffected.
- **Change succeeds, store fails.** A narrow window where the account has a
  password the vault does not know. The next sign-in's validity check catches
  it and recovers with a reset, at the cost of one orphaned master key.
- **Domain accounts are unresolved.** `Connect` generates a throwaway password
  for them and never sets it anywhere, so that logon cannot succeed as
  written. The certificate/PKINIT path above is the real answer, not anything
  in this file.
- **DPAPI survival is unverified on real hardware.** The reasoning above says
  it should hold; nobody has watched it. `e2e/README.md` has the checklist.
