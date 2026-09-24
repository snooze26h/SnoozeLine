# Changelog

All notable SnoozeLine changes are recorded here. Entries before SnoozeLine remain available in the preserved Git history and the [CCometixLine changelog](https://github.com/Haleclipse/CCometixLine/blob/v1.1.2/CHANGELOG.md).

## [0.1.0] - Unreleased

### Documentation

- Added an actual `snooze26h` terminal screenshot to both READMEs.
- Reorganized the English and Chinese introductions around the preview, displayed fields, installation, and configuration.
- Documented the output-style field and partial quota availability, and aligned the smoke fixture with the preview.
- Made the settings-update example quote executable paths for shell execution.
- Corrected the current repository visibility to public across project and provenance documentation while retaining the unresolved upstream license notice.

### Identity

- Established the private local project name `SnoozeLine`.
- Set the package, library, binary, and CLI name to `snoozeline`.
- Started a clean downstream version at `0.1.0`; removed local `snoozer.*` version labels.
- Recorded the exact CCometixLine v1.1.2 base and downstream relationship.

### Status line

- Added live reasoning effort beside the model name using Claude Code's native `effort.level`; each refresh reflects the current session, including `/effort` and `/model` changes.
- Kept the model-only display for absent, null, malformed, or unknown effort values, without inferring a potentially stale level from settings or environment variables.
- Updated the configuration preview and native-data smoke fixture to show reasoning effort.
- Made `snooze26h` the default built-in theme.
- Kept the compact model, directory, context, quota, and Git layout.
- Removed the “shared” label and quota reset timestamp from the visible line.
- Preferred native context and quota data while retaining compatible fallbacks.
- Corrected current-context token semantics and bounded percentages to `0–100%`.
- Rendered quota values consistently as used percentages, including explicit missing values.
- Fixed `claude-fable-5-1` rendering as `Fable 5.1` instead of `Fable 5`.
- Preserved actual Claude minor versions when legacy model overrides use standard names (for example, `Opus 5.5` instead of `Opus 5`), while retaining custom aliases, context limits, and context suffixes.
- Hardened directory, Git, segment, and configuration handling against malformed input and slow commands.

### Runtime

- Moved SnoozeLine-owned files to `~/.claude/snoozeline`.
- Added absolute-path `SNOOZELINE_HOME` support.
- Disabled the inherited upstream npm update check.
- Installed SnoozeLine beside the existing `~/.claude/ccline` tree and switched only the Claude Code status-line command, with a timestamped settings backup for rollback.

### Distribution

- Created the private `snooze26h/SnoozeLine` repository and configured `origin`; no GitHub release or npm publication has been performed.
- The source repository is now public; the missing upstream license notice remains unresolved.

## Upstream baseline

- Project: [Haleclipse/CCometixLine](https://github.com/Haleclipse/CCometixLine)
- Release: `v1.1.2`
- Commit: `a73b166557662b2e79b83ea005fb297003748fb0`
- Import date: `2026-09-01`
