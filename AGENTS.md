# AGENTS.md

## Project identity

SnoozeLine `0.1.0` is a private, local Claude Code status-line project derived from CCometixLine `v1.1.2` at commit `a73b166557662b2e79b83ea005fb297003748fb0`.

- Product, crate, library, binary, and CLI name: `SnoozeLine` / `snoozeline`
- Default built-in theme: `snooze26h`
- Runtime root: `~/.claude/snoozeline`
- Optional override: absolute-path `SNOOZELINE_HOME`

Do not reintroduce the `snoozer.*` version suffix or present upstream work as original SnoozeLine work.

## Safety boundaries

- Do not install or migrate SnoozeLine, edit `~/.claude/settings.json`, or replace `~/.claude/ccline/ccline` without explicit user approval.
- Do not move or delete anything under `~/.claude/ccline`; migration must remain side by side and reversible.
- Do not create an `origin`, GitHub release, npm package, or public distribution without explicit user approval.
- Keep the `upstream` remote and inherited Git authorship intact.
- Do not add or invent a `LICENSE`, copyright holder, or definitive licensing claim while the upstream notice remains missing.
- Keep the inherited npm packaging out of release claims until it is deliberately renamed, audited, and authorized.

## Behavioral invariants

- Prefer native Claude Code context and rate-limit data.
- Count current input plus cache input for context usage; exclude output tokens.
- Clamp percentages to `0–100%` and never fabricate missing quota values.
- Treat `5h` and `7d` as used percentages in the visible line.
- Keep the default line compact; do not add a “shared” label or reset timestamp.
- Sanitize untrusted display text and keep Git/network work bounded.
- Do not persist transcript content in SnoozeLine-owned cache files.

## Validation

Run from the repository root:

```sh
cargo metadata --locked --no-deps --format-version 1
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --locked
git diff --check
```

When dependencies are already cached, prefer `cargo test --locked --offline` and `cargo build --locked --offline` for a reproducible local check. Use the native-data smoke fixture in `README.md` to verify the rendered model, directory, context, `5h`, and `7d` fields.

## Documentation

- Update `CHANGELOG.md` for user-visible changes.
- Keep `README.md` and `README.zh.md` behaviorally aligned.
- Preserve `UPSTREAM.md` and `THIRD_PARTY_NOTICES.md` when renaming or reorganizing files.
- State analysis, build success, installation, migration, release, and publication as separate facts.
