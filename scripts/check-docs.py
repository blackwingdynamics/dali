#!/usr/bin/env python3
"""Validate internal Markdown links from the repository root."""

from pathlib import Path
import re
import sys
from urllib.parse import unquote


LINK_PATTERN = re.compile(r"(?<!!)\[[^\]]*\]\(([^)]+)\)")
TREE_ENTRY_PATTERN = re.compile(r"^(?P<prefix>(?:│   )*)(?:├──|└──) (?P<item>.*)$")


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


def check_documentation_indexes(root: Path) -> list[str]:
    docs_root = root / "docs"
    errors: list[str] = []
    for directory in sorted(path for path in docs_root.rglob("*") if path.is_dir()):
        markdown_files = tuple(directory.glob("*.md"))
        if markdown_files and not (directory / "README.md").is_file():
            errors.append(f"{directory}: missing README.md index")
    return errors


def check_repository_tree(root: Path) -> list[str]:
    tree_path = root / "docs/file-structure/repository-tree.md"
    lines = tree_path.read_text(encoding="utf-8").splitlines()
    stack: list[str] = []
    errors: list[str] = []
    for line_number, line in enumerate(lines, start=1):
        match = TREE_ENTRY_PATTERN.match(line)
        if not match:
            continue
        depth = len(match.group("prefix")) // 4
        stack = stack[:depth]
        parent = root.joinpath(*stack)
        item = match.group("item").split("#", 1)[0].strip()
        names = [name.strip() for name in item.split(",") if name.strip()]
        for name in names:
            candidate = parent / name.rstrip("/")
            if candidate.parts and candidate.parts[len(root.parts)] == "docs" and not candidate.exists():
                errors.append(
                    f"{tree_path}:{line_number}: missing tree entry: {candidate.relative_to(root)}"
                )
        if item.endswith("/") and len(names) == 1:
            stack.append(names[0].rstrip("/"))
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
    errors.extend(check_documentation_indexes(root))
    errors.extend(check_repository_tree(root))
    if errors:
        print("Documentation validation failed:", file=sys.stderr)
        print("\n".join(errors), file=sys.stderr)
        return 1
    print(f"Documentation validation passed ({len(markdown_files)} Markdown files).")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
