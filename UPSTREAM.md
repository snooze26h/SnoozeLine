# Upstream provenance

SnoozeLine is an independently maintained derivative of [CCometixLine](https://github.com/Haleclipse/CCometixLine).

| Field | Value |
|---|---|
| Upstream project | CCometixLine |
| Upstream author | Haleclipse and contributors |
| Upstream release | `v1.1.2` |
| Upstream commit | `a73b166557662b2e79b83ea005fb297003748fb0` |
| Import date | `2026-09-01` |
| Downstream project | SnoozeLine `0.1.0` |
| Downstream maintainer | snooze26h |
| Downstream remote | [snooze26h/SnoozeLine](https://github.com/snooze26h/SnoozeLine) (public source repository) |

The upstream Git history through the commit above is preserved. SnoozeLine changes begin after that commit. SnoozeLine is not affiliated with, sponsored by, or endorsed by Haleclipse or the upstream project.

## Downstream changes

SnoozeLine introduces its own name and runtime directory, the built-in `snooze26h` theme, corrected context and quota semantics, safer rendering and collection behavior, and source-build and local-installation documentation. Exact changes are recorded in downstream commits and [CHANGELOG.md](CHANGELOG.md).

The original `upstream` remote is retained for provenance. Upstream changes must be reviewed and integrated deliberately; they are never treated as SnoozeLine-authored work.

## Selected v2.0.1 adaptations in SnoozeLine 0.2.0

- Reviewed release: [CCometixLine v2.0.1](https://github.com/Haleclipse/CCometixLine/releases/tag/v2.0.1).
- Exact commit: `56849931745e05d2d4e506af2bedbc8d1f0616d4`.
- Review date: `2026-11-06`.
- Git collection and caching adapt the design in upstream `src/git.rs`, `src/state/git_cache.rs` and `src/state/swr.rs`, retaining SnoozeLine's output bounds and NUL-delimited filename handling.
- Optional-field parsing and fractional quota timestamps adapt `src/input.rs`; expired quota handling follows `src/metrics/usage.rs`.
- The read-only preview uses SnoozeLine's existing renderer and sample generator, following upstream's preview workflow.
- These changes are adapted from Haleclipse and contributors. They do not replace the preserved v1.1.2 base or imply a merge of the complete v2 history.
- Existing configuration, themes, model/effort behavior and account-launcher paths remain supported. The v2 layout/TUI rewrite and companion mod are not imported.

## License evidence

At `v1.1.2`, the following upstream materials declare the SPDX identifier `MIT`:

- `Cargo.toml`
- npm metadata for `@cometix/ccline@1.1.2`
- `README.md` and `README.zh.md`

However, the upstream `v1.1.2` tree and its preserved history do not contain the `LICENSE` file referenced by those README files. GitHub therefore does not detect a repository license.

SnoozeLine records those facts without inventing a license text or copyright holder. The source repository is public and has no GitHub release; the exact upstream notice remains unresolved. This document is provenance evidence, not a replacement license or legal advice.
