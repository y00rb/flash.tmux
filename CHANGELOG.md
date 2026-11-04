# Changelog

Notable changes to flash.tmux are documented here.

## [0.2.1] - Unreleased

### Fixed

- Make the initialization binary-check message compatible with newer and older
  tmux versions: use non-freezing display when supported to keep short-lived
  popups visible, and fall back to ordinary display without failing startup
  when `-C` is unsupported. The fallback retains the older message-freezing
  behavior.
