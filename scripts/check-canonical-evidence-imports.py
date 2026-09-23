#!/usr/bin/env python3
"""Fail early when behavioral evidence and its certification allowance drift."""

import json
import re
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
INVENTORY = ROOT / "tests/architecture/canonical-evidence-imports.json"
EVIDENCE = ROOT / "src/engine/tests"
IMPORT = re.compile(r"(?m)^[ \t]*use[ \t]+mech_core(?:::snapshot)?::[^;]*;")
GROUP = re.compile(r"usemech_core(?P<snapshot>::snapshot)?::\{(?P<items>[^}]*)\};\Z", re.S)


def main() -> None:
    expected = json.loads(INVENTORY.read_text())
    errors = []
    for filename, modules in expected.items():
        source = (EVIDENCE / filename).read_text()
        actual = {"root": set(), "snapshot": set()}
        for match in IMPORT.finditer(source):
            compact = re.sub(r"\s+", "", match.group())
            parsed = GROUP.fullmatch(compact)
            if parsed is None:
                errors.append(f"{filename}: unsupported mech_core import: {compact}")
                continue
            module = "snapshot" if parsed.group("snapshot") else "root"
            actual[module].update(filter(None, parsed.group("items").split(",")))
        for module in ("root", "snapshot"):
            allowed = set(modules[module])
            missing = sorted(actual[module] - allowed)
            stale = sorted(allowed - actual[module])
            if missing or stale:
                errors.append(
                    f"{filename} {module}: missing allowance {missing}; stale allowance {stale}"
                )
    if errors:
        raise SystemExit("\n".join(errors))
    print("canonical evidence imports match certification allowance")


if __name__ == "__main__":
    main()
