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

PINNED_REV="e93df0814fc3f0a0cd47d40cc9a68a4bc6f77ec5"
REPO_URL="${SCOPEMUX_CORE_REPO:-git@github.com:cchris-p/scopemux-core.git}"

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${SCOPEMUX_CORE_DIR:-${PROJECT_ROOT}/third_party/scopemux-core}"

run_as_root() {
    if [ "$(id -u)" -eq 0 ]; then
        "$@"
    elif [ -t 0 ]; then
        sudo "$@"
    elif [ -n "${SUDO_ASKPASS:-}" ]; then
        sudo -A "$@"
    else
        echo "[fetch-scopemux-core] cmake install requires sudo; rerun from a terminal or install cmake manually" >&2
        exit 1
    fi
}

ensure_cmake() {
    if command -v cmake >/dev/null 2>&1; then
        return
    fi

    echo "[fetch-scopemux-core] cmake not found; installing native build dependency"
    if command -v apt-get >/dev/null 2>&1; then
        run_as_root apt-get update
        run_as_root apt-get install -y cmake
    elif command -v dnf >/dev/null 2>&1; then
        run_as_root dnf install -y cmake
    elif command -v yum >/dev/null 2>&1; then
        run_as_root yum install -y cmake
    elif command -v pacman >/dev/null 2>&1; then
        run_as_root pacman -Sy --needed --noconfirm cmake
    elif command -v brew >/dev/null 2>&1; then
        brew install cmake
    else
        echo "[fetch-scopemux-core] Unable to install cmake automatically; install cmake and rerun this script" >&2
        exit 1
    fi
}

ensure_cmake

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
git -C "${DEST}" \
    -c 'url.git@github.com:.insteadOf=https://github.com/' \
    submodule update --init --recursive

echo "[fetch-scopemux-core] Done. Build with: cargo build -p opencode-scopemux --features native"
