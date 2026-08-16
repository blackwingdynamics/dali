#!/bin/sh

set -eu

version=${1:?Usage: archive-changelog.sh vX.Y.Z}
archive_dir="docs/changelog"
archive_file="${archive_dir}/${version}.md"
index_file="${archive_dir}/README.md"
root_file="CHANGELOG.md"

case "$version" in
    v[0-9]*.[0-9]*.[0-9]*) ;;
    *)
        echo "Invalid release tag: $version (expected vX.Y.Z)" >&2
        exit 1
        ;;
esac

mkdir -p "$archive_dir"
git-cliff --tag "$version" --latest --output "$archive_file"

{
    printf '%s\n\n' '# Archived Dali OS Changelogs'
    printf '%s\n\n' 'Release changelogs are generated automatically from Conventional Commits.'
    find "$archive_dir" -maxdepth 1 -type f -name 'v*.md' -print \
        | sort -Vr \
        | while IFS= read -r file; do
            name=$(basename "$file" .md)
            printf -- '- [%s](%s)\n' "$name" "$file"
        done
} > "$index_file"

{
    printf '%s\n\n' '# Changelog'
    printf '%s\n\n' 'All notable changes to Dali OS are documented in this file.'
    printf '%s\n\n' 'The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and released versions follow the rules in [VERSIONING.md](docs/VERSIONING.md). Release entries are generated automatically from Conventional Commits.'
    printf '%s\n\n' '## [Unreleased]'
    printf '%s\n\n' 'No manual entries are maintained here. Changes are collected from commit history and archived when a release tag is published.'
    printf '%s\n\n' '## Releases'
    find "$archive_dir" -maxdepth 1 -type f -name 'v*.md' -print \
        | sort -Vr \
        | while IFS= read -r file; do
            name=$(basename "$file" .md)
            printf -- '- [%s](%s)\n' "$name" "$file"
        done
} > "$root_file"

printf 'Archived changelog: %s\n' "$archive_file"
