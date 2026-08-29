#!/usr/bin/env python3
"""Check that optional firmware backends remain isolated by Cargo features."""

from __future__ import annotations

import json
import subprocess
import sys


FIRMWARE_PACKAGE = "dali-firmware"


def firmware_metadata() -> dict:
    result = subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"],
        check=False,
        text=True,
        capture_output=True,
    )
    if result.returncode != 0:
        print(result.stdout, end="")
        print(result.stderr, end="", file=sys.stderr)
        raise SystemExit(result.returncode)
    return json.loads(result.stdout)


def backend_features() -> list[tuple[str, str]]:
    metadata = firmware_metadata()
    firmware = next(
        package for package in metadata["packages"] if package["name"] == FIRMWARE_PACKAGE
    )
    optional_dependencies = {
        dependency["name"]
        for dependency in firmware["dependencies"]
        if dependency["optional"]
    }
    return sorted(
        (feature, value.removeprefix("dep:"))
        for feature, values in firmware["features"].items()
        for value in values
        if value.startswith("dep:") and value.removeprefix("dep:") in optional_dependencies
    )


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
    for feature, package in backend_features():
        if package in unselected:
            print(
                f"{package} is present without the {feature} feature",
                file=sys.stderr,
            )
            return 1
        selected = cargo_tree(feature)
        if package not in selected:
            print(
                f"{package} is missing with the {feature} feature",
                file=sys.stderr,
            )
            return 1

    print("Backend isolation checks passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
