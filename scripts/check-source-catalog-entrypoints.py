#!/usr/bin/env python3
"""Require source-executing targets to choose a function catalog explicitly."""

from __future__ import annotations

from pathlib import Path
import re
import sys


REPOSITORY_ROOT = Path(__file__).resolve().parent.parent
SOURCE_ROOTS = (
    "src/runtime/examples",
    "src/runtime/src/bin",
    "src/runtime/benches",
    "examples",
    "src/cli",
    "src/wasm",
)
EXECUTION_PATTERN = re.compile(
    r"\b(?:run_string(?:_with_context)?|run_module(?:_scope)?|"
    r"resolve_and_store_module_source|build_module_from_[A-Za-z0-9_]+)\s*\("
)
SOURCE_CODE_PATTERN = re.compile(r"\bMechSourceCode::String\s*\(")
RUNTIME_CONTEXT_PATTERN = re.compile(
    r"\b(?:MechRuntime|RuntimeBuilder|MechProgram|Interpreter)\b"
)
CATALOG_PATTERNS = (
    re.compile(r"\.function_catalog\s*\("),
    re.compile(r"\bsource_runtime_builder\s*\("),
    re.compile(r"\bMechProgram::with_function_catalog\s*\("),
    re.compile(r"\bInterpreter::with_function_catalog\s*\("),
)


def main() -> int:
    failures: list[str] = []
    for relative_root in SOURCE_ROOTS:
        root = REPOSITORY_ROOT / relative_root
        if not root.exists():
            continue
        for path in sorted(root.rglob("*.rs")):
            source = path.read_text(encoding="utf-8")
            executes_source = EXECUTION_PATTERN.search(source)
            constructs_source = SOURCE_CODE_PATTERN.search(source)
            has_runtime_context = RUNTIME_CONTEXT_PATTERN.search(source)
            # Parser and formatter paths may construct MechSourceCode without
            # evaluating it. A constructed source is an entry point only when
            # the same target also owns a runtime or program.
            has_source_entrypoint = executes_source or (
                constructs_source and has_runtime_context
            )
            if has_source_entrypoint and not any(
                pattern.search(source) for pattern in CATALOG_PATTERNS
            ):
                failures.append(
                    f"{path.relative_to(REPOSITORY_ROOT).as_posix()}: "
                    "source execution entry point lacks explicit catalog selection"
                )

    if failures:
        print("Source catalog entry-point contract failed:", file=sys.stderr)
        print(*failures, sep="\n", file=sys.stderr)
        return 1

    print("Source catalog entry-point contract passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
