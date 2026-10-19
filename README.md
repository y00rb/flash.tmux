# flash.tmux

English | [简体中文](README.zh-CN.md)

Incremental search and label navigation in tmux copy-mode, inspired by [flash.nvim](https://github.com/folke/flash.nvim) by [folke](https://github.com/folke), with a Rust runtime and a small TPM entrypoint.

## Screenshots and Walkthrough

Captured in Ghostty with zsh, JetBrains Mono, and the Catppuccin Frappe theme, using this repository's actual files. These examples use vi copy-mode with `v` configured to begin selection. Selection, copy, pane-navigation, and split bindings are your tmux settings, not bindings installed by flash.tmux.

### 1. Search and Jump Labels

**Steps:** In the repository, run `head -n 16 src/config.rs`. Enter copy-mode with `prefix + [`, press `S`, and type `flash`. Pause before choosing a label.

**What to look for:** The blue `⚡` prompt shows the query and match/label counts. Matches are highlighted and pink single-character labels appear beside them. Press one of the displayed labels to jump to that match's start. Labels depend on the current cursor and visible text; use the labels on your screen rather than memorizing those in the image.

![Searching visible Rust configuration text with highlighted matches and pink jump labels](assets/screenshots/1-search.png)

### 2. Extend a Selection with Search

**Steps:** From the same file output, start in copy-mode with no active selection:

```text
S → DEFAULTS: → Enter → 0 → v
S → flash-prompt-fg → Enter → j → $
```

The first search locates the declaration; `0` moves to the line start and `v` anchors the selection. The second search extends it toward the last option; `j` and `$` include the closing `];` line.

**What to look for:** The complete `DEFAULTS` block is selected and search labels have disappeared. This frame shows the completed selection stage. Next, press your configured `y` key to copy. With a non-exiting copy action such as tmux-yank's `copy-pipe`, you stay in copy-mode and can repeat the search/select/copy cycle.

![The complete DEFAULTS block selected after locating its endpoints with search](assets/screenshots/2-selection.png)

### 3. Search Only the Active Pane

**Steps:** Leave copy-mode and split the window left/right using your tmux binding (`prefix + h` in this setup). Run `head -n 15 Cargo.toml` in the right pane. In the left pane, run `head -n 16 src/config.rs`, enter copy-mode, press `S`, and type `flash-`. Pause before choosing a label.

**What to look for:** Only the left pane is dimmed and annotated with matches and labels. The right pane keeps its normal colors and has no search labels. The blue `⚡` prompt shows the query and match/label counts.

![Active-pane search on the left while Cargo.toml remains unchanged in the right pane](assets/screenshots/3-split-pane.png)

## Requirements

- tmux 3.3 or newer with popup support; currently tested locally on tmux 3.6a, macOS arm64.
- A visible tmux status line at the top or bottom. Search uses the message line for its prompt so that no source row is hidden. `status off` is rejected without modifying the pane.
- A true-color terminal for the default Catppuccin Frappe colors.
- A compatible prebuilt binary, or Rust/Cargo to build from source. Rust is not needed to run a prebuilt binary. Python 3.11+ is only used by development/release tests.
- For automatic installation: Git, curl, tar, and either sha256sum or shasum. Public GitHub releases must be reachable.

## Local Development / Source Installation

Replace `/path/to/flash.tmux` with your own checkout directory:

```sh
git clone https://github.com/y00rb/flash.tmux.git /path/to/flash.tmux
cd /path/to/flash.tmux
cargo build
```

Add the following to `tmux.conf`, using the same checkout path:

```tmux
set -g @flash-bin '/path/to/flash.tmux/target/debug/flash-tmux'
run-shell 'sh /path/to/flash.tmux/flash.tmux'
```

Reload your tmux configuration to load the plugin. After source changes, run `cargo build` again. Binary-only updates do not require a reload; binding or option changes do.

For an optimized build, run `cargo build --release` and point `@flash-bin` to `/path/to/flash.tmux/target/release/flash-tmux` instead.

An explicit `@flash-bin` always bypasses release downloads, even if the path is invalid (an error is shown instead). Loading never runs Cargo, and pressing S never builds or downloads.

## TPM and Prebuilt Releases

Configure TPM with a published version tag:

```tmux
# Put this before TPM initialization; use a published version tag.
set -g @plugin 'y00rb/flash.tmux#v0.1.0'
```

Press `prefix + I`. TPM clones and loads the plugin; `flash.tmux` then installs the binary required by the checked-out `VERSION` into the plugin's `bin/` directory. It identifies OWNER/REPO from the public GitHub HTTPS/SSH origin, never downloads `latest`, and does not require Rust. A compatible installed binary is reused without network access. Private/authenticated release downloads are not supported automatically.

Pinned tags are detached checkouts: TPM's `prefix + U` uses `git pull` and does not switch them to a new tag. Change the configured tag, then explicitly fetch/check out the new tag in the installed plugin directory and reload the plugin:

```sh
PLUGIN_DIR='/path/to/tpm/plugins/flash.tmux'
git -C "$PLUGIN_DIR" fetch origin tag v0.1.1
git -C "$PLUGIN_DIR" checkout --detach v0.1.1
```

Use the actual directory/version and inspect local changes before checkout; do not force it. Verify the checkout and `bin/flash-tmux --version` agree after loading. For branch-tracking installs without `#tag`, `prefix + U` can pull branch changes; wait for the matching Release to exist before updating. The entrypoint synchronizes the binary when loaded. An unavailable release reports an error; an incompatible old executable is not used for search.

For local development, keep `@flash-bin` set. Remove that override from your config and run `tmux set-option -gu @flash-bin` before reloading to test a Release install; deleting a config line alone does not clear a running tmux server's option. If a download fails, wait until the Release is available and reload the plugin, or run `sh scripts/ensure.sh` from its directory. Search never downloads.

You can also install a specific published version manually:

```sh
# Run from the installed plugin directory; substitute the actual repository/tag.
sh scripts/install.sh --version v0.1.0 --repo y00rb/flash.tmux
```

The installer chooses macOS arm64/x86_64 or Linux arm64/x86_64 musl, verifies SHA-256 and the program version, then replaces `bin/flash-tmux` atomically and records its version/platform in `bin/install-info`. It preserves the prior binary if download, verification or execution fails. Concurrent installers are serialized; if an installer is forcibly killed, inspect `bin/.install-lock/pid` and confirm its process has exited before removing that stale lock and retrying. Unsupported/offline systems can use a source build with `@flash-bin`.

Publishing is triggered by pushing a `vX.Y.Z` tag. `VERSION`, Cargo.toml and the tag must agree. Release depends on successful checks, builds all four targets, verifies binary versions and prepares complete draft assets before publishing. Failed draft uploads can resume; published releases are never overwritten. Use a new version for fixes. Prereleases are marked accordingly; ordinary branch pushes never publish a Release.

Linux musl builds aim to avoid glibc-version dependence; they are not a guarantee of support for every kernel, terminal or distribution. Checksums protect against corruption, not a compromised publisher or release channel.

## Usage

1. Enter tmux copy-mode, normally `prefix + [`.
2. Press **S** (Shift+s).
3. Type visible text to narrow literal matches.
4. Press a displayed label to jump to the start of its match, or Enter to choose the nearest match.
5. Press S again for another search.

The prompt appears as `⚡ query  [3 matches / 3 labels]`, with a Frappe Blue background by default. Matches use a slightly brighter Surface1 background; labels keep their Pink background.

Backspace removes one grapheme. Esc or Ctrl+C cancels search without changing the original cursor/selection. Enter with no matches stays in search. Lowercase queries are case-insensitive; uppercase queries are case-sensitive. Labels are single ASCII characters; upper/lowercase labels are separate keys. A label that could continue any match is not used, so further typing remains possible. If there are too many matches, narrow the query.

Labels normally appear immediately after matches. At a row edge, a safe neighboring cell may be used instead. Where no safe cell is available, that match remains highlighted without a label; narrowing the query and Enter still reaches it. Searches do not cross wrapped screen-row boundaries and do not search offscreen history. Scroll first, then search.

The original pane is neither swapped nor resized. A resize or changed copy-mode state cancels a pending jump. Only one search per pane or client can run simultaneously. Search cancellation caused by closing the popup from tmux also leaves copy-mode intact.

## Selection and Repeated Copying

The plugin does not bind **v**, **y** or **q**, change clipboard settings, or depend on tmux-yank. Use your own copy-mode bindings:

```text
S -> search -> label -> v -> S -> search -> label -> y
```

Jumping extends an existing selection. Whether y exits copy-mode is entirely determined by your own binding. With tmux-yank, configure before TPM initialization:

```tmux
set -g @yank_action 'copy-pipe'
```

Then y copies without exiting, and you can repeat search/select/y until q exits. Later bindings can override tmux-yank; check the effective binding with `tmux list-keys -T copy-mode-vi y`.

During search, q and S are ordinary query characters or assigned labels, not exit/restart commands. Cancel search with Esc first if you want q to invoke your normal copy-mode exit binding. With an exiting y binding, re-enter copy-mode before using S again.

## Configuration

Configure options **before TPM initialization or manual plugin loading**. Unset options receive defaults; user values are not overwritten.

```tmux
set -g @flash-key 'S'
set -g @flash-labels 'asdfghjklASDFHJKLqwertyuiopzxcvbnmQWERTYUIOPZXCVBNM'
set -g @flash-label-bg '#f4b8e4'   # Frappe Pink
set -g @flash-label-fg '#303446'   # Base
set -g @flash-match-fg '#c6d0f5'   # Text
set -g @flash-match-bg '#51576d'   # Surface1
set -g @flash-prompt-bg '#8caaee'  # Blue
set -g @flash-prompt-fg '#303446'  # Base
```

Colors accept `#RRGGBB`. Empty match/prompt backgrounds use the terminal default; empty foregrounds or label backgrounds are invalid. Labels must be unique printable non-space ASCII characters. The trigger accepts tmux key names, is registered only in the `copy-mode` and `copy-mode-vi` key tables, and does not capture Neovim keys outside tmux copy-mode.

`@flash-bin` optionally overrides the executable path for local builds. The plugin tracks `@flash-bound-key` internally to remove its previous trigger on reload without deleting user replacement bindings.

## Testing

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build
python3 tests/tmux_probe.py
python3 tests/e2e.py
sh tests/plugin.sh
sh tests/install.sh
python3 tests/bootstrap_test.py
python3 tests/release_version_test.py
python3 tests/publish_test.py
python3 scripts/check-release.py --binary target/debug/flash-tmux
```

Maintainers also run `python3 scripts/check-release.py --tag v0.1.0` before pushing a release tag, using the intended version. Bootstrap tests mock GitHub downloads and exercise the entrypoint in an isolated tmux server. Publishing tests mock `gh`, including draft recovery and refusal to overwrite public releases; they do not publish anything.

The Python terminal tests create unique, disposable tmux servers and PTYs, never your live session. Clipboard operations in tests are replaced with local temporary-file commands. Neovim geometry coverage runs when `nvim` is available. Installer tests use local download mocks and require no network.

Captured tabs use standard eight-column tab stops; custom terminal tab stops are not verified. Unicode tests cover CJK, combining accents, basic emoji, a ZWJ sequence and regional-indicator flags on the tested tmux. Terminal/tmux Unicode tables can differ on other versions; report rendering or jump mismatches with both versions. Input and matching preserve literal Unicode sequences rather than normalizing composed/decomposed forms.

## Runtime Files

The executable uses a private directory at `$XDG_CACHE_HOME/flash-tmux` or `~/.cache/flash-tmux` for per-pane/per-client OS locks and private search snapshots. Normal searches clean their temporary snapshots; the next search for a pane cleans stale snapshots from a crashed search. Small lock files remain reusable and OS locks are released automatically on process exit.
