#!/usr/bin/env python3
"""Generate and validate the complete native factory linkage surface."""

from __future__ import annotations

import json
from hashlib import sha256
import os
from pathlib import Path
import re
import subprocess
import sys
from threading import Event, Thread
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
FIXTURE_MANIFEST = ROOT / "tests/fixtures/native-linkage/Cargo.toml"
FROZEN_SURFACE = ROOT / "tests/architecture/function-system/runtime-factory-surface.json"
REPORT_PATH = ROOT / "tests/architecture/native-linkage/coverage.json"
DETAIL_REPORT_PATH = ROOT / "target/native-linkage/coverage-full.json"
EXPECTED_STANDARD_COUNT = 9_019
EXPECTED_STANDARD_SURFACE_SHA256 = (
    "b9db9003bb9da704d5b61a5a6a3d5fcc6438ef7e433f49fc1918c466fc2fcc62"
)
OWNERS: dict[str, tuple[Path, str, str]] = {
    "mech-engine": (ROOT / "src/engine/Cargo.toml", "extended-engine", "stdlib"),
    "mech-math": (ROOT / "machines/math/Cargo.toml", "extended-math", "runtime_default"),
    "mech-compare": (ROOT / "machines/compare/Cargo.toml", "extended-compare", "runtime_default"),
    "mech-logic": (ROOT / "machines/logic/Cargo.toml", "extended-logic", "runtime_default"),
    "mech-range": (ROOT / "machines/range/Cargo.toml", "extended-range", "runtime_default"),
    "mech-matrix": (ROOT / "machines/matrix/Cargo.toml", "extended-matrix", "runtime_default"),
    "mech-set": (ROOT / "machines/set/Cargo.toml", "extended-set", "runtime_default"),
    "mech-string": (ROOT / "machines/string/Cargo.toml", "extended-string", "runtime_default"),
    "mech-stats": (ROOT / "machines/stats/Cargo.toml", "extended-stats", "runtime_default"),
    "mech-combinatorics": (
        ROOT / "machines/combinatorics/Cargo.toml",
        "extended-combinatorics",
        "runtime_default",
    ),
}
ENGINE_SURFACE_SHARDS = (
    "extended-engine-shard-unsigned",
    "extended-engine-shard-signed",
    "extended-engine-shard-float",
    "extended-engine-shard-convert",
)
CI_EXTENDED_SURFACES = ENGINE_SURFACE_SHARDS + tuple(
    feature for package, (_, feature, _) in OWNERS.items() if package != "mech-engine"
)
CI_SURFACES = ("standard",) + CI_EXTENDED_SURFACES
SURFACE_DIRECTORY = ROOT / "target/native-linkage/surfaces"
FEATURE_NAME = re.compile(r"^[a-z0-9][a-z0-9_-]*$")
RUST_PATH = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)+$")


class ContractError(RuntimeError):
    pass


def canonical_bytes(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":")).encode("utf-8")


def digest(value: Any) -> str:
    return sha256(canonical_bytes(value)).hexdigest()


def run(command: list[str], *, capture: bool = False, fixture: bool = False) -> str:
    environment = os.environ.copy()
    environment.setdefault("CARGO_INCREMENTAL", "0")
    # The fixture executes catalog construction only; debug information does
    # not participate in its linkage contract.  Omitting it keeps the complete
    # all-shape engine profile below the command supervisor limit without
    # weakening feature, metadata, or runtime-set validation.
    environment.setdefault("CARGO_PROFILE_DEV_DEBUG", "0")
    environment.setdefault("CARGO_PROFILE_DEV_CODEGEN_UNITS", "256")
    target = Path(environment.get("CARGO_TARGET_DIR", ROOT / "target"))
    environment["CARGO_TARGET_DIR"] = str(
        target / "native-linkage-fixture" if fixture else target
    )
    heartbeat_stop = Event()

    def report_progress() -> None:
        while not heartbeat_stop.wait(60):
            print(
                f"native linkage command still running: {' '.join(command)}",
                file=sys.stderr,
                flush=True,
            )

    heartbeat = Thread(target=report_progress, name="native-linkage-heartbeat", daemon=True)
    heartbeat.start()
    try:
        completed = subprocess.run(
            command,
            cwd=ROOT,
            env=environment,
            check=False,
            stdout=subprocess.PIPE if capture else None,
            text=True,
        )
    finally:
        heartbeat_stop.set()
        heartbeat.join()
    if completed.returncode:
        raise ContractError(f"command failed ({completed.returncode}): {' '.join(command)}")
    return completed.stdout if capture else ""


