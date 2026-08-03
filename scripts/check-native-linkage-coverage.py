#!/usr/bin/env python3
"""Validate and report Phase 1 native runtime-factory linkage coverage."""

from __future__ import annotations

import json
from hashlib import sha256
import os
from pathlib import Path
import subprocess
import sys
from typing import Any


REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
FIXTURE_MANIFEST = REPOSITORY_ROOT / "tests/fixtures/native-linkage/Cargo.toml"
FROZEN_SURFACE = (
    REPOSITORY_ROOT
    / "tests/architecture/function-system/runtime-factory-surface.json"
)
REPORT_PATH = (
    REPOSITORY_ROOT
    / "tests/architecture/native-linkage/phase1-coverage.json"
)
EXPECTED_STANDARD_COUNT = 9_019
EXPECTED_STANDARD_SURFACE_SHA256 = (
    "b9db9003bb9da704d5b61a5a6a3d5fcc6438ef7e433f49fc1918c466fc2fcc62"
)
VARIADIC_ENTRY = "HorizontalConcatenateRDN<f64>"
EXACT_INSTALLER_FEATURES = (
    "installer-variable-define-f64",
    "installer-horizontal-concatenate-rdn-f64",
    "installer-horizontal-concatenate-s2-f64",
    "installer-vertical-concatenate-n-args-f64",
    "installer-vertical-concatenate-r2-r2-f64",
    "installer-add-ss-f64",
    "installer-add-m2m2-f64",
    "installer-add-mdmd-f64",
)
OWNER_MANIFESTS = {
    "mech-engine": REPOSITORY_ROOT / "src/engine/Cargo.toml",
    "mech-math": REPOSITORY_ROOT / "machines/math/Cargo.toml",
    "mech-compare": REPOSITORY_ROOT / "machines/compare/Cargo.toml",
    "mech-logic": REPOSITORY_ROOT / "machines/logic/Cargo.toml",
    "mech-range": REPOSITORY_ROOT / "machines/range/Cargo.toml",
    "mech-matrix": REPOSITORY_ROOT / "machines/matrix/Cargo.toml",
    "mech-set": REPOSITORY_ROOT / "machines/set/Cargo.toml",
    "mech-string": REPOSITORY_ROOT / "machines/string/Cargo.toml",
    "mech-stats": REPOSITORY_ROOT / "machines/stats/Cargo.toml",
    "mech-combinatorics": REPOSITORY_ROOT / "machines/combinatorics/Cargo.toml",
}

