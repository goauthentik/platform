# Offline FIDO2 logon — implementation plan

Lets a user sign in at the Windows logon screen with a hardware security key
(YubiKey and friends) when authentik is unreachable. Spans `authentik`
(backend), `ak-sysd` and `ee/wcp`. Nothing here is implemented yet.

## Problem

`ee/wcp` today is online-only: `Connect()` spawns `ak_cef.exe`, which asks
`ak-sysd` for a sign-in URL, renders authentik's flow, and comes back with a
token that `ak-sysd` validates against the server (`cef-host::sysd`,
`credprovider::sysd`). With no network there is no URL, no token and no
validation — the tile can only fail.

Two separable problems have to be solved to fix that:

1. **Prove the right human is present**, with no server to verify against.
2. **Produce a Windows credential**, with no server to mint one.

(2) is the one that quietly constrains scope — see "Account types" below.

## Prior art in this repo

`ak-pam` already does CTAP2-over-HID with the `ctap-hid-fido2` crate
(`ak-pam/src/auth/fido.rs`, `protobuf/ic_pam_fido.proto`): authentik's
WebAuthn stage issues a challenge, PAM collects a PIN, gets an assertion, and
the *server* verifies it. That gives us a proven Rust CTAP stack and a
message shape to copy, but it is an **online** design — the trust root is the
server. Offline has no such root, which drives the crypto choice below.

## Crypto design

### Rejected: cache the public key and verify signatures offline

Export the user's registered WebAuthn credential IDs and public keys to the
device; offline, generate a challenge locally, get an assertion, verify it
against the cached key.

This does not survive its own threat model. The offline verifier has no root
of trust: anyone who can write the cache can swap in their own public key and
forge assertions. Since the attacker in the offline scenario is by definition
someone with physical possession of the machine, that is the wrong shape.

### Chosen: `hmac-secret`, so the secret cannot be produced without the key

CTAP2's `hmac-secret` extension makes the authenticator derive a
deterministic 32-byte secret from (credential, salt), released **only after
user verification**. Use it as a key-encryption key. Offline logon then isn't
"check a signature" (forgeable if the cache is tampered with) but "decrypt a
blob", which is impossible without the physical key *and* the PIN. Same
primitive `systemd-cryptenroll --fido2-device` relies on.

Per (Windows account, credential) we store an `offline_record`:

| field | purpose |
| --- | --- |
| `credential_id`, `rp_id` | what to ask the authenticator for |
| `public_key` (COSE) | secondary signature check, defence in depth |
| `hmac_salt` | 32 random bytes, input to `hmac-secret` |
| `sign_count` | last value seen; clone/rollback detection |
| `wrapped_payload` | AEAD over the payload, key = HKDF(hmac-secret) |
| `expires_at`, `logons_remaining` | policy envelope |
| `aaguid`, `authentik_user_id` | allow-listing and audit |