def fixture_catalog(feature: str) -> list[dict[str, Any]]:
    output = run(
        [
            "cargo", "+nightly-2026-03-03", "run",
            "--manifest-path", str(FIXTURE_MANIFEST), "--no-default-features",
            "--features", feature,
        ],
        capture=True,
        fixture=True,
    )
    try:
        result = json.loads(output)
    except json.JSONDecodeError as error:
        raise ContractError(f"{feature} emitted invalid JSON: {error}") from error
    if not isinstance(result, list):
        raise ContractError(f"{feature} did not emit a factory list")
    return result


def manifest_features() -> dict[str, set[str]]:
    result: dict[str, set[str]] = {}
    for package, (manifest, _, _) in OWNERS.items():
        source = manifest.read_text(encoding="utf-8")
        section = re.search(r"^\[features\]$(.*?)(?=^\[|\Z)", source, re.MULTILINE | re.DOTALL)
        if section is None:
            raise ContractError(f"{package} has no Cargo feature table")
        result[package] = set(re.findall(r"^([A-Za-z0-9_-]+)\s*=", section.group(1), re.MULTILINE))
    return result


def validate_catalog(
    entries: list[dict[str, Any]], label: str, known_features: dict[str, set[str]]
) -> list[dict[str, Any]]:
    by_id: dict[str, str] = {}
    by_name: dict[str, str] = {}
    paths: dict[str, str] = {}
    clean: list[dict[str, Any]] = []
    for value in entries:
        name = value.get("name")
        runtime_id = value.get("id_hex")
        package = value.get("package")
        crate_name = value.get("crate_name")
        installer = value.get("installer_path")
        features = value.get("cargo_features")
        if not isinstance(name, str) or not isinstance(runtime_id, str):
            raise ContractError(f"{label} contains a malformed runtime factory")
        if runtime_id in by_id and by_id[runtime_id] != name:
            raise ContractError(f"{label} duplicates runtime ID {runtime_id}")
        if name in by_name and by_name[name] != runtime_id:
            raise ContractError(f"{label} duplicates exact name {name!r}")
        by_id[runtime_id] = name
        by_name[name] = runtime_id
        if not all(isinstance(item, str) for item in (package, crate_name, installer)):
            raise ContractError(f"{label} factory {name!r} has no native linkage")
        if package not in known_features or not RUST_PATH.fullmatch(installer):
            raise ContractError(f"{label} factory {name!r} has invalid linkage metadata")
        if not isinstance(features, list) or not all(isinstance(item, str) for item in features):
            raise ContractError(f"{label} factory {name!r} has invalid Cargo features")
        if features != sorted(set(features)):
            raise ContractError(f"{label} factory {name!r} has unsorted or duplicate features")
        invalid = [item for item in features if not FEATURE_NAME.fullmatch(item)]
        unknown = [item for item in features if item not in known_features[package]]
        if invalid:
            raise ContractError(f"{label} factory {name!r} has invalid features {invalid}")
        if unknown:
            raise ContractError(f"{label} factory {name!r} has unknown features {unknown}")
        required = {"runtime", "native-link"}
        forbidden = {
            "default", "runtime_default", "source", "source_default", "compiler",
            "compiler_default", "standard_runtime", "standard_source",
            "standard_compiler", "native-plan", "stdlib", "baselib",
        }
        if not required.issubset(features):
            raise ContractError(f"{label} factory {name!r} omits required native features")
        if set(features).intersection(forbidden):
            raise ContractError(f"{label} factory {name!r} contains a forbidden aggregate feature")
        if installer in paths and paths[installer] != name:
            raise ContractError(f"{label} duplicates installer path {installer!r}")
        paths[installer] = name
        clean.append(
            {
                "runtime_factory_id": runtime_id,
                "runtime_factory_name": name,
                "package": package,
                "crate_name": crate_name,
                "installer_path": installer,
                "cargo_features": features,
            }
        )
    return sorted(clean, key=lambda item: (item["runtime_factory_id"], item["runtime_factory_name"]))


def merge_surfaces(label: str, surfaces: list[list[dict[str, Any]]]) -> list[dict[str, Any]]:
    by_id: dict[str, dict[str, Any]] = {}
    by_name: dict[str, dict[str, Any]] = {}
    by_installer: dict[str, dict[str, Any]] = {}
    for entry in (entry for surface in surfaces for entry in surface):
        prior_id = by_id.get(entry["runtime_factory_id"])
        prior_name = by_name.get(entry["runtime_factory_name"])
        prior_installer = by_installer.get(entry["installer_path"])
        for prior, key in ((prior_id, "runtime ID"), (prior_name, "exact name"), (prior_installer, "installer path")):
            if prior is not None and prior != entry:
                raise ContractError(f"{label} has conflicting duplicate {key}")
        by_id[entry["runtime_factory_id"]] = entry
        by_name[entry["runtime_factory_name"]] = entry
        by_installer[entry["installer_path"]] = entry
    return sorted(by_id.values(), key=lambda item: (item["runtime_factory_id"], item["runtime_factory_name"]))


