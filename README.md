# SnoozeLine

[English](README.md) | [中文](README.zh.md)

SnoozeLine is a compact Claude Code status line for private, local use. Its default built-in theme is `snooze26h`.

> **Origin:** SnoozeLine is an independently maintained derivative of [CCometixLine](https://github.com/Haleclipse/CCometixLine) v1.1.2 by Haleclipse and contributors. It is not an official upstream release. See [UPSTREAM.md](UPSTREAM.md) and [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) for the exact base, attribution, and license evidence.

## Project state

- Version: `0.1.0` (unreleased)
- Repository: private and local; no `origin` remote
- Distribution: no GitHub release and no SnoozeLine npm package
- Installation: installed side by side at `~/.claude/snoozeline/snoozeline`; Claude Code now uses SnoozeLine while the existing `~/.claude/ccline` tree remains available for rollback

## Display

The default line keeps the information intentionally small:

```text
Model | folder | context% · tokens | 5h% · 7d% | Git branch/status
```

The `snooze26h` theme does not add a “shared” label or quota reset date.

## Data rules

- Native Claude Code context data takes precedence.
- Current context tokens include input and cache input, not output tokens.
- Context and quota percentages are validated and clamped to `0–100%`.
- Native `5h` and `7d` values mean **used percentage**; unavailable values are shown as `-`, not fabricated.
- When native quota data is absent, the compatible Claude usage endpoint and an account-scoped cache may be used.
- Transcript content is not copied into SnoozeLine's cache.

## Runtime files

The default runtime root is `~/.claude/snoozeline`:

```text
~/.claude/snoozeline/
├── config.toml
├── models.toml
├── themes/*.toml
└── .api_usage_cache.json
```

Set `SNOOZELINE_HOME` to an absolute path to use another root. SnoozeLine does not automatically move or delete files under `~/.claude/ccline`.

## Build and test

Rust stable is required.

```sh
cargo metadata --locked --no-deps --format-version 1
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --locked
git diff --check
```

A local smoke test using only native fixture data; the model segment should render `Fable 5.1`:

```sh
printf '%s\n' '{"model":{"id":"claude-fable-5-1","display_name":"Fable 5.1"},"workspace":{"current_dir":"/tmp/snoozeline-demo"},"context_window":{"context_window_size":1000000,"used_percentage":24,"current_usage":{"input_tokens":242700}},"rate_limits":{"five_hour":{"used_percentage":18},"seven_day":{"used_percentage":4}}}' \
  | SNOOZELINE_HOME=/tmp/snoozeline-smoke \
    ./target/debug/snoozeline --theme snooze26h
```

## Local installation and migration

This migration keeps SnoozeLine beside the existing `ccline`, backs up Claude settings, and changes only the status-line command. This machine has completed the same reversible migration.

```sh
cargo build --release --locked

install -d "$HOME/.claude/snoozeline"
install -m 0755 ./target/release/snoozeline "$HOME/.claude/snoozeline/snoozeline"

settings_file="$HOME/.claude/settings.json"
(
  set -eu
  settings_dir="$(dirname "$settings_file")"
  backup_file="$(mktemp "$settings_dir/settings.json.before-snoozeline.$(date +%Y%m%d-%H%M%S).XXXXXX")"
  temp_file="$(mktemp "$settings_dir/.settings.json.snoozeline.XXXXXX")"
  trap 'rm -f "$temp_file"' EXIT HUP INT TERM

  cp -p "$settings_file" "$backup_file"
  cp -p "$settings_file" "$temp_file"
  jq --arg command "$HOME/.claude/snoozeline/snoozeline" \
    '.statusLine = ((.statusLine // {}) + {"type":"command","command":$command,"padding":0})' \
    "$settings_file" > "$temp_file"
  mv "$temp_file" "$settings_file"
  trap - EXIT HUP INT TERM
  printf 'Backup saved to: %s\n' "$backup_file"
)
```

Restart Claude Code after migration. To roll back:

```sh
backup_file="/exact/backup/path/printed/above"
cp -p "$backup_file" "$HOME/.claude/settings.json"
```

## License and provenance

The upstream project declares `MIT` in its package metadata and README, but the v1.1.2 source snapshot does not contain the referenced `LICENSE` text. SnoozeLine therefore does not invent a license file or copyright holder. This repository remains private pending clarification of the exact upstream notice.
