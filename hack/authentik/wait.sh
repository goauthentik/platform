#!/bin/bash
set -xeuo pipefail

function checkConnector {
    output=$(curl "${AK_URL}/api/v3/endpoints/connectors/?search=agent" \
        --header "Authorization: Bearer ${AK_TOKEN}" --silent)
    count=$(echo $output | jq .pagination.count)
    if [[ "$count" == "1" ]]; then
        exit 0
    fi
    return 0
}

echo "Waiting for authentik to be up"
while checkConnector; do
    sleep 5
done
