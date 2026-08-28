#!/usr/bin/env python3
"""Apply conservative formatting to repository Markdown documentation."""

from pathlib import Path
import argparse
import sys


MARKDOWN_SUFFIX = ".md"
GENERATED_CHANGELOG_DIRECTORY = "changelog"
GENERATED_CHANGELOG_NAME = "CHANGELOG.md"
FENCE_MARKERS = ("```", "~~~")


def markdown_files(root: Path) -> list[Path]:
    return sorted(
        path
        for path in root.rglob(f"*{MARKDOWN_SUFFIX}")
        if ".git" not in path.parts
        and "target" not in path.parts
        and path.name != GENERATED_CHANGELOG_NAME
        and GENERATED_CHANGELOG_DIRECTORY not in path.relative_to(root).parts
    )


def format_markdown(source: str) -> str:
    formatted: list[str] = []
    fenced = False
    previous_blank = False

    for raw_line in source.splitlines():
        line = raw_line if fenced else raw_line.rstrip()
        if line.startswith(FENCE_MARKERS):
            fenced = not fenced
            previous_blank = False
            formatted.append(line)
            continue
        if not fenced and not line.strip():
            if previous_blank:
                continue
            previous_blank = True
        else:
            previous_blank = False
        formatted.append(line)

    while formatted and not formatted[-1].strip():
        formatted.pop()
    return "\n".join(formatted) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check",
        action="store_true",
        help="report files that need formatting without modifying them",
    )
    arguments = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    changed: list[Path] = []

    for path in markdown_files(root):
        original = path.read_text(encoding="utf-8")
        formatted = format_markdown(original)
        if original == formatted:
            continue
        changed.append(path)
        if not arguments.check:
            path.write_text(formatted, encoding="utf-8")

    if changed and arguments.check:
        print("Documentation formatting required:", file=sys.stderr)
        print("\n".join(str(path.relative_to(root)) for path in changed), file=sys.stderr)
        return 1
    action = "checked" if arguments.check else "formatted"
    print(f"Documentation formatting {action} ({len(markdown_files(root))} Markdown files).")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