The whole record is additionally sealed to the machine (DPAPI machine scope,
or `ak-platform-keyring`'s Windows store) so it cannot be lifted to another
box and attacked there.

**Payload = a random 32-byte `offline_root`, generated once at enrolment.**
Not the live Windows password. Sealing the password would force enrolment to
coincide with a password rotation, which means the key has to be present at a
*logon* — and that is the one moment we cannot rely on it being plugged in.
Decoupling them lets enrolment happen once, anywhere, at a time of the user's
choosing.

Offline then: unseal `offline_root` (which requires key + PIN), derive a
fresh password from it, `NetUserSetInfo` it onto the local account, and hand
that to the existing `pack_kerb_interactive_unlock_logon` path. The password
reset works offline for local accounts because `ak-sysd` runs as SYSTEM and no
DC is involved — the same call the online flow already makes today.

Worth verifying early: `NetUserSetInfo` level 1003 is an *administrative
reset*, not a user-initiated change, and Windows cannot re-protect a local
profile's DPAPI master key across one. Saved Credential Manager entries, Wi-Fi
secrets and EFS certificates may be lost each time. The current online flow
already does this on every logon, so this is a pre-existing property rather
than something offline introduces — but if it turns out to bite users, it
affects both paths and should be settled before offline multiplies it.

### Account types

- **Local accounts** — fully supported by the above.
- **Domain accounts** — out of scope for the first cut, and this needs saying
  plainly: we never see the domain password (the user authenticates to
  authentik in a browser), and a domain password cannot be reset offline. The
  only way in is to capture and seal the real password during an online
  logon, which the current browser-based flow structurally cannot do. Anyone
  planning domain coverage should treat it as a separate design, not a
  follow-up ticket.

## Enrolment

**The key must be physically present to enrol, and there is no way around
it.** `hmac-secret` only works against a credential created on that
authenticator, so enrolment is a real `authenticatorMakeCredential` ceremony:
key inserted, PIN entered, touch. Nothing the backend holds can substitute —
that is the cost of not trusting a cached public key, and it is the right
trade.

So "automatic" means *automatically prompted, and retried until it takes* —
not silent. Concretely:

- **Primary trigger: in the user's desktop session, via the agent.** When
  policy enables offline and no active record exists, the agent prompts
  ("Set up offline sign-in — insert your security key"). This is the right
  place: the key can be plugged in comfortably, the user can decline and be
  asked again, there is a real window for messaging and errors, and none of
  the secure-desktop constraints apply.
- **Secondary, opportunistic: at the logon screen.** After a successful online
  logon, if an authenticator is *already* present, offer enrolment inline.
  Cheap to add once the tile is a state machine, and catches users who keep a
  key permanently plugged in.
- **Never block the online logon.** A user who declines, or has no key on
  them, signs in exactly as before.
- **Surface "not enrolled" before it matters.** The failure mode to avoid is a
  user discovering on a plane that they never enrolled. `OfflineStatus`
  already reports this; the agent should nag on it, and the tile should say so
  while still online — not the first time it is offline and stuck.

Do *not* try to guarantee presence by forcing the online logon through
authentik's WebAuthn stage. Beyond being user-hostile, it likely does not even
work: that stage runs in the CEF window, Chromium's WebAuthn goes through
`webauthn.dll`, and that wants a foreground window in an interactive session —
which LogonUI's secure desktop is not. Whether browser WebAuthn functions at
all inside `ak_cef.exe` is an open question in its own right, and this design
deliberately does not depend on the answer.

## Backend (authentik)

- **`OfflineCredential` model** — its own model, *not* an extension of
  `WebAuthnDevice`. Browser WebAuthn credentials and device-bound offline
  credentials have different lifecycles, different RP IDs and different
  revocation semantics; conflating them makes both harder to reason about.
  Fields: user, endpoint/device, `credential_id`, `public_key`, `aaguid`,
  `rp_id`, `created`, `last_used_offline`, `revoked`.
- **Enrolment API** — `POST /api/v3/platform/offline_credentials/`, called by
  `ak-sysd` while online and authenticated as the enrolled device plus a live
  user session. Returns the RP ID to use, so the device never invents it.
- **Policy**, delivered with the existing per-domain `AgentConfig` that
  `ak-sysd` already caches (`ak-sysd/src/cfg/domain.rs`):
  `offline_enabled`, `max_offline_days`, `max_offline_logons`, `require_uv`
  (default true), `allowed_aaguids` (optional allow-list, e.g. YubiKey only),
  `min_pin_length`.
- **Revocation** — admin action marks a credential revoked; the revocation
  list rides along with the domain config, and `ak-sysd` wipes matching local
  records on the next successful contact. Revocation is therefore only
  effective once the machine comes back online: that is inherent to offline
  auth and should be stated in the admin UI, not glossed over.
- **Audit** — offline logons are queued locally and POSTed on reconnect,
  landing as login events flagged `offline` with the original timestamp.
  Compliance stories usually hinge on this, so it is not optional.
- **Enrolment gating** — reuse a flow with the existing WebAuthn stage so
  admins control who may enrol at all.

## `ak-sysd`

- **New proto** `protobuf/sys_auth_offline.proto`, service
  `SystemAuthOffline`:
  - `OfflineStatus` → enrolled?, expiry, logons remaining, reason unavailable.
  - `BeginOfflineAuth(username)` → `rp_id`, `credential_ids`, `hmac_salt`,
    `uv_required`, sealed payload.
  - `CompleteOfflineAuth(assertion, sign_count)` → outcome + username.
  - `EnrollOffline(...)` → online-only, drives the backend API.
- **New component** `ak-sysd/src/components/auth/offline.rs`, alongside the
  existing `token.rs` / `interactive` / `apple.rs`.
- **New state tables** in `ak-sysd/src/state/mod.rs` (which already carries
  `sessions`, `domain_cache`, `component_state`): `offline_credentials`,
  `offline_logon_queue`.
- **Reachability** — an explicit online/offline determination with a short
  probe timeout, not "the last request happened to fail". Offline must be a
  decision, not an accident, or the tile will fall back to the weaker path on
  a flaky link.
- **Move local-password rotation into `ak-sysd`.** Today `ee/wcp` generates
  the password and calls `NetUserSetInfo` itself. For offline to work the same
  secret must be sealed, and having two components generate and hold it is how
  they drift. `ak-sysd` should own generation, rotation and sealing, handing
  `ee/wcp` the password only for the serialization buffer. This is a real
  refactor of `credprovider::syscalls::LocalAccountPasswordReset` and should
  be sequenced first.

## `ee/wcp`

- **Tile becomes a state machine.** `wire::TILE_FIELDS` is a fixed 4-entry
  table today (image, hidden label, large text, submit) and
  `tile::field_state_at` is static. Offline needs a PIN field
  (`CPFT_PASSWORD_TEXT`), live status text, and a way to choose the offline
  path. Add a `TileMode` to `wire` and make field state mode-dependent.
- **`Advise`/`UnAdvise` must start doing something.** They are no-ops in
  `credprovider::credential`; driving "insert your key" → "enter PIN" →
  "touch your key" requires `ICredentialProviderCredentialEvents`
  (`SetFieldState`, `SetFieldString`).
- **`Connect()` branches.** Online keeps the existing `ak_cef.exe` flow;
  offline does CTAP2 and never spawns a browser at all — which incidentally
  sidesteps the secure-desktop browser question entirely.
- **CTAP2 lives in the DLL, not `ak-sysd`.** The PIN is collected by the tile,
  and it should not cross a process boundary; so the component that collects
  it is the one that talks to the authenticator, mirroring `ak-pam`. `ak-sysd`
  supplies the challenge and enforces policy; `ee/wcp` performs the exchange
  and unseals. Set `cfg.keep_alive_msg = String::new()` as `ak-pam` does —
  the crate otherwise writes to stdout, which is exactly the kind of thing
  that produced the console-window bug.
- **Cancellation** — reuse `IQueryContinueWithStatus` polling for the
  touch-timeout wait, as the current `wait_for_result` does.

## Biggest technical unknown

**Does `ctap-hid-fido2` work under LogonUI, on the secure desktop, as
SYSTEM?** HID access should be fine for SYSTEM, but the crate has not been
exercised there, and the answer determines whether the whole `ee/wcp` half of
this design stands. Validate it with a throwaway spike *before* any of the
backend or `ak-sysd` work — a day spent here de-risks weeks. If it fails, the
fallback is to have `ak-sysd` own the HID exchange and accept the PIN crossing
the pipe, which is a materially worse design and should be a deliberate
decision rather than a discovery halfway through.

## Anti-replay and abuse

- Enforce monotonic `sign_count`; refuse on rollback (clone detection). Some
  authenticators always return 0 — record that at enrolment and skip the check
  for those, rather than silently accepting any value for everyone.
- Consecutive-failure counter that disables the offline path until a
  successful online logon.
- Refuse once `expires_at` or `logons_remaining` is exhausted.
- Reject a system clock that has moved backwards materially since the last
  logon; an offline machine's clock is attacker-controllable.

## Testing

The existing harness (`ee/wcp/e2e`, `AK_WCP_E2E`) generalises, but a CI runner
has no YubiKey, so:

- **Put CTAP behind a trait**, exactly like `AuthPackageLookup` and
  `LocalAccountPasswordReset`, and provide a software authenticator with a
  known key for tests. Without this seam none of the below is automatable.
- Unit: `hmac-secret` → KEK derivation, seal/unseal round-trip, sign-count
  rollback rejection, expiry and logon-count exhaustion, policy parsing.
- e2e: offline logon end-to-end against the software authenticator and a mock
  `ak-sysd` reporting "offline"; enrolment while "online"; revocation wiping
  the record.
- Manual checklist (extend `ee/wcp/e2e/README.md`): real YubiKey at a real
  logon screen, unlock *and* fresh logon, wrong PIN, PIN lockout, key removed
  mid-flow, key never inserted.

## Sequencing

0. Spike: `ctap-hid-fido2` under LogonUI as SYSTEM. Gate on the result.
1. Move local-password rotation into `ak-sysd`; add the seal/unseal helper.
   No user-visible change.
2. Backend model, API, policy and revocation plumbing.
3. Enrolment in-session via the agent, plus `OfflineStatus` reporting and the
   "not enrolled" nag. Writes the record; still no offline logon, so this
   ships safely on its own.
4. Tile state machine and the offline `Connect()` path.
5. Opportunistic enrolment at the logon screen (needs step 4's state machine).
6. Audit queue and reconnect sync.
7. Optional, separate design: domain accounts.

## Open questions

- `hmac-secret` is not universal. Probe at enrolment and fail with a clear
  message rather than silently enrolling a key that cannot be used offline.
- Keys with no PIN set cannot do UV. With `require_uv` on (the default) these
  must be refused at enrolment.
- Multiple credentials per user (a backup key) — the record should hold a set,
  not one. This matters more than it looks: a single enrolled key is a single
  point of failure that only manifests when the user is already offline and
  cannot self-serve a recovery. Prompt for a second key once the first
  succeeds.
- Platform authenticators / Windows Hello are deliberately out of scope;
  Windows already ships a credential provider for those.
- Does browser WebAuthn work at all inside `ak_cef.exe` on the secure desktop?
  Nothing in this design depends on it, but it determines whether security
  keys are usable for *online* logon too, which users will reasonably expect
  once they have enrolled one for offline.
