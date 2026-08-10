#!/usr/bin/env python3
"""Generate C2's frozen, boundary-only legacy adapter allowance."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

SCRIPT_DIRECTORY = Path(__file__).resolve().parent
if str(SCRIPT_DIRECTORY) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIRECTORY))

import value_system_legacy_scanner_v2 as scanner


ROOT = Path(__file__).resolve().parents[1]
BOUNDARY = ROOT / "src/core/src/legacy_adapter/value.rs"
OUTPUT = ROOT / "tests/architecture/value-system/c2-legacy-adapter-boundary.json"
CATEGORIES = (
    ("legacy-value", "LegacyValue"),
    ("legacy-value-kind", "ValueKind"),
    ("legacy-ref-allocation", "Ref"),
    ("cycle-guard-address", "addr"),
)


def generate(root: Path) -> dict[str, object]:
    boundary = root / BOUNDARY.relative_to(ROOT)
    relative = boundary.relative_to(root).as_posix()
    source = boundary.read_text(encoding="utf-8")
    tokens = scanner.rust_tokens(source)
    corpus = [(relative, source, scanner.mask_non_code(source), tokens)]
    uses = []
    for category, identifier in CATEGORIES:
        grouped = scanner.exact_identifier_uses(corpus, identifier)
        sites = [site for record in grouped for site in record["sites"]]
        uses.append(
            {
                "category": category,
                "identifier": identifier,
                "count": len(sites),
                "sites": sites,
            }
        )
    return {
        "schema_version": 1,
        "scanner": {
            "version": scanner.SCANNER_VERSION,
            "module": "scripts/value_system_legacy_scanner_v2.py",
            "implementation_sha256": scanner.scanner_module_sha256(),
        },
        "boundary": relative,
        "policy": "frozen-boundary-only-uses-may-only-shrink-v1",
        "uses": uses,
    }


def render(payload: dict[str, object]) -> str:
    return json.dumps(payload, indent=2, sort_keys=True) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    root = args.root.resolve()
    output = args.output or root / OUTPUT.relative_to(ROOT)
    expected = render(generate(root))
    if args.check:
        if not output.is_file() or output.read_text(encoding="utf-8") != expected:
            print(f"C2 legacy adapter boundary is stale: {output}", file=sys.stderr)
            return 1
        print("C2 legacy adapter boundary passed")
        return 0
    output.write_text(expected, encoding="utf-8")
    print(output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
