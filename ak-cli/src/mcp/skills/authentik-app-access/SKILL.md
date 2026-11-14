---
name: authentik-app-access
description: Call any application protected by authentik on the user's behalf, through a delegated agent identity with user-approved, per-application access.
---

# Accessing applications through authentik

Applications behind authentik are never called with the user's own credentials. Instead, an agent
identity is created for the user, the user approves which applications it may use, and each request
carries a short-lived token scoped to one application.

## Workflow

1. **Find the application.** `list_applications`, optionally with `search`, returns the applications
   available to the user. Note each one's `pbm_uuid` and `launch_url`.
2. **Create an agent identity.** `create_agent` with a `label` describing the task. Keep the
   identifier it returns and reuse it for the rest of the session instead of creating new ones.
   Agent identities expire; create a new one if a tool reports it as unknown or expired.
3. **Request access.** `request_access` with the agent identifier and the `pbm_uuid`s of every
   application needed. It returns a link: show it to the user and wait until they confirm they
   approved it. Do not continue before that, and do not open the link yourself.
4. **Exchange a token.** `token_exchange` with the agent identifier and the application's `pbm_uuid`
   as `target_id`, plus `scopes` if the application needs specific ones. The result lists the origins
   the token may be used against. Exchange again when it expires.
5. **Call the application.** `http_fetch` for `GET`/`HEAD`, `http_send` for
   `POST`/`PUT`/`PATCH`/`DELETE`, both with the agent identifier and an absolute `url`. The token is
   attached automatically; never add an `Authorization` header yourself.

## Rules

- Only hosts sharing a registrable domain with an application the agent exchanged a token for are
  reachable. A refusal means the URL is outside that set: check `launch_url`, don't try other hosts.
- Redirects are not followed; a 3xx response is returned as-is. Follow it yourself if its
  `Location` is on an allowed origin.
- `http_send` defaults the body's content type to `application/json`; set `content_type` for
  anything else.
- Only set `insecure` after a request failed because of the certificate, and tell the user you did.
- `401`/`403` from the application after a successful exchange usually means the token expired or
  lacks a scope: exchange again, with `scopes` if needed, before asking the user.
- Requests through `http_send` change data in the application. Confirm with the user before sending
  them unless they asked for that exact change.
- To manage authentik itself rather than an application behind it, use the `authentik-api` skill.
