---
name: authentik-api
description: Query and change an authentik instance through its REST API, as a delegated agent identity.
---

# authentik API

The authentik API is reached through three tools instead of one tool per endpoint:

1. `api_search` with a few words (`"applications list"`, `"flow bindings"`) to find the command.
   Results are capped at 50; refine the query if it says there are more.
2. `api_describe` with the command path from the search (`"core applications list"`) to see its
   positional arguments and flags.
3. `api_call` with the command path, its arguments, and the agent to act as:
   `{"command": "core applications list", "args": {"search": "grafana", "page-size": 20}, "agent_identifier": "..."}`.

## Arguments

- Keys are the flag names from `api_describe` without the leading `--`.
- Positional arguments use their name from the usage line in lower case: `{"slug": "my-app"}`.
- List-valued arguments take a JSON array.
- Request bodies of create/update commands are flags named after the body's fields; look them up
  with `api_describe` instead of guessing.
- List commands are paginated. Always pass `page-size` and narrow with `search` or field filters
  rather than reading every page.

## Results

Results are JSON. Commands without a response body return `null`. An error result contains the
API's message; fix the arguments and retry instead of switching tools.

## Agent identities

`api_call` always acts as an agent identity, never as the user:

1. `create_agent` and keep the identifier it returns.
2. Pass the identifier as `agent_identifier` to every `api_call`.

A permission error means the agent identity is not allowed to do this; tell the user rather than
retrying.

`api_call_dangerous` acts as the user with all of their permissions. It is only available when the
server was started with `--allow-dangerous-api`; use it only when the user explicitly asks for it
or an agent identity cannot do what is needed.

Calls that modify data (`create`, `update`, `partial-update`, `destroy`) change the live instance.
Confirm with the user before running them unless they asked for the change.
