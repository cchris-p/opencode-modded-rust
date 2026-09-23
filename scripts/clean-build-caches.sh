#!/bin/sh
# Report and reclaim disk used by duplicated Cargo build artifacts.
#
# By default this is a dry run: it prints the size of every per-worktree target/
# directory. Pass --yes to actually delete. The shared worktree target dir and
# the main checkout's target/ are only touched with their explicit flags.
#
# Usage:
#   scripts/clean-build-caches.sh [--yes] [--shared] [--include-main]
#
# Options:
#   --yes           Delete instead of only reporting.
#   --shared        Also delete the shared worktree target dir.
#   --include-main  Also delete the main checkout's target/.
#   -h, --help      Show this help.
set -eu

dry_run=1
clean_shared=0
include_main=0

while [ $# -gt 0 ]; do
  case "$1" in
    --yes) dry_run=0 ;;
    --shared) clean_shared=1 ;;
    --include-main) include_main=1 ;;
    -h|--help)
      sed -n '2,15p' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      printf 'clean-build-caches: unknown option: %s\n' "$1" >&2
      exit 2
      ;;
  esac
  shift
done

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
git_common_dir=$(git -C "$script_dir/.." rev-parse --path-format=absolute --git-common-dir 2>/dev/null \
  || git -C "$script_dir/.." rev-parse --git-common-dir)
main_root=$(dirname "$git_common_dir")
repo_name=$(basename "$main_root")
worktree_root="${OPENCODE_WORKTREE_ROOT:-$HOME/worktrees}"
shared_dir="${OPENCODE_RUST_SHARED_TARGET_DIR:-$worktree_root/$repo_name/.shared-target}"

size_kb() { du -sk "$1" 2>/dev/null | awk '{print $1+0}'; }
human() {
  awk -v kb="$1" 'BEGIN { split("KB MB GB TB", u, " "); i=1; while (kb>=1024 && i<4){kb/=1024;i++} printf "%.1f %s", kb, u[i] }'
}

removed_kb=0
remove_or_report() {
  label=$1
  path=$2
  [ -e "$path" ] || return 0
  kb=$(size_kb "$path")
  if [ "$dry_run" -eq 0 ]; then
    rm -rf "$path"
    removed_kb=$((removed_kb + kb))
    printf '  removed  %8s  %s (%s)\n' "$(human "$kb")" "$path" "$label"
  else
    printf '  %8s  %s (%s)\n' "$(human "$kb")" "$path" "$label"
  fi
}

if [ "$dry_run" -eq 1 ]; then
  printf 'DRY RUN for %s: pass --yes to delete.\n\n' "$repo_name"
fi

printf 'Duplicated worktree target dirs:\n'
tmp=$(mktemp "${TMPDIR:-/tmp}/clean-build-caches.XXXXXX")
trap 'rm -f "$tmp"' EXIT
git -C "$main_root" worktree list --porcelain | awk '/^worktree /{print substr($0, 10)}' >"$tmp"
while IFS= read -r wt; do
  [ -n "$wt" ] || continue
  if [ "$wt" = "$main_root" ]; then
    continue
  fi
  target="$wt/target"
  [ -e "$target" ] || continue
  target_phys=$(cd "$target" 2>/dev/null && pwd -P || printf '%s' "$target")
  shared_phys=$(cd "$shared_dir" 2>/dev/null && pwd -P || printf '%s' "$shared_dir")
  if [ "$target_phys" = "$shared_phys" ]; then
    printf '  (shared)            %s\n' "$target"
    continue
  fi
  remove_or_report "worktree duplicate" "$target"
done <"$tmp"

if [ "$clean_shared" -eq 1 ]; then
  printf '\nShared worktree target dir:\n'
  remove_or_report "shared worktree target" "$shared_dir"
fi

if [ "$include_main" -eq 1 ]; then
  printf '\nMain checkout target:\n'
  remove_or_report "main checkout target" "$main_root/target"
fi

if [ "$dry_run" -eq 0 ]; then
  printf '\nReclaimed %s.\n' "$(human "$removed_kb")"
else
  printf '\nShared worktree target dir: %s\n' "$shared_dir"
  printf 'Add --shared to clear it, or --include-main for the main checkout.\n'
fi
