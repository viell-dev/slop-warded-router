#!/usr/bin/env bash
set -euo pipefail

mode=$1
version=$2
local_checksum=$3
[[ "$local_checksum" =~ ^[0-9a-f]{64}$ ]] || { echo "Invalid package checksum" >&2; exit 1; }

index_body="$(mktemp)"
trap 'rm -f -- "$index_body"' EXIT

index_checksum() {
    local status
    status="$(curl --silent --show-error --connect-timeout 10 --max-time 30 --retry 2 \
        --user-agent warded-release --header 'Cache-Control: no-cache' \
        --output "$index_body" --write-out '%{http_code}' https://index.crates.io/wa/rd/warded)" ||
        return 1
    case "$status" in
        404) echo missing ;;
        200)
            jq -ers --arg version "$version" '
                if length == 0 or any(.[];
                        if type != "object" then true
                        else .name != "warded" or (.vers | type) != "string" or
                             (.cksum | type) != "string" or
                             (.cksum | test("^[0-9a-f]{64}$") | not)
                        end) then
                    error("invalid Warded index response")
                else
                    map(select(.vers == $version)) |
                    if length == 0 then "missing"
                    elif length == 1 then .[0].cksum
                    else error("duplicate Warded version in the index")
                    end
                end
            ' "$index_body"
            ;;
        *) echo "crates.io sparse index returned HTTP $status" >&2; return 1 ;;
    esac
}

case "$mode" in
    check) attempts=1 ;;
    confirm) attempts=18 ;;
    *) echo "Expected check or confirm" >&2; exit 2 ;;
esac

for ((attempt = 1; attempt <= attempts; attempt++)); do
    # The check before uploading fails closed. After an upload, a read failure only means "not
    # yet": the loop retries, and a different checksum still fails.
    if ! published_checksum="$(index_checksum)"; then
        echo "Could not read the crates.io index" >&2
        [[ "$mode" == confirm ]] || exit 1
        published_checksum=missing
    fi
    if [[ "$published_checksum" != missing ]]; then
        if [[ "$published_checksum" != "$local_checksum" ]]; then
            echo "Warded $version already has a different checksum: $published_checksum" >&2
            exit 1
        fi
        echo "Verified Warded $version at crates.io with SHA-256 $published_checksum"
        if [[ "$mode" == check ]]; then
            echo 'published=true' >> "$GITHUB_OUTPUT"
        fi
        exit 0
    fi
    if [[ "$mode" == check ]]; then
        echo "Warded $version is absent from the index"
        echo 'published=false' >> "$GITHUB_OUTPUT"
        exit 0
    fi
    if ((attempt < attempts)); then
        sleep 5
    fi
done

echo "Warded $version did not appear in a readable index after publishing" >&2
exit 1