def family(name: str) -> str:
    return name.split("<", 1)[0]


def grouped(entries: list[dict[str, Any]]) -> list[dict[str, Any]]:
    packages: dict[str, dict[str, list[dict[str, Any]]]] = {}
    for entry in entries:
        packages.setdefault(entry["package"], {}).setdefault(
            family(entry["runtime_factory_name"]), []
        ).append(entry)
    return [
        {
            "package": package,
            "operations_or_families": [
                {
                    "operation_or_family": name,
                    "runtime_factories": sorted(
                        values,
                        key=lambda item: (item["runtime_factory_id"], item["runtime_factory_name"]),
                    ),
                }
                for name, values in sorted(families.items())
            ],
        }
        for package, families in sorted(packages.items())
    ]


def surface_summary(entries: list[dict[str, Any]]) -> dict[str, Any]:
    runtime = [
        {"runtime_factory_id": item["runtime_factory_id"], "runtime_factory_name": item["runtime_factory_name"]}
        for item in entries
    ]
    installers = [
        {"runtime_factory_id": item["runtime_factory_id"], "installer_path": item["installer_path"]}
        for item in entries
    ]
    feature_sets = [
        {"runtime_factory_id": item["runtime_factory_id"], "cargo_features": item["cargo_features"]}
        for item in entries
    ]
    return {
        "entry_count": len(entries),
        "linked_entry_count": len(entries),
        "missing_linkage_count": 0,
        "runtime_surface_digest": digest(runtime),
        "installer_surface_digest": digest(installers),
        "feature_surface_digest": digest(feature_sets),
    }


def verify_standard_surface(entries: list[dict[str, Any]]) -> None:
    raw = FROZEN_SURFACE.read_bytes()
    if sha256(raw).hexdigest() != EXPECTED_STANDARD_SURFACE_SHA256:
        raise ContractError("frozen standard runtime surface digest changed")
    frozen = json.loads(raw)["runtime_factories"]
    if len(frozen) != EXPECTED_STANDARD_COUNT:
        raise ContractError("frozen standard runtime surface count changed")
    expected = {(item["id_hex"], item["name"]) for item in frozen}
    actual = {(item["runtime_factory_id"], item["runtime_factory_name"]) for item in entries}
    if actual != expected:
        raise ContractError("runtime/native-plan drift in the frozen standard surface")


def assemble_report(
    standard: list[dict[str, Any]], extended_surfaces: list[list[dict[str, Any]]]
) -> dict[str, Any]:
    extended = merge_surfaces("extended linkage universe", extended_surfaces)
    all_entries = merge_surfaces("complete linkage universe", [standard, extended])
    report = {
        "schema": "mech.native-linkage-coverage.v2",
        "standard": surface_summary(standard),
        "extended": surface_summary(extended),
        "entries": grouped(all_entries),
    }
    report["coverage_digest"] = {
        "algorithm": "sha256-canonical-json-without-coverage-digest-v2",
        "sha256": digest(report),
    }
    return report


def build_report() -> dict[str, Any]:
    known = manifest_features()
    standard = validate_catalog(fixture_catalog("standard"), "standard", known)
    verify_standard_surface(standard)
    extended_by_owner = [
        validate_catalog(fixture_catalog(feature), f"{package} extended", known)
        for package, (_, feature, _) in OWNERS.items()
    ]
    return assemble_report(standard, extended_by_owner)


def write_ci_surface(feature: str) -> None:
    if feature not in CI_SURFACES:
        raise ContractError(f"unknown CI linkage surface {feature!r}")
    known = manifest_features()
    entries = validate_catalog(fixture_catalog(feature), feature, known)
    if feature == "standard":
        verify_standard_surface(entries)
    surface = {
        "schema": "mech.native-linkage-surface.v1",
        "feature": feature,
        "kind": "standard" if feature == "standard" else "extended",
        "entries": entries,
    }
    SURFACE_DIRECTORY.mkdir(parents=True, exist_ok=True)
    path = SURFACE_DIRECTORY / f"{feature}.json"
    path.write_text(json.dumps(surface, separators=(",", ":")) + "\n", encoding="utf-8")
    print(f"wrote {len(entries)} entries to {path.relative_to(ROOT)}")


def read_ci_surface(path: Path, feature: str, known: dict[str, set[str]]) -> list[dict[str, Any]]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict) or value.get("schema") != "mech.native-linkage-surface.v1":
        raise ContractError(f"{feature}: invalid CI linkage surface schema")
    expected_kind = "standard" if feature == "standard" else "extended"
    if value.get("feature") != feature or value.get("kind") != expected_kind:
        raise ContractError(f"{feature}: CI linkage surface identity changed")
    entries = value.get("entries")
    if not isinstance(entries, list) or not all(isinstance(entry, dict) for entry in entries):
        raise ContractError(f"{feature}: CI linkage surface entries are malformed")
    raw_entries = [
        {
            "name": entry.get("runtime_factory_name"),
            "id_hex": entry.get("runtime_factory_id"),
            "package": entry.get("package"),
            "crate_name": entry.get("crate_name"),
            "installer_path": entry.get("installer_path"),
            "cargo_features": entry.get("cargo_features"),
        }
        for entry in entries
    ]
    return validate_catalog(raw_entries, feature, known)


