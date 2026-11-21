# Maintainer notes

These notes hold what the consumer documentation leaves out: design rationale, evidence,
procedures and measurements. They are for the people and agents who maintain Warded. The
documentation site under [`docs/`](../docs/) does not build them, and the crate package does not
ship them. A site page links a note where a reader may want the detail; a note links the site page
that owns a public fact rather than repeating it.

- [Design and first-release boundaries](./design.md): the selection model, dependency choices and
  the semantic differences from the libraries Warded reuses.
- [Release procedure](./release.md): stages, gates, publication, the documentation site and
  recovery.
- [Verification](./verification.md): what each check proves, how to run it and its evidence limits.
- [Upstream test comparison](./upstream-tests.md): the recurring review of other routers' test
  suites and the cases adopted from it.
- [First-release assessment](./beta-assessment.md): API freeze decisions, consumer evidence and the
  selection-cost measurements.

The documentation has three areas:

| Area                   | Audience          | Source                         | Published at                                   |
| ---------------------- | ----------------- | ------------------------------ | ---------------------------------------------- |
| Guides and explanation | Consumers         | [`docs/`](../docs/)            | <https://viell-dev.github.io/slop-warded-router/> |
| API reference          | Consumers         | Rustdoc in [`src/`](../src/)   | <https://docs.rs/warded>                       |
| Maintainer notes       | Maintainers       | This directory                 | The repository only                            |

Keep a note accurate in the pull request that changes what it describes, as for the consumer
documentation. Start with [CONTRIBUTING.md](../CONTRIBUTING.md) for contribution, checks and pull
requests.