EXPECTED_REPRESENTATIVES: dict[str, dict[str, Any]] = {
    "VariableDefineF64": {
        "id_hex": "0023b6ad86b655e7",
        "package": "mech-engine",
        "crate_name": "mech_engine",
        "installer_path": "mech_engine::__mech_native::install_variable_define_f64",
        "cargo_features": [
            "bool",
            "f64",
            "native-link",
            "runtime",
            "string",
            "variable_define",
        ],
    },
    "AddSS<f64>": {
        "id_hex": "000a2c77688486f3",
        "package": "mech-math",
        "crate_name": "mech_math",
        "installer_path": "mech_math::__mech_native::install_add_ss_f64",
        "cargo_features": ["add", "f64", "native-link", "runtime"],
    },
    "AddM2M2<f64>": {
        "id_hex": "00eb049b7b90a0d9",
        "package": "mech-math",
        "crate_name": "mech_math",
        "installer_path": "mech_math::__mech_native::install_add_m2m2_f64",
        "cargo_features": [
            "add",
            "f64",
            "matrix2",
            "native-link",
            "runtime",
        ],
    },
    "AddMDMD<f64>": {
        "id_hex": "008fa755537dc395",
        "package": "mech-math",
        "crate_name": "mech_math",
        "installer_path": "mech_math::__mech_native::install_add_mdmd_f64",
        "cargo_features": [
            "add",
            "f64",
            "matrixd",
            "native-link",
            "runtime",
        ],
    },
    VARIADIC_ENTRY: {
        "id_hex": "006c13aeb8d21f6c",
        "package": "mech-engine",
        "crate_name": "mech_engine",
        "installer_path": (
            "mech_engine::__mech_native::install_horizontal_concatenate_rdn_f64"
        ),
        "cargo_features": [
            "bool",
            "f64",
            "matrix_horzcat",
            "matrixd",
            "native-link",
            "row_vectord",
            "runtime",
            "vectord",
        ],
    },
    "HorizontalConcatenateS2<f64>": {
        "id_hex": "00c3ae9efc75d589",
        "package": "mech-engine",
        "crate_name": "mech_engine",
        "installer_path": (
            "mech_engine::__mech_native::install_horizontal_concatenate_s2_f64"
        ),
        "cargo_features": [
            "bool",
            "f64",
            "matrix_horzcat",
            "native-link",
            "row_vector2",
            "runtime",
            "vector2",
        ],
    },
    "VerticalConcatenateR2R2<f64Matrix2RowVector2RowVector2>": {
        "id_hex": "00d7d04069950a49",
        "package": "mech-engine",
        "crate_name": "mech_engine",
        "installer_path": (
            "mech_engine::__mech_native::install_vertical_concatenate_r2_r2_f64"
        ),
        "cargo_features": [
            "bool",
            "f64",
            "matrix2",
            "matrix_vertcat",
            "native-link",
            "row_vector2",
            "runtime",
            "vector2",
        ],
    },
    "VerticalConcatenateNArgs<f64>": {
        "id_hex": "006e5ef927b76ce2",
        "package": "mech-engine",
        "crate_name": "mech_engine",
        "installer_path": (
            "mech_engine::__mech_native::install_vertical_concatenate_n_args_f64"
        ),
        "cargo_features": [
            "bool",
            "f64",
            "matrix_vertcat",
            "matrixd",
            "native-link",
            "row_vectord",
            "runtime",
            "vectord",
        ],
    },
}


class ContractError(RuntimeError):
    pass


def run(
    command: list[str],
    *,
    capture: bool = False,
    standalone_fixture: bool = False,
) -> str:
    environment = os.environ.copy()
    environment.setdefault("CARGO_INCREMENTAL", "0")
    target_base = Path(
        environment.get("CARGO_TARGET_DIR", REPOSITORY_ROOT / "target")
    )
    if standalone_fixture:
        # Cargo target artifacts are not safely interchangeable between the
        # root workspace and this standalone fixture workspace: their path
        # package SourceIds differ even though both resolve to this checkout.
        # Keep each workspace cached, but never let one reuse the other's
        # exported mech-core types.
        environment["CARGO_TARGET_DIR"] = str(
            target_base / "native-linkage-fixture"
        )
    else:
        environment["CARGO_TARGET_DIR"] = str(target_base)
    completed = subprocess.run(
        command,
        cwd=REPOSITORY_ROOT,
        env=environment,
        check=False,
        text=True,
        stdout=subprocess.PIPE if capture else None,
    )
    if completed.returncode != 0:
        raise ContractError(f"command failed ({completed.returncode}): {' '.join(command)}")
    return completed.stdout if capture else ""


def cargo_fixture(*arguments: str, capture: bool = False) -> str:
    return run(
        [
            "cargo",
            "+nightly-2026-03-03",
            *arguments,
            "--manifest-path",
            str(FIXTURE_MANIFEST),
        ],
        capture=capture,
        standalone_fixture=True,
    )


def load_fixture_catalog(feature: str) -> list[dict[str, Any]]:
    output = cargo_fixture(
        "run",
        "--quiet",
        "--no-default-features",
        "--features",
        feature,
        capture=True,
    )
    try:
        entries = json.loads(output)
    except json.JSONDecodeError as error:
        raise ContractError(f"{feature} fixture emitted invalid JSON: {error}") from error
    if not isinstance(entries, list):
        raise ContractError(f"{feature} fixture must emit a JSON array")
    return entries


def entry_map(entries: list[dict[str, Any]], label: str) -> dict[str, dict[str, Any]]:
    result: dict[str, dict[str, Any]] = {}
    ids: dict[str, str] = {}
    for entry in entries:
        name = entry.get("name")
        runtime_id = entry.get("id_hex")
        if not isinstance(name, str) or not isinstance(runtime_id, str):
            raise ContractError(f"{label} contains a malformed runtime entry")
        if name in result:
            raise ContractError(f"{label} contains duplicate runtime name {name!r}")
        if runtime_id in ids:
            raise ContractError(
                f"{label} contains ID {runtime_id} for both {ids[runtime_id]!r} and {name!r}"
            )
        result[name] = entry
        ids[runtime_id] = name
    return result


