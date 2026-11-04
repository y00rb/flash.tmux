# Changelog

Notable changes to flash.tmux are documented here.

## [0.2.1] - 2026-11-04

### Fixed

- Make the initialization binary-check message compatible with newer and older
  tmux versions: use non-freezing display when supported to keep short-lived
  popups visible, and fall back to ordinary display without failing startup
  when `-C` is unsupported. The fallback retains the older message-freezing
  behavior.

### Changed

- Generate GitHub Release descriptions from the matching version in this
  changelog, including when resuming a draft. Missing or empty entries stop
  publication before any GitHub API call.
