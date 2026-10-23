#!/usr/bin/env bash
#
# Build and publish the `ort` CLI binary as a downloadable release asset.
#
# INFRA-007: one builder (or CI) publishes `opencode-modded-rust-<os>-<arch>`
# once; every other machine installs it with `standards/utilities/ort-install`
# instead of cloning this repo and compiling locally.
#
# The Linux asset must be built on the oldest glibc the fleet runs (Ubuntu 22.04,
# glibc 2.35). A ubuntu-24.04 build requires GLIBC_2.39 and fails on 22.04
# hosts, so this script warns when the build host's glibc is newer.
#
# Usage:
#   scripts/publish-cli-artifact.sh [options]
#
# Options:
#   --tag TAG        release tag to publish (default: native-artifacts)
#   --rev REV        revision recorded in the .rev sidecar (default: git HEAD)
#   --repo OWNER/R   GitHub repo (default: cchris-p/opencode-modded-rust)
#   --out DIR        staging directory (default: dist/cli)
#   --no-build       package an existing target/release binary
#   --no-native      do not ensure the prebuilt native provider bundle
#   --dry-run        package only; do not upload
#   -h, --help       show this help
#
# Env:
#   GH_TOKEN / GITHUB_TOKEN   auth for the (private) release upload
#   ORT_RELEASE_TAG           default tag override
#   ORT_RELEASE_REPO          default repo override
#   SCOPEMUX_PREBUILT_DIR     skip bundle discovery and link this bundle

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TAG="${ORT_RELEASE_TAG:-native-artifacts}"
REPO="${ORT_RELEASE_REPO:-cchris-p/opencode-modded-rust}"
OUT_DIR="$ROOT/dist/cli"
REV=""
DO_BUILD=1
DO_NATIVE=1
DRY_RUN=0

while [ "$#" -gt 0 ]; do
    case "$1" in
        --tag) shift; [ "$#" -gt 0 ] || { echo "publish-cli-artifact: --tag needs a value" >&2; exit 2; }; TAG="$1" ;;
        --rev) shift; [ "$#" -gt 0 ] || { echo "publish-cli-artifact: --rev needs a value" >&2; exit 2; }; REV="$1" ;;
        --repo) shift; [ "$#" -gt 0 ] || { echo "publish-cli-artifact: --repo needs a value" >&2; exit 2; }; REPO="$1" ;;
        --out) shift; [ "$#" -gt 0 ] || { echo "publish-cli-artifact: --out needs a value" >&2; exit 2; }; OUT_DIR="$1" ;;
        --no-build) DO_BUILD=0 ;;
        --no-native) DO_NATIVE=0 ;;
        --dry-run|--no-upload) DRY_RUN=1 ;;
        -h|--help) sed -n '2,34p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) echo "publish-cli-artifact: unknown argument: $1" >&2; exit 2 ;;
    esac
    shift
done

if [ -f "$HOME/.cargo/env" ]; then
    # shellcheck disable=SC1091
    . "$HOME/.cargo/env"
fi
if ! command -v cargo >/dev/null 2>&1; then
    echo "publish-cli-artifact: cargo is not on PATH" >&2
    exit 1
fi

if [ -z "$REV" ]; then
    REV="$(git -C "$ROOT" rev-parse HEAD 2>/dev/null || echo unknown)"
fi

detect_os() { case "$(uname -s)" in Linux) echo linux ;; Darwin) echo macos ;; *) echo "unsupported OS $(uname -s)" >&2; return 1 ;; esac; }
detect_arch() { case "$(uname -m)" in x86_64|amd64) echo x86_64 ;; arm64|aarch64) echo aarch64 ;; *) echo "unsupported arch $(uname -m)" >&2; return 1 ;; esac; }
OS="$(detect_os)"
ARCH="$(detect_arch)"
# CI names the macOS asset `darwin`; keep that convention for install parity.
[ "$OS" = "macos" ] && ASSET_OS="darwin" || ASSET_OS="$OS"
PKG="opencode-modded-rust-${ASSET_OS}-${ARCH}"

