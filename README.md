# SnoozeLine

[English](README.md) | [中文](README.zh.md)

SnoozeLine is a compact, customizable Claude Code status line written in Rust. See your model, working directory, context usage, and 5-hour / 7-day quota usage in one line, with Git status and output style when available.

Independently maintained from [CCometixLine](https://github.com/Haleclipse/CCometixLine) v1.1.2, with `snooze26h` as the default theme.

## Preview

![SnoozeLine using the snooze26h theme: Fable 5.1, snooze26h folder, 26% context, 256.8k tokens, unavailable 5h quota, 24% 7d quota, and default output style](assets/snoozeline-preview.png)

An actual terminal screenshot of the `snooze26h` theme. Icons require a Nerd Font; colors follow your terminal palette.

## What it shows

| Field | Example | Meaning |
| --- | --- | --- |
| Model | `Fable 5.1` | Current model name |
| Directory | `snooze26h` | Current working folder |
| Context | `26% · 256.8k tokens` | Context window usage and current input plus cache input tokens |
| Quota | `5h - · 7d 24%` | **Used percentage** for each quota window; `-` means that window is unavailable |
| Git | Branch and status | Shown when Git information is available for the working directory |
| Output style | `default` | The output style reported by Claude Code |

Git information is absent from this screenshot. The final rocket icon and `default` label indicate the output style.

### Features

- **Compact default layout:** model, directory, context, quota, Git, and output style; no quota reset timestamp.
- **Native data first:** uses Claude Code's context and quota fields when available, with compatible fallbacks.
- **Terminal configuration UI:** preview themes and adjust segment visibility, order, colors, icons, and separators with `--config`.
- **Ten built-in themes:** `snooze26h`, `cometix`, `default`, `minimal`, `gruvbox`, `nord`, and four Powerline themes. Custom themes use TOML files.
- **Separate runtime directory:** configuration and cache live under `~/.claude/snoozeline`, allowing an existing `ccline` installation to remain available for rollback.

## Installation

The repository publicly hosts the source for version `0.1.0`; no GitHub release is published. The steps below build and install from source on macOS or Linux.

Requirements: Claude Code, Git, Rust stable, and a Nerd Font selected in your terminal for the default theme's icons. The `default` theme uses ordinary emoji icons instead.

### 1. Build and install

```sh
git clone https://github.com/snooze26h/SnoozeLine.git
cd SnoozeLine
cargo build --release --locked

install -d "$HOME/.claude/snoozeline"
install -m 0755 ./target/release/snoozeline "$HOME/.claude/snoozeline/snoozeline"
```

### 2. Connect to Claude Code

Back up `~/.claude/settings.json`, then merge the following `statusLine` entry into it, preserving your other settings. Replace the example command with the absolute path to the installed binary; keep the inner quotes if the path contains spaces.

```json
{
  "statusLine": {
    "type": "command",
    "command": "\"/absolute/path/to/.claude/snoozeline/snoozeline\"",
    "padding": 0
  }
}
```

Restart Claude Code to load the status line. When migrating from CCometixLine, keep `~/.claude/ccline` in place so you can restore the previous command.

<details>
<summary>Optional: back up and update an existing settings file with jq</summary>

Requires `jq` and an existing `~/.claude/settings.json`. This creates a timestamped backup and updates only the `statusLine` entry.

```sh
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
    '.statusLine = ((.statusLine // {}) + {"type":"command","command":($command | @sh),"padding":0})' \
    "$settings_file" > "$temp_file"
  mv "$temp_file" "$settings_file"
  trap - EXIT HUP INT TERM
  printf 'Backup saved to: %s\n' "$backup_file"
)
```

To roll back, replace the placeholder with the exact backup path printed above. This restores the entire settings file from that backup:

```sh
backup_file="/exact/backup/path/printed/above"
cp -p "$backup_file" "$HOME/.claude/settings.json"
```

</details>

## Configuration

Open the interactive editor:

```sh
"$HOME/.claude/snoozeline/snoozeline" --config
```

Press `p` to cycle through themes, `s` to save, and `Esc` to exit. The editor also supports changing individual segments and saving custom themes.

The default runtime root is `~/.claude/snoozeline`:

```text
~/.claude/snoozeline/
├── config.toml          # Status-line appearance and enabled segments
├── models.toml          # Model display-name overrides
├── themes/*.toml        # Built-in and custom themes
└── .api_usage_cache.json
```

Set `SNOOZELINE_HOME` to an absolute path to use another runtime root. Files are created as needed.

In `models.toml`, a Claude entry whose `display_name` matches the standard name for its `pattern` follows the actual model version. For example, `pattern = "claude-opus-5"` with `display_name = "Opus 5"` displays `Opus 5.5` for `claude-opus-5-5`, while preserving the entry's `context_limit`. Custom aliases such as `Work Opus` stay fixed, and context suffixes such as `1M` still apply.

## How usage is calculated

- Native Claude Code context data takes precedence. Current context tokens include input and cache input, excluding output tokens.
- Context and quota percentages are validated and clamped to `0–100%`.
- `5h` and `7d` show **used percentages**. When one window is unavailable, its value is `-`; when neither is available after fallback, the quota segment is hidden.
- If native quota data is unavailable, SnoozeLine can use the compatible Claude usage endpoint and an account-scoped cache.
- SnoozeLine's own cache does not store transcript content.

## Development

Run from the repository root:

```sh
cargo metadata --locked --no-deps --format-version 1
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --locked
git diff --check
```

After building, render a line using native fixture data and a temporary runtime directory:

```sh
smoke_root="$(mktemp -d)"
printf '%s\n' '{"model":{"id":"claude-fable-5-1","display_name":"Fable 5.1"},"workspace":{"current_dir":"/tmp/snoozeline-demo"},"context_window":{"context_window_size":1000000,"used_percentage":26,"current_usage":{"input_tokens":256800}},"rate_limits":{"five_hour":{},"seven_day":{"used_percentage":24}},"output_style":{"name":"default"}}' \
  | SNOOZELINE_HOME="$smoke_root" \
    ./target/debug/snoozeline --theme snooze26h
```

Expected fields: `Fable 5.1`, `snoozeline-demo`, `26% · 256.8k tokens`, `5h - · 7d 24%`, and `default`. This fixture does not require an account or a quota API request.

## Upstream and license

SnoozeLine derives from CCometixLine v1.1.2 by Haleclipse and contributors. The upstream Git history and authorship are preserved; SnoozeLine is independently maintained and is not an official upstream release.

The upstream package metadata and README declare `MIT`, but the imported v1.1.2 snapshot lacks the referenced `LICENSE` text. That notice remains unresolved; public source visibility does not resolve the missing notice. SnoozeLine does not supply an inferred license text or copyright holder.

See [UPSTREAM.md](UPSTREAM.md) for the exact base and downstream changes, [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) for attribution, and [CHANGELOG.md](CHANGELOG.md) for the change history.
