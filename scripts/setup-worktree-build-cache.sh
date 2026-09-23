#!/bin/sh
# Configure a single shared Cargo target directory for this repository's git
# worktrees.
#
# Every git worktree compiles its own target/ by default, so a full workspace
# build is duplicated per worktree and can exhaust the disk. Cargo discovers
# .cargo/config.toml in parent directories of the build directory, so a managed
# config placed at the worktree root makes every worktree beneath it share one
# target directory while leaving the main checkout's target/ untouched.
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
git_common_dir=$(git -C "$script_dir/.." rev-parse --path-format=absolute --git-common-dir 2>/dev/null \
  || git -C "$script_dir/.." rev-parse --git-common-dir)
main_root=$(dirname "$git_common_dir")
repo_name=$(basename "$main_root")

worktree_root="${OPENCODE_WORKTREE_ROOT:-$HOME/worktrees}"
worktree_repo_root="$worktree_root/$repo_name"
shared_dir="${OPENCODE_RUST_SHARED_TARGET_DIR:-$worktree_repo_root/.shared-target}"
config_dir="$worktree_repo_root/.cargo"
config_file="$config_dir/config.toml"

mkdir -p "$config_dir" "$shared_dir"

cat >"$config_file" <<EOF
# Managed by scripts/setup-worktree-build-cache.sh in $repo_name.
# Cargo reads this from any git worktree under $worktree_repo_root/ and directs
# all build artifacts to one shared target directory instead of a per-worktree
# target/. Re-run the script to refresh it; manual edits are overwritten.
[build]
target-dir = "$shared_dir"
EOF

printf 'Shared worktree build cache configured.\n'
printf '  repo:    %s\n' "$repo_name"
printf '  config:  %s\n' "$config_file"
printf '  target:  %s\n' "$shared_dir"
printf 'Reclaim old per-worktree target/ dirs with: scripts/clean-build-caches.sh --yes\n'
