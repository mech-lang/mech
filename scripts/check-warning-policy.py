#!/usr/bin/env python3
"""Reject warning suppressions and unexpected runtime unsafe boundaries."""

from pathlib import Path
import re
import sys


ROOT = Path(__file__).resolve().parents[1]
ATTRIBUTE = re.compile(r"#!?\s*\[[^\]]*\b(?:allow|expect)\s*\(", re.DOTALL)
DEPRECATED_ATTRIBUTE = re.compile(r"#!?\s*\[[^\]]*\bdeprecated\b", re.DOTALL)
IGNORED_BINDING = re.compile(r"\blet\s+_\s*=")
UNSUPPORTED_DYLIB = re.compile(r'^\s*crate-type\s*=\s*\[[^\]]*"dylib"', re.MULTILINE)
UNSAFE_BOUNDARY = re.compile(r"\bunsafe\s+(?:fn|impl|trait|extern)\b|\bunsafe\s*\{")
EXPECTED_RUNTIME_UNSAFE = {
    (
        Path("src/runtime/src/runtime/program/external/coordinator.rs"),
        "unsafe impl ResidentExternalPublicationAuthority for "
        "RuntimeResidentPublicationAuthority {}",
    ),
}


def fail(message: str) -> None:
    print(f"warning policy failed: {message}", file=sys.stderr)
    raise SystemExit(1)


rust_sources = sorted(
    path
    for path in ROOT.rglob("*.rs")
    if "target" not in path.parts and ".git" not in path.parts
)
for path in rust_sources:
    source = path.read_text(encoding="utf-8")
    if ATTRIBUTE.search(source):
        fail(f"warning suppression remains in {path.relative_to(ROOT)}")
    if DEPRECATED_ATTRIBUTE.search(source):
        fail(f"deprecated Rust surface remains in {path.relative_to(ROOT)}")
    if IGNORED_BINDING.search(source):
        fail(f"ignored-result binding remains in {path.relative_to(ROOT)}")

for path in sorted(ROOT.rglob("Cargo.toml")):
    if "target" in path.parts or ".git" in path.parts:
        continue
    manifest = path.read_text(encoding="utf-8")
    if UNSUPPORTED_DYLIB.search(manifest):
        fail(
            f"target-agnostic Rust dylib crate type remains in {path.relative_to(ROOT)}; "
            "it warns on WASM builds"
        )

runtime_unsafe = set()
for path in (ROOT / "src/runtime/src").rglob("*.rs"):
    source = path.read_text(encoding="utf-8")
    for match in UNSAFE_BOUNDARY.finditer(source):
        line = source.count("\n", 0, match.start()) + 1
        statement = source[match.start() : source.find("\n", match.start())].strip()
        runtime_unsafe.add((path.relative_to(ROOT), statement))
        if (path.relative_to(ROOT), statement) not in EXPECTED_RUNTIME_UNSAFE:
            fail(f"unexpected runtime unsafe boundary in {path.relative_to(ROOT)}:{line}")

if runtime_unsafe != EXPECTED_RUNTIME_UNSAFE:
    fail("the reviewed resident publication authority boundary is missing or changed")

config = (ROOT / ".cargo/config.toml").read_text(encoding="utf-8")
if 'rustflags = ["-D", "warnings"]' not in config:
    fail("Cargo no longer promotes compiler warnings to build errors")
if 'rustdocflags = ["-D", "warnings"]' not in config:
    fail("Cargo no longer promotes documentation warnings to build errors")

print("warning policy contract passed")
