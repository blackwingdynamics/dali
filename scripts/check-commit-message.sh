#!/bin/sh

set -eu

commit_message_file=$1
subject=$(sed -n '1p' "$commit_message_file")

case "$subject" in
    feat\(*\):\ *|fix\(*\):\ *|test\(*\):\ *|docs\(*\):\ *|refactor\(*\):\ *|build\(*\):\ *|chore\(*\):\ *|perf\(*\):\ *)
        exit 0
        ;;
    feat:\ *|fix:\ *|test:\ *|docs:\ *|refactor:\ *|build:\ *|chore:\ *|perf:\ *)
        exit 0
        ;;
    *)
        echo "Commit rejected: use Conventional Commits, for example:" >&2
        echo "  feat(loader): validate AMRN payload bounds" >&2
        exit 1
        ;;
esac
