# Changelog

Notable changes to flash.tmux are documented here.

## [0.2.1] - Unreleased

### Fixed

- Prevent the initialization binary-check message from freezing tmux client
  updates. Previously, reloading the plugin could leave short-lived popups
  blank until the message expired.
