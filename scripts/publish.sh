#!/bin/sh
# CI publishing only. Never invoked by the installer or TPM.
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
[ "$#" -eq 2 ] || { echo 'Usage: publish.sh TAG DIST' >&2; exit 2; }
tag=$1
dist=$(CDPATH= cd -- "$2" && pwd)
: "${GH_REPO:?Set GH_REPO to OWNER/REPO}"
python3 "$ROOT/scripts/check-release.py" --tag "$tag"
for target in aarch64-apple-darwin x86_64-apple-darwin aarch64-unknown-linux-musl x86_64-unknown-linux-musl; do
    archive="$dist/flash-tmux-$tag-$target.tar.gz"
    [ -f "$archive" ] || { echo "Missing release target: $target" >&2; exit 1; }
    [ "$(tar -tzf "$archive")" = flash-tmux ] || { echo 'Unexpected archive content' >&2; exit 1; }
done
set -- "$dist"/*.tar.gz
[ "$#" -eq 4 ] || { echo 'Expected exactly four release archives' >&2; exit 1; }
(
    cd "$dist"
    if command -v sha256sum >/dev/null 2>&1; then sha256sum ./*.tar.gz;
    else shasum -a 256 ./*.tar.gz; fi
) | sed 's|  \./|  |' > "$dist/SHA256SUMS"
error=$(mktemp)
trap 'rm -f "$error"' EXIT
if draft=$(gh api "repos/$GH_REPO/releases/tags/$tag" --jq .draft 2>"$error"); then
    [ "$draft" = true ] || { echo 'Release is already public; publish a new version instead of replacing it' >&2; exit 1; }
else
    if ! grep -q 'HTTP 404' "$error"; then cat "$error" >&2; exit 1; fi
    case "$tag" in
        *-*) gh release create "$tag" --draft --prerelease --verify-tag --generate-notes --title "$tag" ;;
        *) gh release create "$tag" --draft --verify-tag --generate-notes --title "$tag" ;;
    esac
fi
# Only drafts reach this point; a failed upload can be retried safely.
gh release upload "$tag" "$dist"/*.tar.gz "$dist/SHA256SUMS" --clobber
case "$tag" in
    *-*) gh release edit "$tag" --draft=false --prerelease --latest=false ;;
    *) gh release edit "$tag" --draft=false --prerelease=false --latest ;;
esac
