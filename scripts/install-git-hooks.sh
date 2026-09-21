#!/bin/sh
# Install this repository's version-controlled git hooks.
#
# Points core.hooksPath at .githooks so commits automatically run the
# pre-commit formatting hook. Run once per clone; worktrees of the clone share
# the setting.
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
root=$(git -C "$script_dir/.." rev-parse --show-toplevel)

chmod +x "$root/.githooks/pre-commit"
git -C "$root" config core.hooksPath .githooks

echo "Installed git hooks from .githooks (core.hooksPath=.githooks) in $root"
echo "Bypass a hook with: git commit --no-verify"
