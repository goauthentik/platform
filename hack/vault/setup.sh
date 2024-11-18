#!/bin/bash -xe
URL="http://localhost:9000/application/o/vault/"
CLIENT_ID="MksCc4QYrJJLjOJui2yNsZa3JLN0p20AJFezsn3z"

vault audit enable file file_path=stdout
vault auth enable oidc

vault write auth/oidc/config \
    oidc_discovery_url="${URL}" \
    oidc_client_id="" \
    oidc_client_secret="" \
    default_role="reader"

vault write auth/oidc/role/reader \
    bound_audiences="${CLIENT_ID}" \
    user_claim="sub" \
    policies="reader" \
    role_type="jwt"
