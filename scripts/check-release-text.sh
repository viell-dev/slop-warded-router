#!/usr/bin/env bash
# Usage: bash scripts/check-release-text.sh <package version>, from the repository root.
#
# A published README cannot be corrected, so it must pin the version being packaged, and its
# install pins, version-pinned links and the changelog's latest release heading must name only
# that version. The documentation site under docs/ is built from the release tag and describes
# that release, so its pages follow the same rules for pins and pinned links, and one of them
# must show the install pin. crates.io resolves relative README links against the default
# branch, so every README link must be absolute or an in-page anchor.
set -euo pipefail

package_version="${1:?Usage: check-release-text.sh <package version>}"

# Print matching lines, or nothing when none match; other grep failures still stop the check.
matches() {
    grep "$@" || (($? == 1))
}

# Link destinations outside fenced code: inline links and images, and reference definitions,
# including one whose destination is on the next line. Angle brackets and titles are dropped.
# As in CommonMark, a fence is indented at most three spaces, and only a fence of the opening
# character and at least its length closes a block.
readme_links() {
    awk '
        function emit(destination) {
            sub(/^</, "", destination)
            sub(/>$/, "", destination)
            print destination
        }
        {
            sub(/\r$/, "")
            text = $0
            sub(/^ */, "", text)
            indent = length($0) - length(text)
        }
        indent < 4 && match(text, /^(````*|~~~~*)/) {
            marker = substr(text, 1, RLENGTH)
            if (fence == "") {
                fence = marker
            } else if (substr(marker, 1, 1) == substr(fence, 1, 1) &&
                    length(marker) >= length(fence) && substr(text, RLENGTH + 1) ~ /^[ \t]*$/) {
                fence = ""
            }
            next
        }
        fence != "" { next }
        pending { if (NF) { emit($1); pending = 0 }; next }
        /^ *\[[^]]+\]:/ {
            rest = $0
            sub(/^ *\[[^]]+\]:/, "", rest)
            if (rest ~ /^[ \t]*$/) { pending = 1 } else { split(rest, field, " "); emit(field[1]) }
            next
        }
        {
            line = $0
            while (match(line, /\]\([^)]*\)/)) {
                split(substr(line, RSTART + 2, RLENGTH - 3), field, " ")
                emit(field[1])
                line = substr(line, RSTART + RLENGTH)
            }
        }
    ' README.md
}

# The site's pages, without the site tooling's installed packages.
site_pages=()
while IFS= read -r -d '' page; do
    site_pages+=("$page")
done < <(find docs -name node_modules -prune -o -type f -name '*.md' -print0 | sort -z)
if ((${#site_pages[@]} == 0)); then
    echo 'Found no site pages under docs/' >&2
    exit 1
fi
texts=(README.md "${site_pages[@]}")

pin_versions="$(
    matches -hoE "warded@=[^' ]+|^[[:space:]]*warded *= *(\{.*version *= *)?\"=[^\"]+\"" \
        "${texts[@]}" | sed -E 's/.*=//; s/"$//'
)"
link_versions="$(
    matches -hoE 'slop-warded-router/(blob|tree)/[0-9][^/)#>]*|(crates|docs\.rs)/warded/[0-9][^/)#>]*' \
        "${texts[@]}" | sed -E 's#.*/##; s/[.,;:]+$//'
)"
changelog_version="$(matches -m 1 -E '^## [0-9]' CHANGELOG.md | sed -E 's/^## ([^ ]+).*/\1/')"
release_versions="$(
    printf '%s\n' "$pin_versions" "$link_versions" "$changelog_version" | sed '/^$/d' | sort -u
)"
install_pin="warded = \"=$package_version\""
if [[ "$release_versions" != "$package_version" || "$changelog_version" != "$package_version" ]] ||
    ! grep -qF "$install_pin" README.md || ! grep -qF "$install_pin" "${site_pages[@]}"; then
    printf 'README, site pages and CHANGELOG must pin and name only %s; found:\n%s\n' \
        "$package_version" "$release_versions" >&2
    exit 1
fi
links="$(readme_links)"
relative_links="$(matches -vE '^(https?://|#|mailto:)' <<< "$links")"
if [[ -n "$relative_links" ]]; then
    printf 'README links must be absolute for crates.io; found:\n%s\n' "$relative_links" >&2
    exit 1
fi
