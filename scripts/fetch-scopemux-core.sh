#!/usr/bin/env bash
#
# Fetch the pinned scopemux-core source for the native FFI provider.
#
# Clones scopemux-core at the recorded revision into third_party/scopemux-core
# (gitignored) and initializes its submodules (Tree-sitter grammars, pybind11),
# which the C build needs.
#
# It also provisions the native build dependencies when they are missing: cmake,
# and a Python 3.10/3.11 interpreter with development headers. scopemux-core's
# CMake requires `find_package(Python 3.10...<3.12)`, so a pyenv or distro
# `python3` newer than 3.11 cannot satisfy it. Set SCOPEMUX_PYTHON to force a
# specific interpreter.
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

# Interpreter candidates acceptable to scopemux-core's `find_package(Python
# 3.10...<3.12)`. An explicit SCOPEMUX_PYTHON wins; otherwise versioned binaries
# come before `python3`, so a pyenv virtualenv or distro `python3` newer than
# 3.11 cannot shadow a compatible interpreter. pyenv version directories are
# included because they carry development headers but are not always reachable
# as a bare `python3.11` on PATH.
python_candidates() {
    if [ -n "${SCOPEMUX_PYTHON:-}" ]; then
        printf '%s\n' "${SCOPEMUX_PYTHON}"
    fi
    printf '%s\n' python3.11 python3.10 python3

    local versions_dir="${PYENV_ROOT:-${HOME:-}/.pyenv}/versions"
    if [ -d "${versions_dir}" ]; then
        local dir
        for dir in "${versions_dir}"/3.11.* "${versions_dir}"/3.10.*; do
            [ -x "${dir}/bin/python3" ] && printf '%s\n' "${dir}/bin/python3"
        done
    fi
}

python_dev_ok() {
    "${1}" - <<'PY' >/dev/null 2>&1
import glob
import os
import sys
import sysconfig

if not ((3, 10) <= sys.version_info[:2] < (3, 12)):
    raise SystemExit(1)

include = sysconfig.get_path("include")
if not include or not os.path.exists(os.path.join(include, "Python.h")):
    raise SystemExit(1)

libdir = sysconfig.get_config_var("LIBDIR")
if libdir and not glob.glob(os.path.join(libdir, "libpython*")):
    raise SystemExit(1)
PY
}

have_python_dev() {
    local py
    while IFS= read -r py; do
        [ -n "${py}" ] || continue
        if python_dev_ok "${py}"; then
            return 0
        fi
    done < <(python_candidates)
    return 1
}

ensure_python_dev() {
    if have_python_dev; then
        return
    fi

    echo "[fetch-scopemux-core] no Python 3.10/3.11 development files found; installing native build dependency"

    if command -v apt-get >/dev/null 2>&1; then
        run_as_root apt-get update || true
        local pkg
        for pkg in python3.11-dev python3.10-dev; do
            if run_as_root apt-get install -y "${pkg}" 2>/dev/null; then
                break
            fi
        done
    elif command -v dnf >/dev/null 2>&1; then
        local pkg
        for pkg in python3.11-devel python3.10-devel; do
            if run_as_root dnf install -y "${pkg}" 2>/dev/null; then
                break
            fi
        done
    elif command -v yum >/dev/null 2>&1; then
        local pkg
        for pkg in python3.11-devel python3.10-devel; do
            if run_as_root yum install -y "${pkg}" 2>/dev/null; then
                break
            fi
        done
    elif command -v pacman >/dev/null 2>&1; then
        run_as_root pacman -Sy --needed --noconfirm python
    elif command -v brew >/dev/null 2>&1; then
        local formula
        for formula in python@3.11 python@3.10; do
            if brew install "${formula}" 2>/dev/null; then
                break
            fi
        done
    else
        echo "[fetch-scopemux-core] Unable to install Python development files automatically; install them and rerun this script" >&2
        exit 1
    fi

    if ! have_python_dev; then
        echo "[fetch-scopemux-core] could not provision a Python 3.10/3.11 interpreter with development files." >&2
        echo "[fetch-scopemux-core] Install one (for example python3.11-dev), or point SCOPEMUX_PYTHON at a compatible interpreter, then rerun." >&2
        exit 1
    fi
}

# Submodules the native C build compiles from source. Keep in sync with
# REQUIRED_SUBMODULES in crates/opencode-scopemux/build.rs.
required_submodules() {
    printf '%s\n' \
        vendor/tree-sitter \
        vendor/tree-sitter-c \
        vendor/tree-sitter-cpp \
        vendor/tree-sitter-python \
        vendor/tree-sitter-javascript \
        vendor/tree-sitter-typescript \
        vendor/tree-sitter-rust \
        vendor/pybind11
}

# Print the required submodule paths that are missing or empty.
missing_required_submodules() {
    local rel
    while IFS= read -r rel; do
        [ -n "${rel}" ] || continue
        if [ ! -d "${DEST}/${rel}" ] || [ -z "$(ls -A "${DEST}/${rel}" 2>/dev/null)" ]; then
            printf '%s\n' "${rel}"
        fi
    done < <(required_submodules)
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
    check_missing="$(missing_required_submodules || true)"
    if [ -n "${check_missing}" ]; then
        echo "[fetch-scopemux-core] ${DEST} is missing submodule sources:" >&2
        printf '  %s\n' ${check_missing} >&2
        echo "[fetch-scopemux-core] run without --check to initialize them" >&2
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

update_missing="$(missing_required_submodules || true)"
if [ -n "${update_missing}" ]; then
    echo "[fetch-scopemux-core] required submodules are still empty after update:" >&2
    printf '  %s\n' ${update_missing} >&2
    echo "[fetch-scopemux-core] check network access to github.com and rerun" >&2
    exit 1
fi

echo "[fetch-scopemux-core] Done. Build with: cargo build -p opencode-scopemux --features native"
