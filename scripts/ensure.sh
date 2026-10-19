#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$ROOT/scripts/common.sh"
version=$(flash_version "$ROOT")
target=$(flash_target)
if flash_installed "$ROOT" "$version" "$target"; then exit 0; fi
if [ "${1:-}" = --check ]; then
    echo "flash-tmux: missing or incompatible binary for v$version; reload the plugin or run scripts/ensure.sh to install it" >&2
    exit 1
fi
[ "$#" -eq 0 ] || { echo 'Usage: scripts/ensure.sh [--check]' >&2; exit 2; }
repo=$(flash_github_repo "$ROOT")
sh "$ROOT/scripts/install.sh" --version "v$version" --repo "$repo" --if-needed
flash_installed "$ROOT" "$version" "$target" || {
    echo 'flash-tmux: installation did not produce the required binary' >&2; exit 1;
}
