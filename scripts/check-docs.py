#!/usr/bin/env python3
"""Validate internal Markdown links from the repository root."""

from pathlib import Path
import re
import sys
from urllib.parse import unquote


LINK_PATTERN = re.compile(r"(?<!!)\[[^\]]*\]\(([^)]+)\)")


def link_target(raw_target: str) -> str:
    target = raw_target.strip()
    if target.startswith("<") and ">" in target:
        return target[1 : target.index(">")]
    return target.split()[0]


def is_external(target: str) -> bool:
    return (
        not target
        or target.startswith("#")
        or target.startswith("/")
        or "://" in target
        or target.startswith("mailto:")
    )


def check_file(path: Path, root: Path) -> list[str]:
    errors: list[str] = []
    text = path.read_text(encoding="utf-8")
    for match in LINK_PATTERN.finditer(text):
        target = link_target(match.group(1))
        if is_external(target):
            continue
        target_path = unquote(target.split("#", 1)[0])
        resolved = (path.parent / target_path).resolve()
        try:
            resolved.relative_to(root)
        except ValueError:
            errors.append(f"{path}: link escapes repository: {target}")
            continue
        if not resolved.exists():
            errors.append(f"{path}: missing link target: {target}")
    return errors


def main() -> int:
    root = Path(__file__).resolve().parents[1]
    markdown_files = sorted(
        path
        for path in root.rglob("*.md")
        if ".git" not in path.parts
        and "target" not in path.parts
        and path.name != "CHANGELOG.md"
        and "changelog" not in path.relative_to(root).parts
    )
    errors = [error for path in markdown_files for error in check_file(path, root)]
    if errors:
        print("Documentation validation failed:", file=sys.stderr)
        print("\n".join(errors), file=sys.stderr)
        return 1
    print(f"Documentation validation passed ({len(markdown_files)} Markdown files).")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
