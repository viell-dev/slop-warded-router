#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
    echo "Usage: fixture-required.sh <changes result> <affected> <fixture result>" >&2
    exit 2
fi

case "$1:$2:$3" in
    success:true:success|success:false:skipped) exit 0 ;;
    *)
        printf 'Unexpected fixture gate state: changes=%q, affected=%q, fixture=%q\n' \
            "$1" "$2" "$3" >&2
        exit 1
        ;;
esac
