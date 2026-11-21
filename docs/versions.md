# Versions

Warded uses [Semantic Versioning](https://semver.org/) with prerelease stages. The version this
site describes is shown in the navigation bar.

## Release stages

The first release line is `0.1.0`. Its prereleases pass through three stages:

- **Alpha:** the scope and core behavior were still changing.
- **Beta:** the agreed features of the first release are in place, and work concentrates on
  fixes, documentation and polish. Warded is in beta. A beta can still change public names or
  behavior when that fixes a real problem; [upgrading](./upgrading.md) lists every such change.
- **Release candidate:** a build intended to become the stable release unchanged.

## Pin the exact version

Install with an exact requirement, the version prefixed with `=`, as [getting
started](./getting-started.md#install) does. A plain requirement without `=` also allows Cargo to
move to a later prerelease of 0.1.0 when you run `cargo update`, which can include the changes
betas are allowed to make. With the exact pin you choose when to upgrade.

The [crates.io version list](https://crates.io/crates/warded/versions) shows every published
version, and this command describes the latest one:

```sh
cargo info warded
```

## What a stable 0.1 release keeps

From the first stable `0.1.0`, every `0.1.x` release keeps:

- the public API;
- the documented routing, capture, negotiation and dispatch behavior;
- the feature names `tower` and `negotiation`;
- the minimum supported Rust version, 1.99.1.

An intentional breaking change waits for 0.2.0. Fixing a defect can still change behavior that
contradicted the documentation.

## Documentation for each version

- **This site** describes the latest installable release. Its `version.json` file, at the root of
  the site, names the version and the commit the site was built from.
- **The API reference** on [docs.rs](https://docs.rs/warded) exists for every published version;
  use its version menu to pick another one.
- **The README** on [crates.io](https://crates.io/crates/warded) matches each published version.
- **Unreleased changes** on the default branch are listed under _Unreleased_ in the
  [changelog](./changelog.md). To try them, depend on a commit as [getting
  started](./getting-started.md#try-the-repository-examples) shows.

## Security fixes

Security fixes land on the default branch and ship in a new release of the current line. See the
[security policy](https://github.com/viell-dev/slop-warded-router/security/policy) for how to
report a vulnerability.
