#!/usr/bin/env python3
"""Enforce the bounded native-build requirement-analysis module layout."""

from __future__ import annotations

from pathlib import Path
import sys


ROOT = Path(__file__).resolve().parent.parent
ANALYSIS = ROOT / "src/build/src/analysis"
REQUIREMENTS = ANALYSIS / "requirements"
EXPECTED = {
    "mod.rs",
    "config.rs",
    "ownership.rs",
    "grants.rs",
    "external.rs",
    "actor.rs",
    "tests.rs",
}
PRODUCTION = EXPECTED - {"tests.rs"}


def nonblank_lines(path: Path) -> int:
    return sum(bool(line.strip()) for line in path.read_text(encoding="utf-8").splitlines())


def main() -> int:
    failures: list[str] = []
    legacy = ANALYSIS / "requirements.rs"
    if legacy.exists():
        failures.append("src/build/src/analysis/requirements.rs must remain absent")

    missing = sorted(name for name in EXPECTED if not (REQUIREMENTS / name).is_file())
    if missing:
        failures.append(f"missing requirement-analysis modules: {', '.join(missing)}")

    if not missing:
        mod_lines = nonblank_lines(REQUIREMENTS / "mod.rs")
        if mod_lines > 300:
            failures.append(f"requirements/mod.rs has {mod_lines} nonblank lines (limit 300)")
        for name in sorted(PRODUCTION - {"mod.rs"}):
            count = nonblank_lines(REQUIREMENTS / name)
            if count > 650:
                failures.append(f"requirements/{name} has {count} nonblank lines (limit 650)")

        orchestration = (REQUIREMENTS / "mod.rs").read_text(encoding="utf-8")
        for marker in (".instructions", "for instruction in", "for (instruction"):
            if marker in orchestration:
                failures.append(
                    f"requirements/mod.rs contains forbidden instruction traversal marker {marker!r}"
                )

        structured_sources = [REQUIREMENTS / name for name in PRODUCTION]
        structured_sources.append(ROOT / "src/build/src/project/render.rs")
        for path in structured_sources:
            text = path.read_text(encoding="utf-8")
            for marker in ("grant.target.split_once", "split_once('/')", 'split_once("/")'):
                if marker in text:
                    failures.append(
                        f"{path.relative_to(ROOT)} reconstructs planned owner identity with {marker!r}"
                    )

    if failures:
        print("Native build analysis layout contract failed:", file=sys.stderr)
        print(*failures, sep="\n", file=sys.stderr)
        return 1

    print("Native build analysis layout contract passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
