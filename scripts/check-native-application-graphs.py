#!/usr/bin/env python3
"""Validate exact generated Phase 1 Mech dependency graphs with Cargo metadata."""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
EXPECTED_FEATURES = {
    "phase1_native_literal": {
        "mech-core": {"f64", "program"},
        "mech-engine": {"f64", "runtime"},
    },
    "phase1_native_scalar": {
        "mech-core": {"f64", "program"},
        "mech-engine": {"f64", "runtime"},
        "mech-math": {"add", "f64", "native-link", "runtime"},
    },
    "phase1_native_fixed_matrix": {
        "mech-core": {"f64", "matrix2", "program", "row_vector2"},
        "mech-engine": {
            "bool",
            "f64",
            "matrix2",
            "matrix_horzcat",
            "matrix_vertcat",
            "native-link",
            "row_vector2",
            "runtime",
            "vector2",
        },
        "mech-math": {"add", "f64", "matrix2", "native-link", "runtime"},
    },
    "phase1_native_dynamic_matrix": {
        "mech-core": {"f64", "matrixd", "program", "row_vectord"},
        "mech-engine": {
            "bool",
            "f64",
            "matrix_horzcat",
            "matrix_vertcat",
            "matrixd",
            "native-link",
            "row_vectord",
            "runtime",
            "vectord",
        },
        "mech-math": {"add", "f64", "matrixd", "native-link", "runtime"},
    },
    "phase1_native_variadic": {
        "mech-core": {"f64", "program", "row_vectord"},
        "mech-engine": {
            "bool",
            "f64",
            "matrix_horzcat",
            "matrixd",
            "native-link",
            "row_vectord",
            "runtime",
            "vectord",
        },
    },
    "phase1_native_cli_hosted": {
        "mech-core": {"program", "string"},
        "mech-engine": {"runtime", "string"},
        "mech-host-cli": {"provider"},
        "mech-runtime": {"runtime", "string"},
    },
}
EXPECTED = {binary: set(packages) for binary, packages in EXPECTED_FEATURES.items()}
EXPECTED_RESOLVED_FEATURES = {
    "phase1_native_literal": {
        "mech-core": {
            "byteorder",
            "crc32fast",
            "f64",
            "floats",
            "functions",
            "indexmap",
            "num-traits",
            "numbers",
            "program",
            "symbol_table",
        },
        "mech-engine": {
            "assign",
            "f64",
            "floats",
            "functions",
            "numbers",
            "program",
            "runtime",
            "symbol_table",
        },
    },
    "phase1_native_scalar": {
        "mech-core": {
            "byteorder",
            "crc32fast",
            "f64",
            "floats",
            "functions",
            "indexmap",
            "num-traits",
            "numbers",
            "program",
            "symbol_table",
        },
        "mech-engine": {
            "assign",
            "f64",
            "floats",
            "functions",
            "numbers",
            "program",
            "runtime",
            "symbol_table",
        },
        "mech-math": {
            "add",
            "f64",
            "floats",
            "math",
            "native-link",
            "numbers",
            "ops",
            "runtime",
        },
    },
    "phase1_native_fixed_matrix": {
        "mech-core": {
            "bool",
            "byteorder",
            "crc32fast",
            "f64",
            "floats",
            "functions",
            "indexmap",
            "matrix",
            "matrix2",
            "nalgebra",
            "num-traits",
            "numbers",
            "program",
            "row_vector2",
            "symbol_table",
            "vector2",
        },
        "mech-engine": {
            "assign",
            "bool",
            "f64",
            "floats",
            "functions",
            "matrix",
            "matrix2",
            "matrix_horzcat",
            "matrix_vertcat",
            "nalgebra",
            "native-link",
            "numbers",
            "program",
            "row_vector2",
            "runtime",
            "symbol_table",
            "vector2",
        },
        "mech-math": {
            "add",
            "f64",
            "floats",
            "math",
            "matrix",
            "matrix2",
            "nalgebra",
            "native-link",
            "numbers",
            "ops",
            "runtime",
        },
    },
    "phase1_native_dynamic_matrix": {
        "mech-core": {
            "bool",
            "byteorder",
            "crc32fast",
            "f64",
            "floats",
            "functions",
            "indexmap",
            "matrix",
            "matrixd",
            "nalgebra",
            "num-traits",
            "numbers",
            "program",
            "row_vectord",
            "symbol_table",
            "vectord",
        },
        "mech-engine": {
            "assign",
            "bool",
            "f64",
            "floats",
            "functions",
            "matrix",
            "matrix_horzcat",
            "matrix_vertcat",
            "matrixd",
            "nalgebra",
            "native-link",
            "numbers",
            "program",
            "row_vectord",
            "runtime",
            "symbol_table",
            "vectord",
        },
        "mech-math": {
            "add",
            "f64",
            "floats",
            "math",
            "matrix",
            "matrixd",
            "nalgebra",
            "native-link",
            "numbers",
            "ops",
            "runtime",
        },
    },
    "phase1_native_variadic": {
        "mech-core": {
            "bool",
            "byteorder",
            "crc32fast",
            "f64",
            "floats",
            "functions",
            "indexmap",
            "matrix",
            "matrixd",
            "nalgebra",
            "num-traits",
            "numbers",
            "program",
            "row_vectord",
            "symbol_table",
            "vectord",
        },
        "mech-engine": {
            "assign",
            "bool",
            "f64",
            "floats",
            "functions",
            "matrix",
            "matrix_horzcat",
            "matrix_vertcat",
            "matrixd",
            "nalgebra",
            "native-link",
            "numbers",
            "program",
            "row_vectord",
            "runtime",
            "symbol_table",
            "vectord",
        },
    },
    "phase1_native_cli_hosted": {
        "mech-core": {
            "byteorder",
            "crc32fast",
            "functions",
            "indexmap",
            "program",
            "string",
            "symbol_table",
        },
        "mech-engine": {
            "assign",
            "functions",
            "program",
            "runtime",
            "string",
            "symbol_table",
        },
        "mech-host-cli": {"provider"},
        "mech-runtime": {"runtime", "string"},
    },
}
FORBIDDEN_FEATURES = {"source", "compiler", "native-plan"}


