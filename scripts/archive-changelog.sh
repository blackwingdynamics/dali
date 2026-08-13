#!/bin/sh

set -eu

version=${1:?Usage: archive-changelog.sh vX.Y.Z}
archive_dir="docs/changelog"
archive_file="${archive_dir}/${version}.md"
index_file="${archive_dir}/README.md"

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

printf 'Archived changelog: %s\n' "$archive_file"