def build_report_from_ci_surfaces() -> dict[str, Any]:
    observed = {path.stem for path in SURFACE_DIRECTORY.glob("*.json")}
    expected = set(CI_SURFACES)
    if observed != expected:
        missing = sorted(expected - observed)
        unexpected = sorted(observed - expected)
        raise ContractError(
            f"CI linkage surfaces changed: missing={missing}, unexpected={unexpected}"
        )
    known = manifest_features()
    surfaces = {
        feature: read_ci_surface(SURFACE_DIRECTORY / f"{feature}.json", feature, known)
        for feature in CI_SURFACES
    }
    standard = surfaces.pop("standard")
    verify_standard_surface(standard)
    return assemble_report(standard, [surfaces[feature] for feature in CI_EXTENDED_SURFACES])


def report_summary(report: dict[str, Any]) -> dict[str, Any]:
    return {
        "schema": "mech.native-linkage-coverage-summary.v1",
        "detail_schema": report["schema"],
        "standard": report["standard"],
        "extended": report["extended"],
        "coverage_digest": report["coverage_digest"],
        "detail_artifact": {
            "path": str(DETAIL_REPORT_PATH.relative_to(ROOT)),
            "generated_by": "python3 scripts/check-native-linkage-coverage.py strict",
        },
    }


def verify_owner_native_link_profiles() -> None:
    for package, (manifest, _, profile) in OWNERS.items():
        run(
            [
                "cargo", "+nightly-2026-03-03", "check",
                "--manifest-path", str(manifest), "--no-default-features",
                "--features", f"{profile} native-link",
            ]
        )


def verify_owner_contracts() -> None:
    for package, (manifest, _, _) in OWNERS.items():
        compact = "".join(manifest.read_text(encoding="utf-8").split())
        if 'native-plan=["runtime","mech-core/native-plan"]' not in compact:
            raise ContractError(f"{package} has an invalid native-plan feature edge")
        if 'native-link=["runtime"]' not in compact:
            raise ContractError(f"{package} has an invalid native-link feature edge")
        source = manifest.parent / "src/lib.rs"
        if "pubmod__mech_native" not in "".join(source.read_text(encoding="utf-8").split()):
            raise ContractError(f"{package} does not expose __mech_native")


def main() -> int:
    if len(sys.argv) == 3 and sys.argv[1] == "surface":
        try:
            write_ci_surface(sys.argv[2])
            return 0
        except (ContractError, OSError, TypeError, ValueError, KeyError) as error:
            print(f"native linkage coverage failed: {error}", file=sys.stderr)
            return 1
    mode = sys.argv[1] if len(sys.argv) == 2 else "strict"
    if mode not in {"coverage", "merge", "owners", "report", "strict"}:
        print(
            "usage: scripts/check-native-linkage-coverage.py "
            "[coverage|merge|owners|report|strict|surface FEATURE]",
            file=sys.stderr,
        )
        return 2
    try:
        if mode in {"owners", "strict"}:
            verify_owner_contracts()
            verify_owner_native_link_profiles()
        if mode == "owners":
            print(f"validated {len(OWNERS)} isolated owner native-link profiles")
            return 0
        report = build_report_from_ci_surfaces() if mode == "merge" else build_report()
        summary = report_summary(report)
        rendered = json.dumps(summary, indent=2) + "\n"
        DETAIL_REPORT_PATH.parent.mkdir(parents=True, exist_ok=True)
        DETAIL_REPORT_PATH.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
        if mode == "report":
            REPORT_PATH.parent.mkdir(parents=True, exist_ok=True)
            REPORT_PATH.write_text(rendered, encoding="utf-8")
            action = "wrote"
        else:
            if not REPORT_PATH.is_file() or REPORT_PATH.read_text(encoding="utf-8") != rendered:
                raise ContractError("coverage report is stale; run `check-native-linkage-coverage.py report`")
            action = "validated"
        print(
            f"native linkage coverage: {report['standard']['entry_count']} standard and "
            f"{report['extended']['entry_count']} extended entries, zero missing linkage"
        )
        print(f"{action} {REPORT_PATH.relative_to(ROOT)}")
        print(f"generated full inventory at {DETAIL_REPORT_PATH.relative_to(ROOT)}")
        return 0
    except (ContractError, OSError, TypeError, ValueError, KeyError) as error:
        print(f"native linkage coverage failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
