#!/bin/sh
set -eu
DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
binary=$(tmux show-option -gqv @flash-bin)
if [ -z "$binary" ]; then
    binary="$DIR/bin/flash-tmux"
    if ! error=$(sh "$DIR/scripts/ensure.sh" --check 2>&1); then
        tmux display-message -l -c "$2" "$error"
        exit 1
    fi
fi
if [ ! -x "$binary" ]; then
    tmux display-message -l -c "$2" 'flash-tmux: binary missing; build @flash-bin or reload the plugin to install the release binary'
    exit 1
fi
if ! error=$("$binary" start --pane "$1" --client "$2" --mode "${3:-search}" 2>&1); then
    tmux display-message -l -c "$2" "flash-tmux: ${error#flash-tmux: }"
    # The diagnostic is already visible; a nonzero run-shell exit would replace
    # it with tmux's generic "returned 1" message.
    exit 0
fi
