#!/bin/sh
set -eu
DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
set_default() {
    if [ -z "$(tmux show-option -gq "@$1")" ]; then
        tmux set-option -g "@$1" "$2"
    fi
}
set_default flash-key s
set_default flash-treesitter-key S
set_default flash-treesitter-search-key R
set_default flash-treesitter-language auto
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
cleanup() {
    key=$(tmux show-option -gqv "@$1")
    old=$(tmux show-option -gqv "@$2")
    if [ -z "$old" ] || [ "$old" = "$key" ]; then return; fi
    for table in copy-mode copy-mode-vi; do
        binding=$(tmux list-keys -T "$table" "$old" 2>/dev/null || :)
        current=$(tmux list-keys -T "$table" "$key" 2>/dev/null || :)
        # Compare tmux's parsed identities too: M-C-a and C-M-a may name
        # the same key even though their option strings differ.
        if [ -n "$binding" ] && [ "$binding" = "$current" ]; then continue; fi
        case "$binding" in
            *"$DIR/scripts/launch.sh"*) tmux unbind-key -T "$table" "$old" ;;
        esac
    done
}
# Clean every old key before installing any new key, including key swaps.
cleanup flash-key flash-bound-key
cleanup flash-treesitter-key flash-bound-treesitter-key
cleanup flash-treesitter-search-key flash-bound-treesitter-search-key
register() {
    key=$(tmux show-option -gqv "@$1")
    old=$(tmux show-option -gqv "@$3")
    for table in copy-mode copy-mode-vi; do
        if [ -n "$old" ]; then
            binding=$(tmux list-keys -T "$table" "$old" 2>/dev/null || :)
            case "$binding" in
                *"$DIR/scripts/launch.sh"*) : ;;
                *)
                    current=$(tmux list-keys -T "$table" "$key" 2>/dev/null || :)
                    if [ "$old" = "$key" ] || { [ -n "$binding" ] && [ "$binding" = "$current" ]; }; then continue; fi ;;
            esac
        fi
        tmux bind-key -T "$table" "$key" run-shell -b "$command $2"
    done
    tmux set-option -g "@$3" "$key"
}
register flash-key search flash-bound-key
register flash-treesitter-key treesitter flash-bound-treesitter-key
register flash-treesitter-search-key treesitter-search flash-bound-treesitter-search-key
