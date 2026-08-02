#!/usr/bin/env python3
"""Validate the wheels produced by maturin before they are published.

Two things are easy to break silently and impossible to fix after an upload:

* the PEP 561 marker (``py.typed``) and the hand-written ``_core.pyi`` stub must
  be packaged, since they are the entire typed surface of the native module
  (OMEP-0008);
* the artifact must be a stable-ABI (``abi3``) wheel, otherwise a single build
  no longer covers every supported Python and the matrix is silently wrong
  (OMEP-0009).

Usage: ``check_wheel.py <directory-containing-wheels>``
"""

from __future__ import annotations

import sys
import zipfile
from pathlib import Path

REQUIRED_MEMBERS = ("oxydemark/py.typed", "oxydemark/_core.pyi")


def check(wheel: Path) -> list[str]:
    """Return the list of problems found in a single wheel."""
    errors: list[str] = []
    if "abi3" not in wheel.name:
        errors.append(f"{wheel.name}: not an abi3 wheel")
    with zipfile.ZipFile(wheel) as archive:
        names = set(archive.namelist())
    for member in REQUIRED_MEMBERS:
        if member not in names:
            errors.append(f"{wheel.name}: missing {member}")
    return errors


def main(argv: list[str]) -> int:
    """Check every wheel in the directory given as first argument."""
    if len(argv) != 2:
        print(f"usage: {argv[0]} <dist-directory>", file=sys.stderr)
        return 2

    wheels = sorted(Path(argv[1]).glob("*.whl"))
    if not wheels:
        print(f"::error::no wheel found in {argv[1]}", file=sys.stderr)
        return 1

    errors = [error for wheel in wheels for error in check(wheel)]
    for error in errors:
        print(f"::error::{error}", file=sys.stderr)
    if errors:
        return 1

    for wheel in wheels:
        print(f"OK {wheel.name}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
