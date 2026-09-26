#!/usr/bin/env bash
#
# Fetch the pinned scopemux-core source for the native FFI provider.
#
# Clones scopemux-core at the recorded revision into third_party/scopemux-core
# (gitignored) and initializes its submodules (Tree-sitter grammars, pybind11),
# which the C build needs.
#
# `ort-build` runs this automatically when the source is missing and warns when
# an existing checkout drifts from the pin; set OPENCODE_RUST_REFRESH_CORE=1 to
# refresh a drifted checkout.
#
# Options:
#   --force   re-check out the pinned revision and refresh submodules even if
#             the checkout already matches the pin
#   --check   verify the checkout matches the pin without changing it; exits
#             non-zero when missing or stale (for CI and pin-bump tooling)
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
        echo "[fetch-scopemux-core] installing native build dependencies requires sudo; rerun from a terminal or install them manually" >&2
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

have_python_dev() {
    local py="${PYTHON:-python3}"
    command -v "${py}" >/dev/null 2>&1 || return 1
    "${py}" - <<'PY' >/dev/null 2>&1
import glob
import os
import sysconfig

include = sysconfig.get_path("include")
if not include or not os.path.exists(os.path.join(include, "Python.h")):
    raise SystemExit(1)

libdir = sysconfig.get_config_var("LIBDIR")
if libdir and not glob.glob(os.path.join(libdir, "libpython*")):
    raise SystemExit(1)
PY
}

ensure_python_dev() {
    if have_python_dev; then
        return
    fi

    echo "[fetch-scopemux-core] Python development files not found; installing native build dependency"
    local py_version
    py_version="$("${PYTHON:-python3}" -c 'import sys; print(f"{sys.version_info.major}.{sys.version_info.minor}")' 2>/dev/null || true)"

    if command -v apt-get >/dev/null 2>&1; then
        run_as_root apt-get update
        if [ -n "${py_version}" ]; then
            run_as_root apt-get install -y "python${py_version}-dev" ||
                run_as_root apt-get install -y python3-dev
        else
            run_as_root apt-get install -y python3-dev
        fi
    elif command -v dnf >/dev/null 2>&1; then
        run_as_root dnf install -y python3-devel
    elif command -v yum >/dev/null 2>&1; then
        run_as_root yum install -y python3-devel
    elif command -v pacman >/dev/null 2>&1; then
        run_as_root pacman -Sy --needed --noconfirm python
    elif command -v brew >/dev/null 2>&1; then
        brew install python@3.11
    else
        echo "[fetch-scopemux-core] Unable to install Python development files automatically; install them and rerun this script" >&2
        exit 1
    fi
}

force=false
check_only=false
for arg in "$@"; do
    case "$arg" in
        --force) force=true ;;
        --check) check_only=true ;;
        *)
            echo "[fetch-scopemux-core] unknown argument: ${arg}" >&2
            exit 2
            ;;
    esac
done

current_rev=""
if [ -d "${DEST}/.git" ]; then
    current_rev="$(git -C "${DEST}" rev-parse HEAD 2>/dev/null || true)"
fi

if [ "${check_only}" = true ]; then
    if [ -z "${current_rev}" ]; then
        echo "[fetch-scopemux-core] ${DEST} is missing (expected ${PINNED_REV})" >&2
        exit 1
    fi
    if [ "${current_rev}" != "${PINNED_REV}" ]; then
        echo "[fetch-scopemux-core] ${DEST} is at ${current_rev}, expected ${PINNED_REV}" >&2
        exit 1
    fi
    echo "[fetch-scopemux-core] ${DEST} is at the pinned revision ${PINNED_REV}"
    exit 0
fi

ensure_cmake
ensure_python_dev

if [ -d "${DEST}/.git" ]; then
    if [ "${current_rev}" = "${PINNED_REV}" ] && [ "${force}" != true ]; then
        echo "[fetch-scopemux-core] ${DEST} already at pinned revision ${PINNED_REV}; ensuring submodules"
    else
        if [ "${current_rev}" != "${PINNED_REV}" ]; then
            echo "[fetch-scopemux-core] Refreshing ${DEST} from ${current_rev:-unknown} to ${PINNED_REV}"
        else
            echo "[fetch-scopemux-core] Refreshing ${DEST}"
        fi
        git -C "${DEST}" fetch --all --tags
    fi
else
    echo "[fetch-scopemux-core] Cloning ${REPO_URL} into ${DEST}"
    mkdir -p "$(dirname "${DEST}")"
    git clone "${REPO_URL}" "${DEST}"
fi

echo "[fetch-scopemux-core] Checking out pinned revision ${PINNED_REV}"
if ! git -C "${DEST}" checkout --quiet "${PINNED_REV}"; then
    echo "[fetch-scopemux-core] failed to check out ${PINNED_REV}; is the pin reachable from ${REPO_URL}?" >&2
    exit 1
fi

echo "[fetch-scopemux-core] Initializing submodules"
git -C "${DEST}" \
    -c 'url.git@github.com:.insteadOf=https://github.com/' \
    submodule update --init --recursive

echo "[fetch-scopemux-core] Done. Build with: cargo build -p opencode-scopemux --features native"
