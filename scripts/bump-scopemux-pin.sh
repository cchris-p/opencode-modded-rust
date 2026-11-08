#!/usr/bin/env bash
#
# Advance the pinned scopemux-core revision and verify the native provider.
#
# Usage: scripts/bump-scopemux-pin.sh <scopemux-core-rev>
#
# Updates PINNED_REV in scripts/fetch-scopemux-core.sh, refreshes the
# third_party checkout at the new revision, and runs the native provider tests.
# It does not commit or open a PR: review the generated diff, then land it
# through the normal feature-branch/PR flow and record the new pin on the board.

set -euo pipefail

REV="${1:-}"
if [ -z "${REV}" ]; then
    echo "usage: $(basename "$0") <scopemux-core-rev>" >&2
    exit 2
fi
case "${REV}" in
    *[!0-9a-f]*)
        echo "error: '${REV}' is not a lowercase hex commit id" >&2
        exit 2
        ;;
esac
if [ "${#REV}" -lt 7 ]; then
    echo "error: '${REV}' is too short to be a commit id" >&2
    exit 2
fi

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FETCH="${PROJECT_ROOT}/scripts/fetch-scopemux-core.sh"

if ! grep -q '^PINNED_REV=' "${FETCH}"; then
    echo "error: PINNED_REV not found in ${FETCH}" >&2
    exit 1
fi

before="$(sed -n 's/^PINNED_REV="\([0-9a-f]*\)".*/\1/p' "${FETCH}" | head -n 1)"
echo "[bump-scopemux-pin] ${before:-unknown} -> ${REV}"

tmp="$(mktemp)"
sed "s|^PINNED_REV=.*|PINNED_REV=\"${REV}\"|" "${FETCH}" > "${tmp}"
mv "${tmp}" "${FETCH}"
chmod +x "${FETCH}"

echo "[bump-scopemux-pin] Fetching ${REV} and initializing submodules"
bash "${FETCH}" --force

echo "[bump-scopemux-pin] Confirming the checkout matches the pin"
bash "${FETCH}" --check

echo "[bump-scopemux-pin] Running native provider tests"
(
    cd "${PROJECT_ROOT}"
    cargo test -p opencode-scopemux --features native
)

cat <<EOF

[bump-scopemux-pin] Done. Next steps:
  1. Review the diff for scripts/fetch-scopemux-core.sh.
  2. Commit on a feature branch and open a PR into development.
  3. Record the new pin and why it was advanced on the board card.
EOF
