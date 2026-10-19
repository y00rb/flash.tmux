#!/bin/sh
# Explicit provisioning, also used by the TPM entrypoint; never by search.
set -eu
umask 077
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$ROOT/scripts/common.sh"
tag=
repo=
if_needed=0
while [ "$#" -gt 0 ]; do
    case "$1" in
        --version) [ "$#" -ge 2 ] || exit 2; tag=$2; shift 2 ;;
        --repo) [ "$#" -ge 2 ] || exit 2; repo=$2; shift 2 ;;
        --if-needed) if_needed=1; shift ;;
        *) echo 'Usage: scripts/install.sh --version vX.Y.Z --repo OWNER/REPO [--if-needed]' >&2; exit 2 ;;
    esac
done
case "$tag" in v*) version=${tag#v} ;; *) echo 'Provide --version vX.Y.Z' >&2; exit 2 ;; esac
flash_valid_version "$version" || { echo 'Invalid release version' >&2; exit 2; }
case "$repo" in *[!A-Za-z0-9_./-]*|'') echo 'Invalid repository' >&2; exit 2 ;; esac
printf '%s\n' "$repo" | LC_ALL=C grep -Eq '^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$' || {
    echo 'Provide --repo OWNER/REPO' >&2; exit 2;
}
target=$(flash_target)
mkdir -p "$ROOT/bin"
lock="$ROOT/bin/.install-lock"
work=
owned=0
cleanup() {
    [ -z "$work" ] || rm -rf "$work"
    if [ "$owned" = 1 ]; then rm -f "$lock/pid"; rmdir "$lock"; fi
}
trap cleanup EXIT
trap 'exit 1' HUP INT TERM
# A directory lock works before Rust exists and on both macOS and Linux.
# Do not steal locks: an interrupted owner can require explicit local cleanup.
tries=0
until mkdir "$lock" 2>/dev/null; do
    tries=$((tries + 1))
    if [ "$tries" -ge 30 ]; then
        echo "flash-tmux: installation busy or interrupted; inspect $lock/pid before removing a stale lock and retrying" >&2
        exit 1
    fi
    sleep 1
done
owned=1
printf '%s\n' "$$" > "$lock/pid"
if [ "$if_needed" = 1 ] && flash_installed "$ROOT" "$version" "$target"; then exit 0; fi
command -v curl >/dev/null 2>&1 || { echo 'curl is required' >&2; exit 1; }
work=$(mktemp -d "$ROOT/bin/.install.XXXXXXXX")
asset="flash-tmux-$tag-$target.tar.gz"
base="https://github.com/$repo/releases/download/$tag"
echo "flash-tmux: installing $tag for $target" >&2
curl -fsSL --connect-timeout 15 --max-time 180 -o "$work/$asset" "$base/$asset"
curl -fsSL --connect-timeout 15 --max-time 60 -o "$work/SHA256SUMS" "$base/SHA256SUMS"
expected=$(awk -v name="$asset" '$2 == name { print $1 }' "$work/SHA256SUMS")
case "$expected" in *[!0-9a-fA-F]*|'') echo 'flash-tmux: missing or ambiguous checksum' >&2; exit 1 ;; esac
[ "${#expected}" -eq 64 ] || { echo 'flash-tmux: malformed checksum' >&2; exit 1; }
if command -v sha256sum >/dev/null 2>&1; then
    actual=$(sha256sum "$work/$asset" | awk '{print $1}')
else
    actual=$(shasum -a 256 "$work/$asset" | awk '{print $1}')
fi
expected=$(printf '%s' "$expected" | tr 'A-F' 'a-f')
[ "$expected" = "$actual" ] || { echo 'flash-tmux: checksum mismatch' >&2; exit 1; }
# Extract only the expected file to stdout; archive paths cannot escape work.
tar -xzOf "$work/$asset" flash-tmux > "$work/flash-tmux"
[ -s "$work/flash-tmux" ] || { echo 'flash-tmux: release binary is empty' >&2; exit 1; }
chmod 755 "$work/flash-tmux"
[ "$("$work/flash-tmux" --version)" = "flash-tmux $version" ] || {
    echo 'flash-tmux: binary version does not match requested release' >&2; exit 1;
}
printf '%s %s\n' "$version" "$target" > "$work/install-info"
mv -f "$work/flash-tmux" "$ROOT/bin/flash-tmux"
mv -f "$work/install-info" "$ROOT/bin/install-info"
printf 'Installed %s (%s) at %s/bin/flash-tmux\n' "$tag" "$target" "$ROOT"
