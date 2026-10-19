#!/bin/sh
set -eu
DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
set_default() {
    if [ -z "$(tmux show-option -gq "@$1")" ]; then
        tmux set-option -g "@$1" "$2"
    fi
}
set_default flash-key S
set_default flash-labels asdfghjklASDFHJKLqwertyuiopzxcvbnmQWERTYUIOPZXCVBNM
set_default flash-label-bg '#f4b8e4'
set_default flash-label-fg '#303446'
set_default flash-match-fg '#c6d0f5'
set_default flash-match-bg '#51576d'
set_default flash-prompt-bg '#8caaee'
set_default flash-prompt-fg '#303446'

binary=$(tmux show-option -gqv @flash-bin)
if [ -z "$binary" ]; then
    binary="$DIR/bin/flash-tmux"
    tmux display-message -l 'flash-tmux: checking release binary...'
    if ! error=$(sh "$DIR/scripts/ensure.sh" 2>&1); then
        tmux display-message -l "flash-tmux: installation failed. $error"
        exit 1
    fi
fi
if [ ! -x "$binary" ]; then
    tmux display-message -l 'flash-tmux: @flash-bin is not executable; build it or remove the override'
    exit 1
fi
if ! error=$("$binary" validate-config 2>&1); then
    tmux display-message -l "flash-tmux: $error"
    exit 1
fi

# Quote the install directory once; tmux quotes pane/client values on expansion.
escaped_dir=$(printf '%s' "$DIR" | sed "s/'/'\\\\''/g")
command="sh '$escaped_dir/scripts/launch.sh' #{q:pane_id} #{q:client_name}"
key=$(tmux show-option -gqv @flash-key)
old=$(tmux show-option -gqv @flash-bound-key)
for table in copy-mode copy-mode-vi; do
    if [ -n "$old" ] && [ "$old" != "$key" ]; then
        binding=$(tmux list-keys -T "$table" "$old" 2>/dev/null || :)
        case "$binding" in
            *"$DIR/scripts/launch.sh"*) tmux unbind-key -T "$table" "$old" ;;
        esac
    fi
    tmux bind-key -T "$table" "$key" run-shell -b "$command"
done
tmux set-option -g @flash-bound-key "$key"
