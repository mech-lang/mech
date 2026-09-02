#!/usr/bin/env python3
"""Keep the R1 compatibility closures retired from active product surfaces."""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]

RETIRED_PATHS = (
    "machines/math/src/logarithm/ilogb.rs",
    "machines/math/src/trig/hypot.rs",
    "machines/math/src/trig/sincos.rs",
    "machines/math/docs/log/ilogb.mec",
    "machines/math/docs/log/logb.mec",
    "machines/math/docs/trig/hypot.mec",
    "machines/math/docs/trig/sincos.mec",
)

ARTIFACT_COMPLETENESS_PROOFS = (
    (
        "scripts/check-r1-artifact-closure.py",
        (
            '"standard-native"',
            '"nbody"',
            '"particles"',
            '"ekf"',
        ),
    ),
    (
        ".github/workflows/ci-full.yml",
        (
            "r1-artifact-closure",
            "scripts/check-r1-artifact-closure.py ${{ matrix.representative }}",
        ),
    ),
    (
        "src/wasm/src/project.rs",
        (
            "ekf_scene_advances_on_every_resident_timer_packet",
            "prepared.coordinator.contracts().get(node.contract)",
        ),
    ),
    (
        "tests/fixtures/d2-contract-generator/src/main.rs",
        (
            "every n-body operation must carry a declared contract",
            "ResolvedOperationContract::Declared",
        ),
    ),
    (
        "hosts/gpu/tests/particle_source.rs",
        (
            "particle_arithmetic_reaches_artifact_with_declared_contracts",
            "artifact.contracts().get(node.contract)",
        ),
    ),
    (
        "src/engine/src/program/compiler_planning.rs",
        (
            "ordinary_mech_sources_emit_equivalent_program_artifacts_in_bytecode_v1",
            "artifact_a.contracts().get(node.contract)",
        ),
    ),
    (
        "tests/mech_build.rs",
        (
            "assert_bytecode_artifact_is_fully_declared",
            "distribution_source_bytecode_native_canary",
        ),
    ),
    (
        "machines/logic/src/not.rs",
        (
            "pub struct NotV",
            "logic_unary_full_write_contract(MatA::REPRESENTATION)",
            "FunctionMatrixRepresentation::MatrixD",
        ),
    ),
    (
        "src/engine/src/resident/numeric.rs",
        (
            "fn bool_vector_not(",
            "one_by_one_boolean_matrix_not_uses_matrix_change_contract",
        ),
    ),
)

RUST_RULES = (
    (
        ("src", "machines", "hosts"),
        re.compile(
            r"\b(?:LegacyOpaqueOperationContract|RuntimeResidentResourceWriteRequest|"
            r"prepare_resident_write|RuntimeOperationIdentity|"
            r"compile_legacy_bytecode_program_artifact|compile_source_frozen_v1|"
            r"compile_frozen_v1_program_product|ImportedOperationContractRow|"
            r"LegacyOperationContractResolver|LegacyContractMetadataUnavailable|"
            r"import_program_artifact_bytecode_v1|import_program_artifact_sections_v1)\b"
        ),
        "retired compatibility symbol",
    ),
    (
        ("src/engine/src/artifact", "src/runtime/src/runtime/program"),
        re.compile(r"\b(?:LegacyCall|FrozenV1)\b"),
        "retired implementation-identity fallback",
    ),
    (
        ("src/engine/src/intrinsics/assign", "machines/set/src"),
        re.compile(r"\b(?:assign_output_alias_policy|set_runtime_contract)\b"),
        "inferred contract metadata",
    ),
    (
        ("src/engine/src/artifact", "src/engine/src/resident", "machines"),
        re.compile(r"(?:\[\s*|vec!\[\s*)\"runtime\""),
        "implementation-identity operation namespace",
    ),
)


def files_under(root: Path, relative_roots: tuple[str, ...], suffix: str):
    for relative in relative_roots:
        path = root / relative
        if path.is_file() and path.suffix == suffix:
            yield path
        elif path.is_dir():
            yield from path.rglob(f"*{suffix}")


def matching_lines(path: Path, pattern: re.Pattern[str]):
    for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if pattern.search(line):
            yield number, line.strip()


def failures(root: Path) -> list[str]:
    root = root.resolve()
    found: list[str] = []

    for relative in RETIRED_PATHS:
        path = root / relative
        populated_directory = path.is_dir() and any(
            item.is_file() for item in path.rglob("*")
        )
        if path.is_file() or populated_directory:
            found.append(f"retired compatibility path exists: {relative}")

    for relative, markers in ARTIFACT_COMPLETENESS_PROOFS:
        path = root / relative
        if not path.is_file():
            found.append(f"artifact-completeness proof is missing: {relative}")
            continue
        source = path.read_text(encoding="utf-8")
        for marker in markers:
            if marker not in source:
                found.append(
                    f"artifact-completeness proof {relative} lost {marker}"
                )

    for roots, pattern, label in RUST_RULES:
        for path in files_under(root, roots, ".rs"):
            relative = path.relative_to(root).as_posix()
            for line, text in matching_lines(path, pattern):
                found.append(f"{relative}:{line}: {label}: {text}")

    math_sources = files_under(root, ("machines/math/src",), ".rs")
    todo = re.compile(r"\b(?:todo|unimplemented)\s*!\s*\(")
    for path in math_sources:
        relative = path.relative_to(root).as_posix()
        for line, text in matching_lines(path, todo):
            found.append(f"{relative}:{line}: advertised compiler placeholder: {text}")

    product_surfaces = (
        "machines/math/Cargo.toml",
        "machines/math/src/catalog.rs",
        "machines/math/src/lib.rs",
        "machines/math/docs/index.mec",
    )
    retired_operations = re.compile(r"\b(?:hypot|ilogb|sincos)\b")
    for relative in product_surfaces:
        path = root / relative
        if not path.is_file():
            continue
        for line, text in matching_lines(path, retired_operations):
            found.append(f"{relative}:{line}: retired operation remains advertised: {text}")

    return found


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=ROOT)
    args = parser.parse_args()
    found = failures(args.root)
    if not found:
        print("R1 compatibility closure contract passed")
        return 0
    print("R1 compatibility closure contract failed:", file=sys.stderr)
    for failure in found:
        print(f"  {failure}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