if [ "$OS" = "linux" ]; then
    glibc="$(ldd --version 2>/dev/null | awk 'NR==1{print $NF}')"
    echo "publish-cli-artifact: linux glibc=$glibc (fleet minimum is 2.35 on Ubuntu 22.04)"
    if [ -n "$glibc" ] && [ "$glibc" != "2.35" ]; then
        case "$glibc" in
            2.3[0-4]|2.2*|2.1*|2.0*|1.*) ;;
            *) echo "publish-cli-artifact: WARNING: glibc $glibc assets may not run on Ubuntu 22.04 (glibc 2.35)." >&2 ;;
        esac
    fi
fi

if [ "$DO_NATIVE" = "1" ] && [ -z "${SCOPEMUX_PREBUILT_DIR:-}" ] && [ -x "$ROOT/scripts/ensure-native-artifacts.sh" ]; then
    if bundle="$("$ROOT/scripts/ensure-native-artifacts.sh" 2>/dev/null)"; then
        export SCOPEMUX_PREBUILT_DIR="$bundle"
        echo "publish-cli-artifact: using native bundle at $bundle"
    else
        echo "publish-cli-artifact: no prebuilt native bundle; falling back to the source build" >&2
    fi
fi

if [ "$DO_BUILD" = "1" ]; then
    echo "publish-cli-artifact: building release CLI"
    ( cd "$ROOT" && cargo build --release -p opencode-cli )
fi

BINARY="$ROOT/target/release/opencode-modded-rust"
if [ ! -x "$BINARY" ]; then
    echo "publish-cli-artifact: release binary not found at $BINARY" >&2
    exit 1
fi

mkdir -p "$OUT_DIR"
STAGE="$(mktemp -d "${TMPDIR:-/tmp}/publish-cli.XXXXXX")"
trap 'rm -rf "$STAGE"' EXIT
cp "$BINARY" "$STAGE/opencode-modded-rust"
chmod +x "$STAGE/opencode-modded-rust"
tar -C "$STAGE" -czf "$OUT_DIR/${PKG}.tar.gz" opencode-modded-rust

if command -v shasum >/dev/null 2>&1; then
    ( cd "$OUT_DIR" && shasum -a 256 "${PKG}.tar.gz" > "${PKG}.tar.gz.sha256" )
else
    ( cd "$OUT_DIR" && sha256sum "${PKG}.tar.gz" > "${PKG}.tar.gz.sha256" )
fi
printf '%s\n' "$REV" > "$OUT_DIR/${PKG}.tar.gz.rev"

echo "publish-cli-artifact: packaged $PKG"
ls -lh "$OUT_DIR/${PKG}.tar.gz" "$OUT_DIR/${PKG}.tar.gz.sha256" "$OUT_DIR/${PKG}.tar.gz.rev"

if [ "$DRY_RUN" = "1" ]; then
    echo "publish-cli-artifact: dry run; assets left in $OUT_DIR"
    exit 0
fi

if ! command -v gh >/dev/null 2>&1; then
    echo "publish-cli-artifact: 'gh' is required to publish" >&2
    exit 1
fi
if [ -z "${GH_TOKEN:-${GITHUB_TOKEN:-}}" ]; then
    echo "publish-cli-artifact: GH_TOKEN/GITHUB_TOKEN is required to publish a private release" >&2
    exit 1
fi

gh release view "$TAG" --repo "$REPO" >/dev/null 2>&1 || \
    gh release create "$TAG" \
        --repo "$REPO" \
        --title "Native artifacts" \
        --notes "Prebuilt native ScopeMux provider bundles and CLI binaries (INFRA-006/INFRA-007). Assets are keyed by platform and the pinned scopemux-core revision; .rev carries the source commit."

gh release upload "$TAG" \
    "$OUT_DIR/${PKG}.tar.gz" \
    "$OUT_DIR/${PKG}.tar.gz.sha256" \
    "$OUT_DIR/${PKG}.tar.gz.rev" \
    --repo "$REPO" --clobber

echo "publish-cli-artifact: published ${PKG} to $REPO@$TAG (rev $REV)"
