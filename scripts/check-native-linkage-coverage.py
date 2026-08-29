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
EXACT_CLOSURE_DIRECTORY = ROOT / "target/native-linkage/exact-closures"
EXACT_CLOSURE_SHARDS = 8
NAMED_EXACT_CLOSURE_REGRESSIONS = {
    "DotM1M1<f64>",
    "MatMulR3M3x2<f64>",
    "MatMulRDVD<f64>",
    "TransposeM2x3<c64>",
    "TransposeM2x3<r64>",
}
PREFERRED_OWNER_REPRESENTATIVES = {
    "mech-engine": "VariableDefineF64",
    "mech-math": "AddSS<f64>",
}
EXPECTED_FULL_COUNT = 9_033
EXPECTED_FULL_SURFACE_SHA256 = (
    "031accdaa26458a494f5331b0e1db1b54b138a3ff2df4ac4c03351e5bf8eb306"
)
OWNERS: dict[str, tuple[Path, str, str]] = {
    "mech-engine": (ROOT / "src/engine/Cargo.toml", "extended-engine", "stdlib"),
    "mech-math": (ROOT / "machines/math/Cargo.toml", "extended-math", "full_runtime"),
    "mech-compare": (ROOT / "machines/compare/Cargo.toml", "extended-compare", "full_runtime"),
    "mech-logic": (ROOT / "machines/logic/Cargo.toml", "extended-logic", "full_runtime"),
    "mech-range": (ROOT / "machines/range/Cargo.toml", "extended-range", "full_runtime"),
    "mech-matrix": (ROOT / "machines/matrix/Cargo.toml", "extended-matrix", "full_runtime"),
    "mech-set": (ROOT / "machines/set/Cargo.toml", "extended-set", "full_runtime"),
    "mech-string": (ROOT / "machines/string/Cargo.toml", "extended-string", "full_runtime"),
    "mech-stats": (ROOT / "machines/stats/Cargo.toml", "extended-stats", "full_runtime"),
    "mech-combinatorics": (
        ROOT / "machines/combinatorics/Cargo.toml",
        "extended-combinatorics",
        "full_runtime",
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
CI_SURFACES = ("full",) + CI_EXTENDED_SURFACES
SURFACE_DIRECTORY = ROOT / "target/native-linkage/surfaces"
FEATURE_NAME = re.compile(r"^[a-z0-9][a-z0-9_-]*$")
RUST_PATH = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)+$")
CONTRACT_KIND = re.compile(r"^[a-z][a-z0-9_]*$")
MATRIX_FEATURES = {
    "matrix", "matrix1", "matrix2", "matrix3", "matrix4", "matrix2x3", "matrix3x2",
    "matrixd", "row_vector2", "row_vector3", "row_vector4", "row_vectord",
    "vector2", "vector3", "vector4", "vectord",
}
REPRESENTATION_FEATURES = {
    "bool", "string",
    "u8", "u16", "u32", "u64", "u128",
    "i8", "i16", "i32", "i64", "i128",
    "f32", "f64", "c64", "r64",
    *MATRIX_FEATURES,
    "atom", "enum", "record", "map", "set", "table", "tuple", "kind_annotation",
}
MATRIX_MINIMAL_NATIVE_LINK_PROFILES = (
    "dot f64 matrix1 native-link runtime source",
    "matmul f64 matrix3x2 native-link row_vector2 row_vector3 runtime",
)


class ContractError(RuntimeError):
    pass


def canonical_bytes(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":")).encode("utf-8")


def digest(value: Any) -> str:
    return sha256(canonical_bytes(value)).hexdigest()


