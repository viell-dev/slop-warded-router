#!/usr/bin/env bash
set -euo pipefail

usage() {
    cat >&2 <<'USAGE'
Usage: bash scripts/check-clean-consumer.sh <pushed commit SHA> [previous release SHA]
   or: bash scripts/check-clean-consumer.sh --registry <version> [previous commit SHA or version]
USAGE
    exit 2
}

commit_pattern='^[0-9a-f]{40}$'
version_pattern='^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$'
mode=git
baseline_default=59710b07569ac065cf97f2b3c7f4a84389489c44
if [[ "${1:-}" == --registry ]]; then
    mode=registry
    baseline_default=2804571157bafda67a832439d1442586fdf77d2a
    shift
fi
candidate="${1:-}"
baseline="${2-$baseline_default}"
[[ $# -le 2 ]] || usage
if [[ "$mode" == registry ]]; then
    [[ "$candidate" =~ $version_pattern && "$baseline" != "$candidate" ]] || usage
    [[ "$baseline" =~ $commit_pattern || "$baseline" =~ $version_pattern ]] || usage
else
    [[ "$candidate" =~ $commit_pattern && "$baseline" =~ $commit_pattern ]] || usage
fi

repository=https://github.com/viell-dev/slop-warded-router
source_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Read the matching dependency block, so the script exercises the installation snippets.
read_dependencies() {
    awk -v pattern="$1" '
        { sub(/\r$/, "") }
        /^```$/ { if (found) { printf "%s", block; exit }; snippet = 0 }
        snippet { block = block $0 "\n"; if ($0 ~ pattern) found = 1 }
        /^```toml$/ { snippet = 1; block = "" }
        END { if (!found) exit 1 }
    ' "$source_dir/README.md"
}

consumer="$(mktemp -d "${TMPDIR:-/tmp}/warded-consumer.XXXXXXXX")"
trap 'rm -rf -- "$consumer"' EXIT
cd "$consumer"
export CARGO_TARGET_DIR="$consumer/target"
mkdir src

write_manifest() {
    cat > Cargo.toml <<'TOML'
[package]
name = "warded-source-consumer"
version = "0.0.0"
edition = "2024"
publish = false
TOML
    if [[ $# == 0 ]]; then
        return
    fi
    cat >> Cargo.toml <<'TOML'

[features]
default = ["warded/default", "dep:axum", "dep:tokio", "dep:tower"]
tower = ["warded/tower", "dep:axum", "dep:tokio", "dep:tower"]
negotiation = ["warded/negotiation"]

TOML
    if [[ "$1" =~ $commit_pattern ]]; then
        read_dependencies '^warded = [{] git = ' |
            sed "s/rev = \"<full commit SHA>\"/rev = \"$1\", default-features = false/" >> Cargo.toml
    else
        read_dependencies '^warded = "=' |
            sed "s/^warded = \"=[^\"]*\"$/warded = { version = \"=$1\", default-features = false }/" \
                >> Cargo.toml
    fi
    cat >> Cargo.toml <<'TOML'
axum = { version = "=0.8.9", default-features = false, features = ["http1", "tokio"], optional = true }
tokio = { version = "=1.54.1", features = ["macros", "rt"], optional = true }
tower = { version = "=0.5.3", features = ["util"], optional = true }
TOML
}

assert_locked_source() {
    local candidate=$1
    local source version=''
    if [[ "$candidate" =~ $commit_pattern ]]; then
        source="source = \"git+$repository?rev=$candidate#$candidate\""
    else
        source='source = "registry+https://github.com/rust-lang/crates.io-index"'
        version="version = \"$candidate\""
    fi
    if ! awk -v source="$source" -v version="$version" '
        BEGIN { RS = ""; FS = "\n" }
        {
            warded = same_source = same_version = checksum = 0
            for (i = 1; i <= NF; i++) {
                if ($i == "name = \"warded\"") warded = 1
                if ($i == source) same_source = 1
                if ($i == version) same_version = 1
                if ($i ~ /^checksum = "[0-9a-f]+"$/) {
                    split($i, fields, "\"")
                    checksum = length(fields[2]) == 64
                }
            }
            if (warded) {
                packages++
                if (same_source && (version == "" || (same_version && checksum))) matches++
            }
        }
        END { exit !(packages == 1 && matches == 1) }
    ' Cargo.lock; then
        echo "Expected exactly one Warded package locked to $candidate ($source)" >&2
        exit 1
    fi
    printf 'Locked Warded to %s (%s)\n' "$candidate" "$source"
}

cat > src/main.rs <<'RUST'
use http::Request;
use warded::{Head, Matcher, Selector, matchers::path};

fn selector_example() -> Result<(), Box<dyn std::error::Error>> {
    let routes = Selector::new()
        .route(
            path("/users/{id}")?.filter(|captures| {
                captures.get("id").is_some_and(|raw| raw.parse::<u64>().is_ok())
            }),
            "id",
        )
        .route(path("/users/{name}")?, "name");

    for (uri, target, name, value, rejected) in [
        ("/users/42", "id", "id", "42", "name"),
        ("/users/alice", "name", "name", "alice", "id"),
    ] {
        let request = Request::builder().uri(uri).body(())?;
        let selected = routes.select(&Head::from_request(&request))?.unwrap();
        assert_eq!(*selected.target, target);
        assert_eq!(selected.selection.captures.get(name), Some(value));
        assert_eq!(selected.selection.captures.get(rejected), None);
    }
    let miss = Request::builder().uri("/elsewhere").body(())?;
    assert!(routes.select(&Head::from_request(&miss))?.is_none());
    Ok(())
}

#[cfg(any(feature = "default", feature = "negotiation"))]
fn negotiation_example() -> Result<(), Box<dyn std::error::Error>> {
    use warded::negotiation::Offers;

    let formats = Offers::media_types(["text/html", "application/json"])?;
    let routes = Selector::new()
        .route(formats.selected("text/html")?, "html")
        .route(formats.selected("application/json")?, "json");
    let request = Request::builder()
        .header("accept", "application/json, text/html;q=0.5")
        .body(())?;
    let selected = routes.select(&Head::from_request(&request))?.unwrap();
    assert_eq!(*selected.target, "json");
    Ok(())
}

#[cfg(any(feature = "default", feature = "tower"))]
#[tokio::main(flavor = "current_thread")]
async fn tower_example() -> Result<(), Box<dyn std::error::Error>> {
    use axum::{
        body::{Body, to_bytes},
        error_handling::HandleError,
        extract::Path,
        routing::get,
    };
    use http::StatusCode;
    use std::convert::Infallible;
    use tower::ServiceExt;
    use warded::{Captures, DispatchError, Router};

    let users = axum::Router::new().route(
        "/users/{name}",
        get(
            |Path(name): Path<String>, axum::Extension(captures): axum::Extension<Captures>| async move {
                assert_eq!(captures.get("name"), Some(name.as_str()));
                format!("user {name}")
            },
        ),
    );
    let routes = Router::new().route(path("/users/{name}")?, users);
    let service = HandleError::new(routes, |_: DispatchError<Infallible>| async {
        StatusCode::INTERNAL_SERVER_ERROR
    });
    let host = axum::Router::new().fallback_service(service);
    let request = Request::builder().uri("/users/alice").body(Body::empty())?;
    let response = host.clone().oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(&to_bytes(response.into_body(), 128).await?[..], b"user alice");
    let miss = Request::builder().uri("/elsewhere").body(Body::empty())?;
    assert_eq!(host.oneshot(miss).await?.status(), StatusCode::NOT_FOUND);
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    selector_example()?;
    #[cfg(any(feature = "default", feature = "negotiation"))]
    negotiation_example()?;
    #[cfg(any(feature = "default", feature = "tower"))]
    tower_example()?;
    println!("Consumer assertions passed");
    Ok(())
}
RUST

rustc --version --verbose
cargo --version

if [[ "$mode" == registry ]]; then
    echo "Installing the exact registry version $candidate with a fresh lockfile"
    write_manifest "$candidate"
    cargo generate-lockfile
    assert_locked_source "$candidate"
    cargo run --locked
    rm -- Cargo.lock
fi

echo "Installing the update baseline $baseline"
write_manifest "$baseline"
cargo generate-lockfile
assert_locked_source "$baseline"
cargo run --locked

echo "Updating the existing consumer lockfile to $candidate"
write_manifest "$candidate"
cargo update --workspace
assert_locked_source "$candidate"

echo "Running with Warded's default features"
cargo run --locked
echo "Running standalone matching with no default features"
cargo run --locked --no-default-features
echo "Running with Tower alone"
cargo run --locked --no-default-features --features tower
echo "Running with negotiation alone"
cargo run --locked --no-default-features --features negotiation

echo "Building the dependency's API reference"
RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps --package warded
api_index="$CARGO_TARGET_DIR/doc/warded/index.html"
test -s "$api_index"

echo "Removing Warded and refreshing the consumer lockfile"
mv src/main.rs main-with-warded.rs
echo 'fn main() {}' > src/main.rs
if [[ "$mode" == registry ]]; then
    cargo remove warded
else
    write_manifest
    cargo update --workspace
fi
if grep -Fxq 'name = "warded"' Cargo.lock || grep -Fq "$repository" Cargo.lock; then
    echo "Warded remains in the lockfile after removal" >&2
    exit 1
fi
cargo run --locked

echo "Recovering with the pinned candidate"
mv main-with-warded.rs src/main.rs
write_manifest "$candidate"
cargo update --workspace
assert_locked_source "$candidate"
cargo run --locked
echo "Clean consumer verification passed for $candidate"