def verify_normal_standard_surface() -> None:
    run(
        [
            "cargo",
            "+nightly-2026-03-03",
            "test",
            "-p",
            "mech-stdlib",
            "--test",
            "profile_contracts",
            "--no-default-features",
            "--features",
            "standard_runtime",
            "standard_runtime_matches_the_frozen_runtime_surface",
            "--",
            "--exact",
        ]
    )


def verify_feature_graph(feature: str) -> None:
    graph = cargo_fixture(
        "tree",
        "--no-default-features",
        "--features",
        feature,
        "-e",
        "features",
        capture=True,
    )
    forbidden = [
        'feature "source"',
        'feature "compiler"',
        "mech-bytecode v",
        "mech-syntax v",
    ]
    if feature in {"installers", "owner-native-link"} or feature.startswith(
        "installer-"
    ):
        forbidden.append('feature "native-plan"')
    for marker in forbidden:
        if marker in graph:
            raise ContractError(
                f"fixture feature {feature!r} unexpectedly activates {marker!r}"
            )

def verify_owner_layers() -> None:
    for package, manifest in OWNER_MANIFESTS.items():
        compact = "".join(manifest.read_text(encoding="utf-8").split())
        if 'native-plan=["runtime","mech-core/native-plan"]' not in compact:
            raise ContractError(f"{package} has an invalid native-plan feature edge")
        if 'native-link=["runtime"]' not in compact:
            raise ContractError(f"{package} has an invalid native-link feature edge")

        source = manifest.parent / "src/lib.rs"
        source_compact = "".join(source.read_text(encoding="utf-8").split())
        if "pubmod__mech_native" not in source_compact:
            raise ContractError(f"{package} does not expose root __mech_native")

    stdlib = "".join(
        (REPOSITORY_ROOT / "src/stdlib/Cargo.toml")
        .read_text(encoding="utf-8")
        .split()
    )
    for package in OWNER_MANIFESTS:
        expected = (
            f'"{package}/native-plan"'
            if package == "mech-engine"
            else f'"{package}?/native-plan"'
        )
        if expected not in stdlib:
            raise ContractError(f"mech-stdlib/native-plan omits {expected}")
    if '"mech-core/native-plan"' not in stdlib:
        raise ContractError("mech-stdlib/native-plan omits mech-core/native-plan")

    cargo_fixture(
        "check",
        "--quiet",
        "--no-default-features",
        "--features",
        "owner-native-link",
    )


def verify_representatives(
    entries: list[dict[str, Any]], *, require_exact: bool
) -> list[dict[str, Any]]:
    catalog = entry_map(entries, "representative catalog")
    linked = {
        name: entry for name, entry in catalog.items() if entry.get("package") is not None
    }
    if require_exact and set(linked) != set(EXPECTED_REPRESENTATIVES):
        raise ContractError(
            "representative catalog linkage names differ: "
            f"actual={sorted(linked)}, expected={sorted(EXPECTED_REPRESENTATIVES)}"
        )
    missing_representatives = set(EXPECTED_REPRESENTATIVES) - set(linked)
    if missing_representatives:
        raise ContractError(
            "representative catalog omits required linkage names: "
            f"{sorted(missing_representatives)}"
        )

    paths: set[str] = set()
    verified: list[dict[str, Any]] = []
    for name, expected in EXPECTED_REPRESENTATIVES.items():
        entry = linked[name]
        actual = {key: entry.get(key) for key in expected}
        if actual != expected:
            raise ContractError(
                f"native linkage mismatch for {name!r}: actual={actual}, expected={expected}"
            )
        features = entry["cargo_features"]
        if features != sorted(set(features)):
            raise ContractError(f"Cargo features are not sorted and unique for {name!r}")
        path = entry["installer_path"]
        if path in paths:
            raise ContractError(f"duplicate installer path {path!r}")
        paths.add(path)
        verified.append(
            {
                "runtime_factory_name": name,
                "runtime_factory_id": entry["id_hex"],
                "package": entry["package"],
                "crate_name": entry["crate_name"],
                "installer_path": path,
                "cargo_features": features,
            }
        )
    verified.sort(key=lambda entry: (entry["runtime_factory_id"], entry["runtime_factory_name"]))
    return verified


