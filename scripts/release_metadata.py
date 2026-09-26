"""Release-channel policy shared by tag validation and binary packaging."""

from __future__ import annotations

import re


VERSION = re.compile(
    r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)"
    r"(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?"
    r"(?:\+([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?"
)


def release_channel(version: str) -> str:
    match = VERSION.fullmatch(version)
    if not match:
        raise ValueError(f"invalid release version: {version!r}")
    prerelease = match.group(4)
    if prerelease and any(part.isdigit() and len(part) > 1 and part[0] == "0"
                          for part in prerelease.split(".")):
        raise ValueError(f"invalid numeric prerelease identifier: {version!r}")
    return "preview" if prerelease else "stable"


def resolve_channel(requested: str, version: str) -> str:
    expected = release_channel(version)
    if requested == "auto":
        return expected
    if requested == "nightly" or requested == expected:
        return requested
    raise ValueError(f"version {version!r} requires {expected!r}, not {requested!r}")