def execute(arguments: list[str]) -> str:
    process = subprocess.run(
        arguments,
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if process.returncode:
        details = "\n".join(
            output.strip()
            for output in (process.stdout, process.stderr)
            if output.strip()
        )
        raise RuntimeError(f"{' '.join(arguments)} failed:\n{details}")
    return process.stdout


def materialize_projects() -> list[Path]:
    output = execute(
        [
            "cargo",
            "+nightly-2026-03-03",
            "test",
            "-p",
            "mech-build",
            "--features",
            "standard-hosts",
            "--test",
            "generate_phase1_projects",
            "--",
            "--nocapture",
            "--test-threads=1",
        ]
    )
    paths = []
    for line in output.splitlines():
        marker = "MECH_NATIVE_PROJECT="
        if marker in line:
            paths.append(Path(line.split(marker, 1)[1].strip()))
    if len(set(paths)) != len(EXPECTED):
        raise RuntimeError(f"expected {len(EXPECTED)} generated projects, found {len(set(paths))}")
    return sorted(set(paths))


def dependency_key(dependency: dict[str, object]) -> str:
    rename = dependency.get("rename")
    name = rename if isinstance(rename, str) else dependency.get("name")
    if not isinstance(name, str):
        raise RuntimeError("Cargo metadata contains a dependency without a name")
    return name


def optional_dependency_is_active(
    package: dict[str, object],
    node: dict[str, object],
    dependency: dict[str, object],
) -> bool:
    if not dependency.get("optional"):
        return True

    key = dependency_key(dependency)
    active_features = node.get("features")
    feature_definitions = package.get("features")
    if not isinstance(active_features, list) or not isinstance(feature_definitions, dict):
        raise RuntimeError("Cargo metadata contains malformed feature data")
    if key in active_features:
        return True

    for feature in active_features:
        if not isinstance(feature, str):
            raise RuntimeError("Cargo metadata contains a non-string feature")
        expansion = feature_definitions.get(feature, [])
        if not isinstance(expansion, list):
            raise RuntimeError("Cargo metadata contains a malformed feature expansion")
        for item in expansion:
            if not isinstance(item, str):
                raise RuntimeError("Cargo metadata contains a non-string feature expansion")
            if item == f"dep:{key}" or (
                item.startswith(f"{key}/") and not item.startswith(f"{key}?/")
            ):
                return True
    return False


def edge_is_active(
    packages: dict[str, dict[str, object]],
    node: dict[str, object],
    edge: dict[str, object],
) -> bool:
    package = packages[node["id"]]
    child = packages[edge["pkg"]]
    edge_name = edge.get("name")
    dependencies = package.get("dependencies")
    if not isinstance(edge_name, str) or not isinstance(dependencies, list):
        raise RuntimeError("Cargo metadata contains malformed dependency edges")

    candidates = [
        dependency
        for dependency in dependencies
        if isinstance(dependency, dict)
        and dependency.get("name") == child.get("name")
        and dependency_key(dependency).replace("-", "_") == edge_name
        and dependency.get("kind") != "dev"
    ]
    if not candidates:
        # Cargo has already resolved this edge. Retain unknown metadata shapes
        # conservatively rather than hiding a package from the contract check.
        return True
    return any(
        optional_dependency_is_active(package, node, dependency)
        for dependency in candidates
    )


def active_resolved_package_ids(
    metadata: dict[str, object],
    packages: dict[str, dict[str, object]],
) -> set[str]:
    resolve = metadata.get("resolve")
    if not isinstance(resolve, dict) or not isinstance(resolve.get("root"), str):
        raise RuntimeError("Cargo metadata has no resolved root")
    nodes = resolve.get("nodes")
    if not isinstance(nodes, list):
        raise RuntimeError("Cargo metadata has no resolved nodes")
    by_id = {
        node["id"]: node
        for node in nodes
        if isinstance(node, dict) and isinstance(node.get("id"), str)
    }

    active: set[str] = set()
    pending = [resolve["root"]]
    while pending:
        package_id = pending.pop()
        if package_id in active:
            continue
        node = by_id.get(package_id)
        if node is None:
            raise RuntimeError(f"Cargo metadata omits resolved node {package_id!r}")
        active.add(package_id)
        edges = node.get("deps")
        if not isinstance(edges, list):
            raise RuntimeError(f"Cargo metadata node {package_id!r} has malformed edges")
        for edge in edges:
            if not isinstance(edge, dict) or not isinstance(edge.get("pkg"), str):
                raise RuntimeError("Cargo metadata contains a malformed dependency edge")
            if edge_is_active(packages, node, edge):
                pending.append(edge["pkg"])
    return active


def validate_project(project: Path) -> str:
    plan = json.loads((project / "build-plan.json").read_text(encoding="utf-8"))
    binary = plan["binary_name"]
    if binary not in EXPECTED:
        raise RuntimeError(f"unexpected generated binary {binary!r}")
    metadata = json.loads(
        execute(
            [
                "cargo",
                "+nightly-2026-03-03",
                "metadata",
                "--format-version=1",
                "--manifest-path",
                str(project / "Cargo.toml"),
                "--locked",
                "--offline",
            ]
        )
    )
    packages = {package["id"]: package for package in metadata["packages"]}
    active_package_ids = active_resolved_package_ids(metadata, packages)
    mech_packages = {
        packages[package_id]["name"]
        for package_id in active_package_ids
        if packages[package_id]["name"].startswith("mech-")
    }
    if mech_packages != EXPECTED[binary]:
        raise RuntimeError(
            f"{binary}: Mech graph {sorted(mech_packages)} != {sorted(EXPECTED[binary])}"
        )

    root = next(
        package for package in packages.values() if package["name"] == binary
    )
    direct_packages = {dependency["name"] for dependency in root["dependencies"]}
    if direct_packages != EXPECTED[binary]:
        raise RuntimeError(
            f"{binary}: direct dependencies {sorted(direct_packages)} != "
            f"{sorted(EXPECTED[binary])}"
        )
    for dependency in root["dependencies"]:
        if dependency["uses_default_features"]:
            raise RuntimeError(f"{binary}: {dependency['name']} enables default features")
        actual_features = set(dependency["features"])
        expected_features = EXPECTED_FEATURES[binary][dependency["name"]]
        if actual_features != expected_features:
            raise RuntimeError(
                f"{binary}: {dependency['name']} declared features "
                f"{sorted(actual_features)} != {sorted(expected_features)}"
            )

    for node in metadata["resolve"]["nodes"]:
        if node["id"] not in active_package_ids:
            continue
        package = packages[node["id"]]
        if not package["name"].startswith("mech-"):
            continue
        package_name = package["name"]
        actual_resolved_features = set(node["features"])
        expected_resolved_features = EXPECTED_RESOLVED_FEATURES[binary][package_name]
        if actual_resolved_features != expected_resolved_features:
            raise RuntimeError(
                f"{binary}: {package_name} resolved features "
                f"{sorted(actual_resolved_features)} != "
                f"{sorted(expected_resolved_features)}"
            )
        forbidden = FORBIDDEN_FEATURES.intersection(actual_resolved_features)
        if forbidden:
            raise RuntimeError(
                f"{binary}: {package_name} enables forbidden features {sorted(forbidden)}"
            )
    planned = {package["package"] for package in plan["packages"]}
    if planned != EXPECTED[binary]:
        raise RuntimeError(f"{binary}: serialized plan package graph is not exact")
    for package in plan["packages"]:
        actual_features = set(package["cargo_features"])
        expected_features = EXPECTED_FEATURES[binary][package["package"]]
        if actual_features != expected_features:
            raise RuntimeError(
                f"{binary}: planned {package['package']} features "
                f"{sorted(actual_features)} != {sorted(expected_features)}"
            )
    return binary


def main() -> int:
    try:
        observed = {validate_project(path) for path in materialize_projects()}
        if observed != set(EXPECTED):
            raise RuntimeError(f"missing generated graphs: {sorted(set(EXPECTED) - observed)}")
    except (OSError, ValueError, KeyError, StopIteration, RuntimeError) as error:
        print(f"native application graph contract failed: {error}", file=sys.stderr)
        return 1
    print("native application graph contract passed (6 exact Cargo graphs)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