def owner_package(owner: str) -> str:
    return "mech-engine" if owner == "mech-engine::stdlib" else owner


def infer_family(name: str) -> str:
    return name.split("<", 1)[0]


def grouped_missing(
    frozen_entries: list[dict[str, Any]],
    actual: dict[str, dict[str, Any]],
) -> tuple[list[dict[str, Any]], int, int]:
    packages: dict[str, dict[str, list[dict[str, str]]]] = {}
    linked_count = 0
    missing_count = 0
    for frozen in frozen_entries:
        name = frozen["name"]
        live = actual.get(name)
        if live is None or live["id_hex"] != frozen["id_hex"]:
            raise ContractError(f"native-plan catalog changed frozen runtime entry {name!r}")
        if live.get("package") is not None:
            linked_count += 1
            continue
        missing_count += 1
        package = owner_package(frozen["owner"])
        family = infer_family(name)
        packages.setdefault(package, {}).setdefault(family, []).append(
            {
                "runtime_factory_name": name,
                "runtime_factory_id": frozen["id_hex"],
            }
        )

    grouped = []
    for package, families in sorted(packages.items()):
        grouped.append(
            {
                "package": package,
                "families": [
                    {
                        "inferred_operation_or_family": family,
                        "runtime_factories": sorted(
                            entries,
                            key=lambda entry: (
                                entry["runtime_factory_id"],
                                entry["runtime_factory_name"],
                            ),
                        ),
                    }
                    for family, entries in sorted(families.items())
                ],
            }
        )
    return grouped, linked_count, missing_count


def build_report(mode: str) -> tuple[dict[str, Any], int]:
    frozen_bytes = FROZEN_SURFACE.read_bytes()
    frozen_digest = sha256(frozen_bytes).hexdigest()
    if frozen_digest != EXPECTED_STANDARD_SURFACE_SHA256:
        raise ContractError(
            "frozen runtime surface digest changed: "
            f"actual={frozen_digest}, expected={EXPECTED_STANDARD_SURFACE_SHA256}"
        )
    frozen = json.loads(frozen_bytes)
    frozen_entries = frozen["runtime_factories"]
    if len(frozen_entries) != EXPECTED_STANDARD_COUNT:
        raise ContractError(
            f"frozen runtime surface has {len(frozen_entries)} entries; "
            f"expected {EXPECTED_STANDARD_COUNT}"
        )

    standard_entries = load_fixture_catalog("standard")
    standard = entry_map(standard_entries, "native-plan standard catalog")
    frozen_names = {entry["name"] for entry in frozen_entries}
    missing_standard = sorted(frozen_names - set(standard))
    extras = sorted(set(standard) - frozen_names)
    if missing_standard or extras:
        raise ContractError(
            "runtime/native-plan executable surfaces differ: "
            f"missing={missing_standard}, extras={extras}"
        )

    expected_standard_linked = set(EXPECTED_REPRESENTATIVES).intersection(frozen_names)
    actual_standard_linked = {
        name for name, entry in standard.items() if entry.get("package") is not None
    }
    if actual_standard_linked != expected_standard_linked:
        raise ContractError(
            "Phase 1 standard linkage names differ: "
            f"actual={sorted(actual_standard_linked)}, "
            f"expected={sorted(expected_standard_linked)}"
        )

    representatives = verify_representatives(
        load_fixture_catalog("representatives"),
        require_exact=mode == "phase1",
    )
    missing, standard_linked, missing_count = grouped_missing(frozen_entries, standard)

    report = {
        "schema": 1,
        "profile": frozen["profile"],
        "standard_runtime": {
            "entry_count": len(frozen_entries),
            "linked_entry_count": standard_linked,
            "missing_linkage_count": missing_count,
            "surface_sha256": frozen_digest,
            "digest_algorithm": "sha256-raw-frozen-runtime-surface-json",
        },
        "selected_representative_contracts": representatives,
        "representatives_outside_standard_surface": [
            {
                "runtime_factory_name": "AddM2M2<f64>",
                "runtime_factory_id": EXPECTED_REPRESENTATIVES["AddM2M2<f64>"][
                    "id_hex"
                ],
                "reason": (
                    "fixed matrix2 factories are intentionally outside the standard "
                    "dynamic-shape runtime profile"
                ),
            },
            {
                "runtime_factory_name": "HorizontalConcatenateS2<f64>",
                "runtime_factory_id": EXPECTED_REPRESENTATIVES[
                    "HorizontalConcatenateS2<f64>"
                ]["id_hex"],
                "reason": "fixed row-vector constructor is outside the standard dynamic-shape runtime profile",
            },
            {
                "runtime_factory_name": (
                    "VerticalConcatenateR2R2<f64Matrix2RowVector2RowVector2>"
                ),
                "runtime_factory_id": EXPECTED_REPRESENTATIVES[
                    "VerticalConcatenateR2R2<f64Matrix2RowVector2RowVector2>"
                ]["id_hex"],
                "reason": "fixed matrix constructor is outside the standard dynamic-shape runtime profile",
            },
        ],
        "missing_linkage": {
            "family_inference": "runtime-name-before-generic-v1",
            "packages": missing,
        },
    }
    canonical_report = json.dumps(
        report,
        sort_keys=True,
        separators=(",", ":"),
    ).encode("utf-8")
    report["coverage_digest"] = {
        "algorithm": "sha256-canonical-json-without-coverage-digest-v1",
        "sha256": sha256(canonical_report).hexdigest(),
    }
    return report, missing_count


