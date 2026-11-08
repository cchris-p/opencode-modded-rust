#!/usr/bin/env bash
#
# Package the prebuilt native ScopeMux provider artifacts for distribution.
#
# `INFRA-006`: instead of every machine cloning scopemux-core and compiling the
# C provider, CI builds it once and publishes a platform bundle. This script
# produces that bundle from a local checkout. It builds the `scopemux-core` C
# library and the Tree-sitter grammar archives through the crate's own build
# (`cargo build -p opencode-scopemux --features native`), so it exercises exactly
# the same CMake invocation `build.rs` uses, then stages the link inputs.
#
# Bundle layout (`opencode-native-<os>-<arch>-<pinshort>/`):
#   manifest.json        bundle metadata (scopemux pin, platform, toolchain)
#   lib/libparser_core.a
#   lib/libtree-sitter-<grammar>.a   (c, cpp, python, javascript, typescript, rust)
#   queries/...          Tree-sitter query files baked into the binary at build
#
# Usage:
#   scripts/package-native-artifacts.sh [--out DIR] [--no-fetch] [--keep-build]
#
# Output: prints the path to the produced `.tar.gz` (and a `.sha256`).
#
# Env:
#   SCOPEMUX_CORE_DIR   source checkout to build (default third_party/scopemux-core)
#   NATIVE_DIST_DIR     where the bundle is written (default <root>/native-dist)

set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FETCH="$PROJECT_ROOT/scripts/fetch-scopemux-core.sh"

OUT_DIR="${NATIVE_DIST_DIR:-$PROJECT_ROOT/native-dist}"
DO_FETCH=1

while [ "$#" -gt 0 ]; do
    case "$1" in
        --out) shift; [ "$#" -gt 0 ] || { echo "package-native-artifacts: --out needs a dir" >&2; exit 2; }; OUT_DIR="$1" ;;
        --no-fetch) DO_FETCH=0 ;;
        --keep-build) KEEP_BUILD=1 ;;
        -h|--help) sed -n '2,28p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) echo "package-native-artifacts: unknown argument: $1" >&2; exit 2 ;;
    esac
    shift
done

pin_rev() {
    sed -n 's/^PINNED_REV="\([0-9a-f][0-9a-f]*\)".*/\1/p' "$FETCH" | head -n 1
}

detect_os() {
    case "$(uname -s)" in
        Linux) printf 'linux' ;;
        Darwin) printf 'macos' ;;
        *) echo "package-native-artifacts: unsupported OS $(uname -s)" >&2; return 1 ;;
    esac
}

detect_arch() {
    case "$(uname -m)" in
        x86_64|amd64) printf 'x86_64' ;;
        arm64|aarch64) printf 'aarch64' ;;
        *) echo "package-native-artifacts: unsupported arch $(uname -m)" >&2; return 1 ;;
    esac
}

PIN="$(pin_rev)"
[ -n "$PIN" ] || { echo "package-native-artifacts: could not read PINNED_REV from $FETCH" >&2; exit 1; }
PIN_SHORT="$(printf '%s' "$PIN" | cut -c1-12)"
OS="$(detect_os)"
ARCH="$(detect_arch)"
STAGE_NAME="opencode-native-${OS}-${ARCH}-${PIN_SHORT}"

if [ "$DO_FETCH" = "1" ]; then
    echo "[package-native-artifacts] fetching pinned scopemux-core ($PIN_SHORT)"
    bash "$FETCH"
fi

source_dir="${SCOPEMUX_CORE_DIR:-$PROJECT_ROOT/third_party/scopemux-core}"
if [ ! -f "$source_dir/CMakeLists.txt" ]; then
    echo "[package-native-artifacts] scopemux-core sources not found at $source_dir" >&2
    exit 1
fi

echo "[package-native-artifacts] building native artifacts via cargo (opencode-scopemux --features native)"
(
    cd "$PROJECT_ROOT"
    SCOPEMUX_CORE_DIR="$source_dir" cargo build -p opencode-scopemux --features native
)

# Locate the CMake build tree build.rs produced. The directory name embeds a hash
# of the source path; with a single source checkout there is exactly one. Cargo
# may write to `target/` (CI, normal checkout) or to the shared worktree cache
# (`.shared-target`, when `.cargo/config.toml` redirects it), so search the
# candidate roots in priority order.
find_build_tree() {
    local roots=()
    [ -n "${CARGO_TARGET_DIR:-}" ] && roots+=("$CARGO_TARGET_DIR")
    roots+=("$PROJECT_ROOT/target")
    roots+=("${OPENCODE_WORKTREE_ROOT:-$HOME/worktrees}/opencode-modded-rust/.shared-target")
    local root tree
    for root in "${roots[@]}"; do
        [ -d "$root" ] || continue
        tree="$(find "$root" -type d -name 'scopemux-core-build-*' 2>/dev/null | sort | tail -n 1)"
        if [ -n "$tree" ]; then
            printf '%s' "$tree"
            return 0
        fi
    done
    return 1
}

build_tree="$(find_build_tree || true)"
if [ -z "$build_tree" ] || [ ! -d "$build_tree" ]; then
    echo "[package-native-artifacts] could not locate the scopemux-core build tree under target/" >&2
    exit 1
fi
echo "[package-native-artifacts] build tree: $build_tree"

stage="$OUT_DIR/$STAGE_NAME"
rm -rf "$stage"
mkdir -p "$stage/lib" "$stage/queries"

cp "$build_tree/core/libparser_core.a" "$stage/lib/"

missing=""
for grammar in c cpp python javascript typescript rust; do
    lib="$build_tree/tree-sitter-libs/libtree-sitter-${grammar}.a"
    if [ -f "$lib" ]; then
        cp "$lib" "$stage/lib/"
    else
        missing="$missing tree-sitter-${grammar}"
    fi
done
if [ -n "$missing" ]; then
    echo "[package-native-artifacts] missing grammar archives:$missing" >&2
    exit 1
fi

if [ ! -d "$source_dir/queries" ]; then
    echo "[package-native-artifacts] source has no queries/ directory at $source_dir" >&2
    exit 1
fi
cp -R "$source_dir/queries/." "$stage/queries/"

rustc_version="$(rustc --version 2>/dev/null || echo unknown)"
cat > "$stage/manifest.json" <<JSON
{
  "scopemux_pin": "$PIN",
  "os": "$OS",
  "arch": "$ARCH",
  "created": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "rustc": "$rustc_version",
  "layout": "lib/ + queries/ + manifest.json"
}
JSON

tarball="$OUT_DIR/$STAGE_NAME.tar.gz"
rm -f "$tarball" "$tarball.sha256"
tar -C "$OUT_DIR" -czf "$tarball" "$STAGE_NAME"
if command -v shasum >/dev/null 2>&1; then
    ( cd "$OUT_DIR" && shasum -a 256 "$STAGE_NAME.tar.gz" > "$STAGE_NAME.tar.gz.sha256" )
elif command -v sha256sum >/dev/null 2>&1; then
    ( cd "$OUT_DIR" && sha256sum "$STAGE_NAME.tar.gz" > "$STAGE_NAME.tar.gz.sha256" )
fi

echo "[package-native-artifacts] wrote $tarball"
[ -f "$tarball.sha256" ] && echo "[package-native-artifacts] wrote $tarball.sha256"
