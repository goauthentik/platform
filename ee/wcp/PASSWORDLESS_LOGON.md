# Establishing the Windows logon without resetting the password

Background and a closed door. Read this before proposing a way to stop
`Connect` randomising local account passwords — the obvious answers have been
tried, and one of them is permanently unavailable.

Not to be confused with `BROWSER_PRIVILEGE.md`, which is about what
`ak_cef.exe` runs *as*. This file is about how the interactive logon session
itself gets established.

## What happens today

Once the browser reports a successful sign-in, `credential.rs::Connect`
generates a random password, calls `NetUserSetInfo` level 1003 to set the local
account to it (`syscalls.rs::RealSyscalls::reset`), and hands that
username/password pair to LSA from `GetSerialization` — packed as a
`KERB_INTERACTIVE_UNLOCK_LOGON` for local accounts or via
`CredPackAuthenticationBuffer` for domain accounts, both through the
`Negotiate` package.

So Windows performs an ordinary interactive logon with a password nobody knows,
which the provider rewrites on every sign-in.

## Why `MSV1_0_S4U_LOGON` cannot replace it

S4U mints a token without credentials, which is genuinely useful for launching
a process as some account (see `BROWSER_PRIVILEGE.md`). It cannot establish
this logon, for three separate reasons:

1. **A credential provider does not perform the logon.** `GetSerialization`
   returns a credential blob and an auth-package id; winlogon hands those to
   LSA and LSA creates the session. `CREDENTIAL_PROVIDER_CREDENTIAL_SERIALIZATION`
   has no field for a token you already made.
2. The token S4U produces lives in the calling process. It is not, and cannot
   become, the interactive desktop session winlogon is about to set up.
3. An S4U logon is identity-only — no credential material in the session. Even
   if it could be injected, the resulting desktop would have no DPAPI master
   key, so Credential Manager, EFS, saved browser and wifi passwords would all
   be broken.

## Why a custom LSA authentication package is closed

This is the mechanism that genuinely does passwordless logon: register an
authentication package, have the provider look that package up instead of
`Negotiate` (one line, in `syscalls.rs::negotiate_package`), pack an arbitrary
blob such as the authentik token, and let the package's `LsaApLogonUser`
validate it and call `CreateLogonSession`.

**It was built and abandoned.** `ee/wcp/ak_lsa` in the C++ tree, deleted in
`57d1bd55` — recoverable with `git show 57d1bd55~1:ee/wcp/ak_lsa/Main.cpp`. It
implemented `SpLsaModeInitialize`, `SpInitialize`, `LsaApLogonUser` and
`CreateLogonSession`; `PrepareToken.cpp` assembled the token by hand
(`LookupAccountNameW`, primary-group SID, `NetUserGetGroups` /
`NetUserGetLocalGroups`) and `PrepareProfile.cpp` built the session profile. It
compiled — `add_subdirectory(ak_lsa)` — but was never registered under
`Authentication Packages` and was never called from the logon path.

**It was abandoned because of Microsoft's signing requirements for LSA
plugins.** Under LSA protection, LSASS runs as a protected process and refuses
to load plugins that are not Microsoft-signed; this is not something an EV
certificate satisfies. That gate is getting tighter, not looser — LSA
protection is default-on for new Windows 11 installs from 22H2 onward.

Treat this as closed. It is not a risk to be managed carefully, it is a
capability a third party cannot ship. Do not restart it without first
establishing that Microsoft will sign the package.

## What is actually left

- **Domain-joined: certificate logon.** authentik issues a short-lived
  certificate and Windows performs PKINIT/smartcard-style logon. No password,
  no code inside LSASS, and a fully supported path — it is broadly the
  mechanism behind Windows Hello for Business and cloud Kerberos. Needs PKI and
  KDC trust. Does nothing for standalone machines.
- **Local accounts: the password reset.** With the AP route closed, this is
  essentially the only option, so treat it as the design rather than as a
  shortcut to be replaced.

## The cost of the current approach, which is worth measuring

An admin-initiated `NetUserSetInfo` level 1003 on a **local** account is the
operation Windows warns "might cause irreversible loss of information" for —
EFS-encrypted files, personal certificates, stored passwords — because the
user's DPAPI master key is protected by the old password. `Connect` does this
on every single sign-in.

This has not been verified on a real machine here. It should be, because if it
does bite, it bites quietly and repeatedly:

1. On the test box, save a credential in Credential Manager as the tile user.
2. Sign out, sign in through the authentik tile.
3. Check whether the saved credential survived.

Repeat for an EFS-encrypted file. If they do not survive, that is a product
issue independent of anything in this document, and the only real mitigations
are the certificate path for domain-joined machines or accepting it for local
accounts.