def check_or_write_report(report: dict[str, Any], mode: str) -> None:
    rendered = json.dumps(report, indent=2, sort_keys=False) + "\n"
    if mode == "report":
        REPORT_PATH.parent.mkdir(parents=True, exist_ok=True)
        REPORT_PATH.write_text(rendered, encoding="utf-8")
        return
    if not REPORT_PATH.is_file():
        raise ContractError(f"committed coverage report is missing: {REPORT_PATH}")
    if REPORT_PATH.read_text(encoding="utf-8") != rendered:
        raise ContractError(
            "committed coverage report is stale; regenerate with "
            "check-native-linkage-coverage.py report"
        )


def verify_installers() -> None:
    for feature in EXACT_INSTALLER_FEATURES:
        cargo_fixture(
            "check",
            "--quiet",
            "--no-default-features",
            "--features",
            feature,
        )
    cargo_fixture(
        "test",
        "--quiet",
        "--no-default-features",
        "--features",
        "installers",
    )


def main() -> int:
    mode = sys.argv[1] if len(sys.argv) == 2 else "phase1"
    if mode not in {"phase1", "strict", "report"}:
        print(
            "usage: scripts/check-native-linkage-coverage.py [phase1|strict|report]",
            file=sys.stderr,
        )
        return 2

    try:
        if mode != "report":
            verify_normal_standard_surface()
            verify_owner_layers()
            verify_installers()
            for feature in (
                "standard",
                "representatives",
                "installers",
                "owner-native-link",
                *EXACT_INSTALLER_FEATURES,
            ):
                verify_feature_graph(feature)

        report, missing_count = build_report(mode)
        check_or_write_report(report, mode)
        print(
            "native linkage coverage: "
            f"{report['standard_runtime']['linked_entry_count']} linked standard entries, "
            f"{missing_count} missing, "
            f"{len(report['selected_representative_contracts'])} selected representatives"
        )
        report_action = "wrote" if mode == "report" else "validated"
        print(f"{report_action} {REPORT_PATH.relative_to(REPOSITORY_ROOT)}")

        if mode == "strict" and missing_count:
            print(
                f"strict native linkage coverage failed: {missing_count} entries lack metadata",
                file=sys.stderr,
            )
            return 1
        return 0
    except (ContractError, KeyError, OSError, TypeError, ValueError) as error:
        print(f"native linkage coverage failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
