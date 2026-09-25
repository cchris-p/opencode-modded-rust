#!/usr/bin/env bash
#
# Fetch the pinned scopemux-core source for the native FFI provider.
#
# Clones scopemux-core at the recorded revision into third_party/scopemux-core
# (gitignored) and initializes its submodules (Tree-sitter grammars, pybind11),
# which the C build needs. Re-run with --force to refresh an existing checkout.
#
# After this, build the native provider with:
#   cargo build -p opencode-scopemux --features native

set -euo pipefail

PINNED_REV="230383ea2ee315c2632ad5742e7a0fb4a04e89fa"
REPO_URL="${SCOPEMUX_CORE_REPO:-git@github.com:cchris-p/scopemux-core.git}"

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${SCOPEMUX_CORE_DIR:-${PROJECT_ROOT}/third_party/scopemux-core}"

if [ -d "${DEST}/.git" ]; then
    if [ "${1:-}" != "--force" ]; then
        echo "[fetch-scopemux-core] ${DEST} already exists; use --force to refresh"
        exit 0
    fi
    echo "[fetch-scopemux-core] Refreshing ${DEST}"
    git -C "${DEST}" fetch --all --tags
else
    echo "[fetch-scopemux-core] Cloning ${REPO_URL} into ${DEST}"
    mkdir -p "$(dirname "${DEST}")"
    git clone "${REPO_URL}" "${DEST}"
fi

echo "[fetch-scopemux-core] Checking out pinned revision ${PINNED_REV}"
git -C "${DEST}" checkout --quiet "${PINNED_REV}"

echo "[fetch-scopemux-core] Initializing submodules"
git -C "${DEST}" submodule update --init --recursive

echo "[fetch-scopemux-core] Done. Build with: cargo build -p opencode-scopemux --features native"
