#!/usr/bin/env bash
#
# Ensure (download) the prebuilt native ScopeMux provider artifacts.
#
# `INFRA-006`: rather than cloning scopemux-core and compiling the C provider on
# every machine, download the platform bundle CI published and let `build.rs`
# link it directly (no CMake, no Tree-sitter/pybind11 submodules, no Python).
#
# On success this prints the resolved bundle directory; the caller then exports
# `SCOPEMUX_PREBUILT_DIR=<that dir>` for `cargo build` / `ort-build`.
#
# Usage:
#   scripts/ensure-native-artifacts.sh [--check] [--force] [--tag TAG] [--dir DIR]
#
# Options:
#   --check      report whether a bundle is already cached (no download)
#   --force      re-download even if the cache is present
#   --tag TAG    release tag to pull from (default `native-artifacts`)
#   --dir DIR    cache directory to install into (default below)
#   -h|--help    show this help
#
# Env:
#   SCOPEMUX_PREBUILT_ROOT   cache root (default ~/.cache/opencode-modded-rust/native)
#   SCOPEMUX_PREBUILT_URL    direct tarball URL override (mirror/air-gapped/testing)
#   GITHUB_REPO              source repo (default cchris-p/opencode-modded-rust)
#   GH_TOKEN / GITHUB_TOKEN  auth for the (private) release; `gh` is used if present

set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FETCH="$PROJECT_ROOT/scripts/fetch-scopemux-core.sh"

RELEASE_TAG="${NATIVE_RELEASE_TAG:-native-artifacts}"
REPO="${GITHUB_REPO:-cchris-p/opencode-modded-rust}"
CACHE_ROOT="${SCOPEMUX_PREBUILT_ROOT:-$HOME/.cache/opencode-modded-rust/native}"
CHECK_ONLY=0
FORCE=0
EXPLICIT_DIR=""

while [ "$#" -gt 0 ]; do
    case "$1" in
        --check) CHECK_ONLY=1 ;;
        --force) FORCE=1 ;;
        --tag) shift; [ "$#" -gt 0 ] || { echo "ensure-native-artifacts: --tag needs a value" >&2; exit 2; }; RELEASE_TAG="$1" ;;
        --dir) shift; [ "$#" -gt 0 ] || { echo "ensure-native-artifacts: --dir needs a value" >&2; exit 2; }; EXPLICIT_DIR="$1" ;;
        -h|--help) sed -n '2,26p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) echo "ensure-native-artifacts: unknown argument: $1" >&2; exit 2 ;;
    esac
    shift
done

pin_rev() { sed -n 's/^PINNED_REV="\([0-9a-f][0-9a-f]*\)".*/\1/p' "$FETCH" | head -n 1; }
detect_os() { case "$(uname -s)" in Linux) echo linux ;; Darwin) echo macos ;; *) echo "unsupported OS $(uname -s)" >&2; return 1 ;; esac; }
detect_arch() { case "$(uname -m)" in x86_64|amd64) echo x86_64 ;; arm64|aarch64) echo aarch64 ;; *) echo "unsupported arch $(uname -m)" >&2; return 1 ;; esac; }

PIN="$(pin_rev)"
[ -n "$PIN" ] || { echo "ensure-native-artifacts: could not read PINNED_REV" >&2; exit 1; }
PIN_SHORT="$(printf '%s' "$PIN" | cut -c1-12)"
OS="$(detect_os)"
ARCH="$(detect_arch)"
BUNDLE="opencode-native-${OS}-${ARCH}-${PIN_SHORT}.tar.gz"

target_dir="${EXPLICIT_DIR:-$CACHE_ROOT/${OS}-${ARCH}-${PIN_SHORT}}"
manifest="$target_dir/manifest.json"

if [ -f "$manifest" ] && [ "$FORCE" != "1" ]; then
    if [ "$CHECK_ONLY" = "1" ]; then
        echo "ensure-native-artifacts: cached at $target_dir" >&2
    fi
    printf '%s\n' "$target_dir"
    exit 0
fi

if [ "$CHECK_ONLY" = "1" ]; then
    echo "ensure-native-artifacts: no cached bundle for $BUNDLE" >&2
    exit 1
fi

tmp="$(mktemp -d "${TMPDIR:-/tmp}/ensure-native.XXXXXX")"
trap 'rm -rf "$tmp"' EXIT

url="${SCOPEMUX_PREBUILT_URL:-https://github.com/${REPO}/releases/download/${RELEASE_TAG}/${BUNDLE}}"
echo "ensure-native-artifacts: downloading $BUNDLE" >&2

downloaded=0
if [ -z "${SCOPEMUX_PREBUILT_URL:-}" ] && command -v gh >/dev/null 2>&1 && [ -n "${GH_TOKEN:-${GITHUB_TOKEN:-}}" ]; then
    if gh release download "$RELEASE_TAG" --repo "$REPO" --pattern "$BUNDLE*" --dir "$tmp" --clobber >/dev/null 2>&1; then
        downloaded=1
    fi
fi
if [ "$downloaded" != "1" ]; then
    command -v curl >/dev/null 2>&1 || { echo "ensure-native-artifacts: 'curl' is required to fetch the bundle" >&2; exit 1; }
    auth_header=()
    if [ -n "${GH_TOKEN:-${GITHUB_TOKEN:-}}" ]; then
        auth_header=(-H "Authorization: Bearer ${GH_TOKEN:-${GITHUB_TOKEN:-}}")
    fi
    curl -fSL "${auth_header[@]}" -o "$tmp/$BUNDLE" "$url"
    # Best-effort checksum alongside the tarball.
    curl -fSL "${auth_header[@]}" -o "$tmp/$BUNDLE.sha256" "${url}.sha256" 2>/dev/null || true
fi

if [ ! -s "$tmp/$BUNDLE" ]; then
    echo "ensure-native-artifacts: download failed for $BUNDLE" >&2
    exit 1
fi

# Verify the checksum when the release published one.
sha_file="$tmp/$BUNDLE.sha256"
if [ -f "$sha_file" ]; then
    expected="$(awk '{print $1}' "$sha_file" | head -n1)"
    if [ -n "$expected" ]; then
        if command -v shasum >/dev/null 2>&1; then
            actual="$(shasum -a 256 "$tmp/$BUNDLE" | awk '{print $1}')"
        else
            actual="$(sha256sum "$tmp/$BUNDLE" | awk '{print $1}')"
        fi
        if [ "$actual" != "$expected" ]; then
            echo "ensure-native-artifacts: checksum mismatch for $BUNDLE" >&2
            exit 1
        fi
    fi
fi

rm -rf "$target_dir"
mkdir -p "$target_dir"
tar -C "$target_dir" --strip-components=1 -xzf "$tmp/$BUNDLE"

if [ ! -f "$manifest" ]; then
    echo "ensure-native-artifacts: extracted bundle has no manifest.json" >&2
    exit 1
fi

echo "ensure-native-artifacts: installed to $target_dir" >&2
printf '%s\n' "$target_dir"