def run(
    command: list[str],
    *,
    capture: bool = False,
    fixture: bool = False,
    target_directory: Path | None = None,
) -> str:
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
        target_directory
        if target_directory is not None
        else target / "native-linkage-fixture" if fixture else target
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
    signatures_by_id: dict[str, str] = {}
    by_name: dict[str, str] = {}
    paths: dict[str, str] = {}
    clean: list[dict[str, Any]] = []
    for value in entries:
        name = value.get("name")
        runtime_id = value.get("id_hex")
        runtime_signature = value.get("runtime_signature")
        signature_features = value.get("signature_cargo_features")
        package = value.get("package")
        crate_name = value.get("crate_name")
        installer = value.get("installer_path")
        features = value.get("cargo_features")
        contract_kind = value.get("contract_kind")
        output_alias_policy = value.get("output_alias_policy")
        if not isinstance(name, str) or not isinstance(runtime_id, str):
            raise ContractError(f"{label} contains a malformed runtime factory")
        if not isinstance(runtime_signature, str) or not runtime_signature:
            raise ContractError(f"{label} factory {name!r} has no runtime signature")
        if not isinstance(signature_features, list) or not all(
            isinstance(item, str) for item in signature_features
        ):
            raise ContractError(f"{label} factory {name!r} has invalid signature features")
        if signature_features != sorted(set(signature_features)):
            raise ContractError(
                f"{label} factory {name!r} has unsorted or duplicate signature features"
            )
        if runtime_id in by_id and by_id[runtime_id] != name:
            raise ContractError(f"{label} duplicates runtime ID {runtime_id}")
        if (
            runtime_id in signatures_by_id
            and signatures_by_id[runtime_id] != runtime_signature
        ):
            raise ContractError(
                f"{label} runtime ID {runtime_id} has feature-dependent signatures"
            )
        if name in by_name and by_name[name] != runtime_id:
            raise ContractError(f"{label} duplicates exact name {name!r}")
        by_id[runtime_id] = name
        signatures_by_id[runtime_id] = runtime_signature
        by_name[name] = runtime_id
        if not all(isinstance(item, str) for item in (package, crate_name, installer)):
            raise ContractError(f"{label} factory {name!r} has no native linkage")
        if package not in known_features or not RUST_PATH.fullmatch(installer):
            raise ContractError(f"{label} factory {name!r} has invalid linkage metadata")
        if not isinstance(features, list) or not all(isinstance(item, str) for item in features):
            raise ContractError(f"{label} factory {name!r} has invalid Cargo features")
        if not isinstance(contract_kind, str) or not CONTRACT_KIND.fullmatch(contract_kind):
            raise ContractError(f"{label} factory {name!r} has an unknown contract kind")
        if contract_kind in {"unknown", "unchecked", "infer_from_name", "best_effort"}:
            raise ContractError(f"{label} factory {name!r} has forbidden contract kind {contract_kind!r}")
        if output_alias_policy not in {"disallow_input_alias", "allow_input_alias"}:
            raise ContractError(f"{label} factory {name!r} has an unknown output alias policy")
        if features != sorted(set(features)):
            raise ContractError(f"{label} factory {name!r} has unsorted or duplicate features")
        invalid = [item for item in features if not FEATURE_NAME.fullmatch(item)]
        unknown = [item for item in features if item not in known_features[package]]
        if invalid:
            raise ContractError(f"{label} factory {name!r} has invalid features {invalid}")
        if unknown:
            raise ContractError(f"{label} factory {name!r} has unknown features {unknown}")
        unknown_signature_features = [
            item for item in signature_features if item not in REPRESENTATION_FEATURES
        ]
        if unknown_signature_features:
            raise ContractError(
                f"{label} factory {name!r} has unknown representation features "
                f"{unknown_signature_features}"
            )
        linkage_representation_features = set(features).intersection(REPRESENTATION_FEATURES)
        missing_representation_features = sorted(
            set(signature_features).difference(linkage_representation_features)
        )
        manually_listed_representation_features = sorted(
            linkage_representation_features.difference(signature_features)
        )
        if missing_representation_features:
            raise ContractError(
                f"{label} factory {name!r} omits signature-derived representation features "
                f"{missing_representation_features}"
            )
        if manually_listed_representation_features:
            raise ContractError(
                f"{label} factory {name!r} manually lists non-signature representation "
                f"features {manually_listed_representation_features}"
            )
        required = {"runtime", "native-link"}
        forbidden = {
            "default", "source", "source_default", "compiler",
            "compiler_default", "full_runtime", "full_source",
            "full_compiler", "native-plan", "stdlib", "baselib",
        }
        if not required.issubset(features):
            raise ContractError(f"{label} factory {name!r} omits required native features")
        if set(features).intersection(forbidden):
            raise ContractError(f"{label} factory {name!r} contains a forbidden aggregate feature")
        if contract_kind == "no_matrix" and set(features).intersection(MATRIX_FEATURES):
            raise ContractError(f"{label} matrix factory {name!r} uses no_matrix")
        if output_alias_policy == "allow_input_alias" and not (
            name.startswith("Assign") or name.startswith("VariableDefine")
        ):
            raise ContractError(
                f"{label} factory {name!r} allows output aliasing outside register families"
            )
        if installer in paths and paths[installer] != name:
            raise ContractError(f"{label} duplicates installer path {installer!r}")
        paths[installer] = name
        clean.append(
            {
                "runtime_factory_id": runtime_id,
                "runtime_factory_name": name,
                "runtime_signature": runtime_signature,
                "signature_cargo_features": signature_features,
                "package": package,
                "crate_name": crate_name,
                "installer_path": installer,
                "cargo_features": features,
                "contract_kind": contract_kind,
                "output_alias_policy": output_alias_policy,
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
    contracts = [
        {
            "runtime_factory_id": item["runtime_factory_id"],
            "contract_kind": item["contract_kind"],
            "output_alias_policy": item["output_alias_policy"],
        }
        for item in entries
    ]
    signatures = [
        {
            "runtime_factory_id": item["runtime_factory_id"],
            "runtime_signature": item["runtime_signature"],
        }
        for item in entries
    ]
    return {
        "entry_count": len(entries),
        "linked_entry_count": len(entries),
        "missing_linkage_count": 0,
        "runtime_surface_digest": digest(runtime),
        "installer_surface_digest": digest(installers),
        "feature_surface_digest": digest(feature_sets),
        "runtime_contract_count": len(contracts),
        "missing_contract_count": 0,
        "runtime_contract_surface_digest": digest(contracts),
        "runtime_signature_count": len(signatures),
        "missing_signature_count": 0,
        "runtime_signature_surface_digest": digest(signatures),
    }


def verify_full_surface(entries: list[dict[str, Any]]) -> None:
    raw = FROZEN_SURFACE.read_bytes()
    if sha256(raw).hexdigest() != EXPECTED_FULL_SURFACE_SHA256:
        raise ContractError("frozen full runtime surface digest changed")
    frozen = json.loads(raw)["runtime_factories"]
    if len(frozen) != EXPECTED_FULL_COUNT:
        raise ContractError("frozen full runtime surface count changed")
    expected = {(item["id_hex"], item["name"]) for item in frozen}
    actual = {(item["runtime_factory_id"], item["runtime_factory_name"]) for item in entries}
    if actual != expected:
        raise ContractError("runtime/native-plan drift in the frozen full surface")


def assemble_report(
    full: list[dict[str, Any]], extended_surfaces: list[list[dict[str, Any]]]
) -> dict[str, Any]:
    extended = merge_surfaces("extended linkage universe", extended_surfaces)
    all_entries = merge_surfaces("complete linkage universe", [full, extended])
    report = {
        "schema": "mech.native-linkage-coverage.v2",
        "full": surface_summary(full),
        "extended": surface_summary(extended),
        "entries": grouped(all_entries),
        "signature_invariants": {
            "same_runtime_id_different_signature_count": 0,
            "representation_feature_missing_count": 0,
            "representation_feature_manually_listed_extra_count": 0,
            "feature_dependent_output_representation_count": 0,
        },
    }
    closure_inventory = exact_closures(report)
    report["signature_invariants"]["exact_closure_count"] = len(closure_inventory)
    report["signature_invariants"]["exact_closure_shard_counts"] = [
        sum(closure["shard"] == shard for closure in closure_inventory)
        for shard in range(EXACT_CLOSURE_SHARDS)
    ]
    report["coverage_digest"] = {
        "algorithm": "sha256-canonical-json-without-coverage-digest-v2",
        "sha256": digest(report),
    }
    return report


def build_report() -> dict[str, Any]:
    known = manifest_features()
    full = validate_catalog(fixture_catalog("full"), "full", known)
    verify_full_surface(full)
    extended_by_owner = [
        validate_catalog(fixture_catalog(feature), f"{package} extended", known)
        for package, (_, feature, _) in OWNERS.items()
    ]
    return assemble_report(full, extended_by_owner)


def write_ci_surface(feature: str) -> None:
    if feature not in CI_SURFACES:
        raise ContractError(f"unknown CI linkage surface {feature!r}")
    known = manifest_features()
    entries = validate_catalog(fixture_catalog(feature), feature, known)
    if feature == "full":
        verify_full_surface(entries)
    surface = {
        "schema": "mech.native-linkage-surface.v1",
        "feature": feature,
        "kind": "full" if feature == "full" else "extended",
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
    expected_kind = "full" if feature == "full" else "extended"
    if value.get("feature") != feature or value.get("kind") != expected_kind:
        raise ContractError(f"{feature}: CI linkage surface identity changed")
    entries = value.get("entries")
    if not isinstance(entries, list) or not all(isinstance(entry, dict) for entry in entries):
        raise ContractError(f"{feature}: CI linkage surface entries are malformed")
    raw_entries = [
        {
            "name": entry.get("runtime_factory_name"),
            "id_hex": entry.get("runtime_factory_id"),
            "runtime_signature": entry.get("runtime_signature"),
            "signature_cargo_features": entry.get("signature_cargo_features"),
            "package": entry.get("package"),
            "crate_name": entry.get("crate_name"),
            "installer_path": entry.get("installer_path"),
            "cargo_features": entry.get("cargo_features"),
            "contract_kind": entry.get("contract_kind"),
            "output_alias_policy": entry.get("output_alias_policy"),
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
    full = surfaces.pop("full")
    verify_full_surface(full)
    return assemble_report(full, [surfaces[feature] for feature in CI_EXTENDED_SURFACES])


def inventory_entries(report: dict[str, Any]) -> list[dict[str, Any]]:
    entries = [
        entry
        for package in report["entries"]
        for family_entry in package["operations_or_families"]
        for entry in family_entry["runtime_factories"]
    ]
    return sorted(
        entries,
        key=lambda entry: (entry["runtime_factory_id"], entry["runtime_factory_name"]),
    )


def exact_closures(report: dict[str, Any]) -> list[dict[str, Any]]:
    closures: dict[tuple[str, tuple[str, ...]], list[dict[str, Any]]] = {}
    for entry in inventory_entries(report):
        key = (entry["package"], tuple(entry["cargo_features"]))
        closures.setdefault(key, []).append(entry)

    result = []
    for (package, features), entries in sorted(closures.items()):
        crate_names = {entry["crate_name"] for entry in entries}
        if len(crate_names) != 1:
            raise ContractError(
                f"exact closure {package} {features} has inconsistent crate names"
            )
        installers = [entry["installer_path"] for entry in entries]
        if len(installers) != len(set(installers)):
            raise ContractError(
                f"exact closure {package} {features} contains duplicate installers"
            )
        closure_bytes = package.encode() + b"\0" + "\0".join(features).encode()
        closure_sha256 = sha256(closure_bytes).hexdigest()
        result.append(
            {
                "package": package,
                "crate_name": next(iter(crate_names)),
                "cargo_features": list(features),
                "entries": entries,
                "sha256": closure_sha256,
                "shard": int(closure_sha256, 16) % EXACT_CLOSURE_SHARDS,
            }
        )
    return sorted(result, key=lambda closure: closure["sha256"])


def require_named_closure_regressions(report: dict[str, Any]) -> None:
    by_name: dict[str, list[dict[str, Any]]] = {}
    for entry in inventory_entries(report):
        by_name.setdefault(entry["runtime_factory_name"], []).append(entry)

    cases = {
        "DotM1M1<f64>": ({"f64", "matrix1"}, {"matrix2"}),
        "MatMulR3M3x2<f64>": (
            {"f64", "row_vector3", "matrix3x2", "row_vector2"},
            set(),
        ),
        "MatMulRDVD<f64>": (
            {"f64", "row_vectord", "vectord", "matrix1"},
            {"matrixd"},
        ),
        "TransposeM2x3<c64>": ({"c64", "matrix2x3", "matrix3x2"}, set()),
        "TransposeM2x3<r64>": ({"r64", "matrix2x3", "matrix3x2"}, set()),
    }
    for name, (required, forbidden) in cases.items():
        entries = by_name.get(name, [])
        if len(entries) != 1:
            raise ContractError(
                f"named exact-closure regression {name!r} resolved to {len(entries)} entries"
            )
        features = set(entries[0]["cargo_features"])
        missing = sorted(required.difference(features))
        unexpected = sorted(forbidden.intersection(features))
        if missing or unexpected:
            raise ContractError(
                f"named exact-closure regression {name!r} has missing={missing}, "
                f"forbidden={unexpected}, closure={sorted(features)}"
            )


def exact_closure_manifest(closure: dict[str, Any]) -> str:
    fixture_manifest = FIXTURE_MANIFEST.read_text(encoding="utf-8")
    dependencies = fixture_manifest[fixture_manifest.index("[dependencies]") :]
    dependencies = re.sub(
        r'path = "\.\./\.\./\.\./([^\"]+)"',
        lambda match: f'path = "{ROOT / match.group(1)}"',
        dependencies,
    )
    feature_edges = [
        "dep:mech-core",
        f"dep:{closure['package']}",
        *(f"{closure['package']}/{feature}" for feature in closure["cargo_features"]),
    ]
    rendered_edges = ",\n".join(f"  {json.dumps(edge)}" for edge in feature_edges)
    return (
        "[package]\n"
        f"name = \"native-linkage-closure-{closure['sha256'][:16]}\"\n"
        "version = \"0.0.0\"\n"
        "edition = \"2024\"\n"
        "publish = false\n\n"
        "[features]\n"
        "default = []\n"
        f"exact = [\n{rendered_edges},\n]\n\n"
        f"{dependencies}"
    )


def exact_closure_source(closure: dict[str, Any]) -> str:
    installers = "\n".join(
        f"        {entry['installer_path']}(&mut builder).unwrap();"
        for entry in closure["entries"]
    )
    return f"""fn main() {{
    std::thread::Builder::new()
        .name("native-linkage-exact-closure".to_owned())
        .stack_size(64 * 1024 * 1024)
        .spawn(|| {{
            let mut builder = mech_core::FunctionCatalogBuilder::new();
{installers}
            let catalog = builder.build().unwrap();
            for entry in catalog.runtime_entries() {{
                println!(
                    "{{:016x}}\\t{{}}\\t{{:?}}",
                    entry.id.raw(),
                    entry.name,
                    entry.signature(),
                );
            }}
        }})
        .unwrap()
        .join()
        .unwrap();
}}
"""


def materialize_exact_closure(closure: dict[str, Any]) -> Path:
    project = EXACT_CLOSURE_DIRECTORY / "projects" / closure["sha256"]
    source_directory = project / "src"
    source_directory.mkdir(parents=True, exist_ok=True)
    files = {
        project / "Cargo.toml": exact_closure_manifest(closure),
        source_directory / "main.rs": exact_closure_source(closure),
    }
    for path, contents in files.items():
        if not path.is_file() or path.read_text(encoding="utf-8") != contents:
            path.write_text(contents, encoding="utf-8")
    return project


def validate_exact_closure(closure: dict[str, Any], shard: int) -> None:
    project = materialize_exact_closure(closure)
    command = [
        "cargo", "+nightly-2026-03-03", "run", "--quiet",
        "--manifest-path", str(project / "Cargo.toml"),
        "--no-default-features", "--features", "exact", "--offline",
    ]
    if (project / "Cargo.lock").is_file():
        command.append("--locked")
    output = run(
        command,
        capture=True,
        target_directory=EXACT_CLOSURE_DIRECTORY / f"shard-{shard}" / "cargo-target",
    )
    actual = sorted(
        tuple(line.split("\t", 2))
        for line in output.splitlines()
        if line.strip()
    )
    expected = sorted(
        (
            entry["runtime_factory_id"],
            entry["runtime_factory_name"],
            entry["runtime_signature"],
        )
        for entry in closure["entries"]
    )
    if actual != expected:
        raise ContractError(
            f"exact closure {closure['sha256']} for {closure['package']} "
            "did not install its exact ID/name/signature inventory"
        )


def validate_exact_closure_shard(report: dict[str, Any], shard: int) -> None:
    if not 0 <= shard < EXACT_CLOSURE_SHARDS:
        raise ContractError(
            f"exact closure shard must be in 0..{EXACT_CLOSURE_SHARDS - 1}, found {shard}"
        )
    require_named_closure_regressions(report)
    all_closures = exact_closures(report)
    closures = [closure for closure in all_closures if closure["shard"] == shard]
    representatives: dict[str, dict[str, Any]] = {}
    for closure in all_closures:
        current = representatives.get(closure["package"])
        preference = (
            bool(set(closure["cargo_features"]).intersection(MATRIX_FEATURES)),
            len(closure["cargo_features"]),
            closure["sha256"],
        )
        if current is None:
            representatives[closure["package"]] = closure
            continue
        current_preference = (
            bool(set(current["cargo_features"]).intersection(MATRIX_FEATURES)),
            len(current["cargo_features"]),
            current["sha256"],
        )
        if preference < current_preference:
            representatives[closure["package"]] = closure
    for closure in all_closures:
        preferred_name = PREFERRED_OWNER_REPRESENTATIVES.get(closure["package"])
        if preferred_name is not None and any(
            entry["runtime_factory_name"] == preferred_name
            for entry in closure["entries"]
        ):
            representatives[closure["package"]] = closure
    named = []
    for closure in all_closures:
        if any(
            entry["runtime_factory_name"] in NAMED_EXACT_CLOSURE_REGRESSIONS
            for entry in closure["entries"]
        ):
            named.append(closure)
    compiled = {
        closure["sha256"]: closure
        for closure in [*representatives.values(), *named]
        if closure["shard"] == shard
    }
    for index, closure in enumerate(compiled.values(), start=1):
        print(
            f"compiling exact native closure shard {shard}: {index}/{len(compiled)} "
            f"{closure['package']} {closure['cargo_features']}",
            flush=True,
        )
        validate_exact_closure(closure, shard)
    print(
        f"validated {len(closures)} exact native closure inventories and compiled "
        f"{len(compiled)} owner/named representatives in shard {shard}"
    )


def report_summary(report: dict[str, Any]) -> dict[str, Any]:
    return {
        "schema": "mech.native-linkage-coverage-summary.v1",
        "detail_schema": report["schema"],
        "full": report["full"],
        "extended": report["extended"],
        "signature_invariants": report["signature_invariants"],
        "coverage_digest": report["coverage_digest"],
        "detail_artifact": {
            "path": str(DETAIL_REPORT_PATH.relative_to(ROOT)),
            "generated_by": "python3 scripts/check-native-linkage-coverage.py strict",
        },
    }


def verify_owner_native_link_profiles(packages: list[str]) -> None:
    for package in packages:
        manifest, _, profile = OWNERS[package]
        run(
            [
                "cargo", "+nightly-2026-03-03", "check",
                "--manifest-path", str(manifest), "--no-default-features",
                "--features", f"{profile} native-link",
            ]
        )
        if package == "mech-matrix":
            for minimal_profile in MATRIX_MINIMAL_NATIVE_LINK_PROFILES:
                run(
                    [
                        "cargo", "+nightly-2026-03-03", "check",
                        "--manifest-path", str(manifest), "--no-default-features",
                        "--features", minimal_profile,
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
    if len(sys.argv) == 3 and sys.argv[1] == "closure":
        try:
            shard = int(sys.argv[2])
            report = json.loads(DETAIL_REPORT_PATH.read_text(encoding="utf-8"))
            if report.get("schema") != "mech.native-linkage-coverage.v2":
                raise ContractError("full native linkage inventory schema changed")
            validate_exact_closure_shard(report, shard)
            return 0
        except (ContractError, OSError, TypeError, ValueError, KeyError) as error:
            print(f"native exact closure validation failed: {error}", file=sys.stderr)
            return 1
    mode = sys.argv[1] if len(sys.argv) >= 2 else "strict"
    if mode not in {"coverage", "merge", "owners", "report", "strict"}:
        print(
            "usage: scripts/check-native-linkage-coverage.py "
            "[closure SHARD|coverage|merge|owners [PACKAGE ...]|report|strict|surface FEATURE]",
            file=sys.stderr,
        )
        return 2
    try:
        if mode != "owners" and len(sys.argv) > 2:
            raise ContractError(f"{mode} does not accept owner package arguments")
        requested_owners = sys.argv[2:] if mode == "owners" and len(sys.argv) > 2 else list(OWNERS)
        unknown_owners = sorted(set(requested_owners).difference(OWNERS))
        if unknown_owners:
            raise ContractError(f"unknown native-link owners: {unknown_owners}")
        if len(requested_owners) != len(set(requested_owners)):
            raise ContractError("native-link owner arguments contain duplicates")
        if mode in {"owners", "strict"}:
            verify_owner_contracts()
            verify_owner_native_link_profiles(requested_owners)
        if mode == "owners":
            print(f"validated {len(requested_owners)} isolated owner native-link profiles")
            return 0
        report = build_report_from_ci_surfaces() if mode == "merge" else build_report()
        require_named_closure_regressions(report)
        summary = report_summary(report)
        rendered = json.dumps(summary, indent=2) + "\n"
        DETAIL_REPORT_PATH.parent.mkdir(parents=True, exist_ok=True)
        DETAIL_REPORT_PATH.write_text(
            json.dumps(report, separators=(",", ":")) + "\n",
            encoding="utf-8",
        )
        if mode == "report":
            REPORT_PATH.parent.mkdir(parents=True, exist_ok=True)
            REPORT_PATH.write_text(rendered, encoding="utf-8")
            action = "wrote"
        else:
            if not REPORT_PATH.is_file() or REPORT_PATH.read_text(encoding="utf-8") != rendered:
                raise ContractError("coverage report is stale; run `check-native-linkage-coverage.py report`")
            action = "validated"
        print(
            f"native linkage coverage: {report['full']['entry_count']} full and "
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
