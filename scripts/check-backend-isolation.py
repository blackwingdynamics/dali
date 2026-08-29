#!/usr/bin/env python3
"""Check that optional firmware backends remain isolated by Cargo features."""

from __future__ import annotations

import subprocess
import sys


FIRMWARE_PACKAGE = "dali-firmware"
F405_FEATURE = "stm32f405"
F405_PACKAGE = "dali-board-stm32f405"


def cargo_tree(features: str | None) -> str:
    command = ["cargo", "tree", "-p", FIRMWARE_PACKAGE, "--edges", "normal"]
    if features:
        command.extend(["--features", features])
    else:
        command.append("--no-default-features")
    result = subprocess.run(command, check=False, text=True, capture_output=True)
    if result.returncode != 0:
        print(result.stdout, end="")
        print(result.stderr, end="", file=sys.stderr)
        raise SystemExit(result.returncode)
    return result.stdout


def main() -> int:
    unselected = cargo_tree(None)
    if F405_PACKAGE in unselected:
        print(
            f"{F405_PACKAGE} is present without the {F405_FEATURE} feature",
            file=sys.stderr,
        )
        return 1

    selected = cargo_tree(F405_FEATURE)
    if F405_PACKAGE not in selected:
        print(
            f"{F405_PACKAGE} is missing with the {F405_FEATURE} feature",
            file=sys.stderr,
        )
        return 1

    print("Backend isolation check passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
