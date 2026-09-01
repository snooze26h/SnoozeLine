# SnoozeLine npm packaging

This directory contains private, local-only packaging scaffolding for SnoozeLine.
None of these packages are configured for publication.

Set `SNOOZELINE_SKIP_POSTINSTALL=1` to prevent a local npm install from copying
a binary into `~/.claude/snoozeline`.

The `snoozeline` wrapper first checks
`~/.claude/snoozeline/snoozeline` (`snoozeline.exe` on Windows), then looks for
the matching private platform package. It never falls back to a legacy
installation path.
