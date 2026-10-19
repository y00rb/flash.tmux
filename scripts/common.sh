#!/bin/sh
# Shared functions only; sourcing this file has no installation side effects.
flash_valid_version() {
    case "$1" in *[!0-9A-Za-z.-]*|'') return 1 ;; esac
    printf '%s\n' "$1" | LC_ALL=C grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+(-[A-Za-z0-9]+([.-][A-Za-z0-9]+)*)?$'
}
flash_version() {
    version=$(cat "$1/VERSION") || return 1
    flash_valid_version "$version" || { echo 'flash-tmux: invalid VERSION file' >&2; return 1; }
    printf '%s\n' "$version"
}
flash_target() {
    case "$(uname -s)/$(uname -m)" in
        Darwin/arm64|Darwin/aarch64) echo aarch64-apple-darwin ;;
        Darwin/x86_64) echo x86_64-apple-darwin ;;
        Linux/aarch64|Linux/arm64) echo aarch64-unknown-linux-musl ;;
        Linux/x86_64) echo x86_64-unknown-linux-musl ;;
        *) echo 'flash-tmux: unsupported platform; build from source and set @flash-bin' >&2; return 1 ;;
    esac
}
flash_installed() {
    [ -x "$1/bin/flash-tmux" ] && [ -f "$1/bin/install-info" ] || return 1
    [ "$(cat "$1/bin/install-info")" = "$2 $3" ] || return 1
    [ "$("$1/bin/flash-tmux" --version 2>/dev/null)" = "flash-tmux $2" ]
}
flash_github_repo() {
    # Never echo the original remote: HTTPS remotes can contain credentials.
    remote=$(git -C "$1" config --get remote.origin.url) || {
        echo 'flash-tmux: no Git origin; use scripts/install.sh --version vX.Y.Z --repo y00rb/flash.tmux' >&2; return 1;
    }
    case "$remote" in
        https://github.com/*) repo=${remote#https://github.com/} ;;
        # TPM's public shorthand clone uses this fixed, non-secret userinfo.
        https://git::@github.com/*) repo=${remote#https://git::@github.com/} ;;
        git@github.com:*) repo=${remote#git@github.com:} ;;
        ssh://git@github.com/*) repo=${remote#ssh://git@github.com/} ;;
        *) echo 'flash-tmux: origin must be a public GitHub HTTPS or SSH repository; use manual installation' >&2; return 1 ;;
    esac
    repo=${repo%.git}
    case "$repo" in *[!A-Za-z0-9_./-]*|'') return 1 ;; esac
    printf '%s\n' "$repo" | LC_ALL=C grep -Eq '^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$' || {
        echo 'flash-tmux: cannot identify GitHub OWNER/REPO' >&2; return 1;
    }
    printf '%s\n' "$repo"
}
