#!/usr/bin/env python3
"""Validate the frozen standard and full Cargo distribution graphs."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[1]
CONTRACTS = ROOT / "tests/architecture/distributions"
PROFILES = ("standard", "full")
PROFILE_ARGUMENTS = {
    "standard": [],
    "full": ["--no-default-features", "--features", "distribution-full"],
}
NATIVE_PLANNING_OWNER_COUNTS = {"standard": 5, "full": 6}
MACHINE_OPERATION_FEATURES = {
    "mech-combinatorics": {"n_choose_k"},
    "mech-compare": {"eq", "gt", "gte", "lt", "lte", "max", "min", "neq", "seq", "sneq"},
    "mech-logic": {"and", "not", "or", "xor"},
    "mech-math": {
        "abs", "acos", "acosh", "acot", "acsc", "add", "add_assign", "asec",
        "asin", "asinh", "atan", "atan2", "atanh", "cbrt", "ceil", "copysign",
        "cos", "cosh", "cot", "csc", "div", "div_assign", "erf", "erfc", "exp",
        "exp10", "exp2", "expm1", "fdim", "floor", "fmod", "hypot", "j0", "j1",
        "jn", "lgamma", "log", "log10", "log1p", "log2", "mod", "mul",
        "mul_assign", "neg", "nextafter", "pow", "remainder", "rint", "round",
        "roundeven", "sec", "sin", "sinh", "sqrt", "sub", "sub_assign", "tan",
        "tanh", "tgamma", "trunc", "y0", "y1", "yn",
    },
    "mech-matrix": {"dot", "matmul", "solve", "transpose"},
    "mech-range": {"exclusive", "exclusive_increment", "inclusive", "inclusive_increment"},
    "mech-set": {
        "cartesian_product", "difference", "disjoint", "element_of", "equals", "insert",
        "intersection", "not_element_of", "not_equals", "powerset", "proper_subset",
        "proper_superset", "remove", "size", "subset", "superset", "symmetric_difference",
        "union",
    },
    "mech-stats": {"sum"},
    "mech-string": {"concat"},
}
HOST_PACKAGES = {
    "mech-browser",
    "mech-console",
    "mech-robot-arm",
    "mech-scene",
    "mech-terminal",
    "mech-time",
    "mech-timer",
}
STANDARD_FORBIDDEN_PACKAGES = {"mech-combinatorics", "mech-robot-arm"}
STANDARD_FORBIDDEN_FEATURES = {
    "c64", "complex", "f32", "r64", "rational",
    "u8", "u16", "u32", "u64", "u128",
    "i8", "i16", "i32", "i64", "i128",
    "matrix1", "matrix2", "matrix3", "matrix4", "matrix2x3", "matrix3x2",
    "row_vector2", "row_vector3", "row_vector4",
    "vector2", "vector3", "vector4", "fixed_matrix",
    "bessel", "bessel_default", "gamma", "gamma_default", "stat_error",
    "stat_error_default", "combinatorics_default", "n_choose_k", "cartesian_product",
    "powerset", "experimental-actors", "full_operations", "full_runtime", "full_source",
    "full_compiler", "full_values", "full-hosts", "full-language", "distribution-full",
}
FULL_FORBIDDEN_MACHINE_FEATURES = {
    # Fixed storage remains available to custom builds and owner-level
    # exhaustive profiles, but is not part of either product distribution.
    "matrix1", "matrix2", "matrix3", "matrix4", "matrix2x3", "matrix3x2",
    "row_vector2", "row_vector3", "row_vector4",
    "vector2", "vector3", "vector4",
    # Machine `full_*` profiles are exhaustive owner profiles. The product
    # composes full operation families over its own dynamic-shape value set.
    "full_runtime", "full_source", "full_compiler", "full_values",
}
PACKAGE_LINE = re.compile(r"^(?P<name>\S+) v(?P<version>\S+)(?:\s|$)")
CATALOG_COUNTS_LINE = re.compile(
    r"^MECH_CATALOG_COUNTS\s+(?P<factories>\d+)\s+(?P<specializers>\d+)\s+"
    r"(?P<digest>[0-9a-f]{64})$",
    re.MULTILINE,
)


class DistributionContractError(RuntimeError):
    pass


def command(arguments: list[str]) -> str:
    process = subprocess.run(
        arguments,
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if process.returncode:
        raise DistributionContractError(
            f"command failed ({process.returncode}): {' '.join(arguments)}\n"
            f"{process.stdout}{process.stderr}"
        )
    return process.stdout


def cargo_graph(profile: str) -> tuple[dict[str, set[str]], set[str], int]:
    output = command(
        [
            "cargo", "tree", "--locked", "--offline", "-p", "mech", "-e",
            "features,no-dev",
            # Freeze the product graph rather than the runner's host-specific
            # graph. Cargo otherwise omits target-only dependencies, making a
            # contract recorded on macOS drift on Linux and Windows.
            "--target", "all", "--prefix", "none", "-f", "{p}\t{f}",
            *PROFILE_ARGUMENTS[profile],
        ]
    )
    package_features: dict[str, set[str]] = {}
    workspace_packages: set[str] = set()
    dependencies: set[str] = set()
    for line in output.splitlines():
        if "\t" not in line:
            continue
        package, raw_features = line.split("\t", 1)
        package = package.removesuffix(" (*)")
        match = PACKAGE_LINE.match(package)
        if match is None:
            continue
        name = match.group("name")
        version = match.group("version")
        dependencies.add(f"{name}@{version}")
        features = {feature for feature in raw_features.split(",") if feature}
        package_features.setdefault(name, set()).update(features)
        if str(ROOT) in package:
            workspace_packages.add(name)
    return package_features, workspace_packages, len(dependencies)


def catalog_surface(profile: str, selected_features: set[str]) -> dict[str, object]:
    if not selected_features:
        raise DistributionContractError(
            f"{profile} graph selected no mech-stdlib catalog features"
        )
    output = command(
        [
            "cargo", "test", "--locked", "--offline", "-p", "mech-stdlib",
            "--test", "profile_contracts", "--no-default-features", "--features",
            ",".join(sorted(selected_features)),
            "distribution_size_report_catalog_counts",
            "--", "--exact", "--nocapture",
        ]
    )
    match = CATALOG_COUNTS_LINE.search(output)
    if match is None:
        raise DistributionContractError(
            f"{profile} catalog probe produced no canonical surface result"
        )
    return {
        "runtime_factory_count": int(match.group("factories")),
        "source_specializer_count": int(match.group("specializers")),
        "runtime_surface_digest": match.group("digest"),
        "runtime_surface_digest_algorithm": "sha256-canonical-id-tab-name-lf-v1",
    }


def graph_surface_digest(snapshot: dict[str, object], machine_features: dict[str, list[str]]) -> str:
    payload = {
        "dependency_count": snapshot["dependency_count"],
        "native_planning_owner_count": snapshot["native_planning_owner_count"],
        "runtime_factory_count": snapshot["runtime_factory_count"],
        "selected_hosts": snapshot["selected_hosts"],
        "selected_machine_features": machine_features,
        "selected_packages": snapshot["selected_packages"],
        "selected_root_features": snapshot["selected_root_features"],
        "source_specializer_count": snapshot["source_specializer_count"],
    }
    encoded = json.dumps(payload, sort_keys=True, separators=(",", ":")).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()


def snapshot(profile: str) -> dict[str, object]:
    features, workspace_packages, dependency_count = cargo_graph(profile)
    if "mech" not in features:
        raise DistributionContractError("root mech package is absent from the selected graph")

    if profile == "standard":
        leaked_packages = sorted(workspace_packages.intersection(STANDARD_FORBIDDEN_PACKAGES))
        if leaked_packages:
            raise DistributionContractError(
                f"standard graph contains forbidden packages: {leaked_packages}"
            )
        leaked_features = {
            package: sorted(selected.intersection(STANDARD_FORBIDDEN_FEATURES))
            for package, selected in features.items()
            if package in workspace_packages and selected.intersection(STANDARD_FORBIDDEN_FEATURES)
        }
        if leaked_features:
            raise DistributionContractError(
                f"standard graph contains forbidden features: {leaked_features}"
            )
    elif profile == "full":
        leaked_features = {
            package: sorted(selected.intersection(FULL_FORBIDDEN_MACHINE_FEATURES))
            for package, selected in features.items()
            if package in MACHINE_OPERATION_FEATURES
            and selected.intersection(FULL_FORBIDDEN_MACHINE_FEATURES)
        }
        if leaked_features:
            raise DistributionContractError(
                f"full product graph contains exhaustive machine features: {leaked_features}"
            )

    machine_features = {
        package: sorted(features.get(package, set()))
        for package in sorted(MACHINE_OPERATION_FEATURES)
        if package in workspace_packages
    }
    selected_operations = {
        package: sorted(features.get(package, set()).intersection(operation_features))
        for package, operation_features in sorted(MACHINE_OPERATION_FEATURES.items())
        if package in workspace_packages
    }
    counts = catalog_surface(profile, features.get("mech-stdlib", set()))
    result: dict[str, object] = {
        "schema": "mech.distribution-contract.v1",
        "distribution": profile,
        "selected_packages": sorted(workspace_packages),
        "selected_root_features": sorted(features["mech"]),
        "selected_machine_operations": selected_operations,
        "selected_hosts": sorted(workspace_packages.intersection(HOST_PACKAGES)),
        "runtime_factory_count": counts["runtime_factory_count"],
        "source_specializer_count": counts["source_specializer_count"],
        "native_planning_owner_count": NATIVE_PLANNING_OWNER_COUNTS[profile],
        "dependency_count": dependency_count,
        "runtime_surface_digest": counts["runtime_surface_digest"],
        "runtime_surface_digest_algorithm": counts["runtime_surface_digest_algorithm"],
    }
    result["surface_digest"] = graph_surface_digest(result, machine_features)
    result["surface_digest_algorithm"] = "sha256-canonical-selected-cargo-surface-v1"
    return result


def validate(profile: str) -> None:
    path = CONTRACTS / f"{profile}.json"
    try:
        expected = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise DistributionContractError(f"cannot read {path.relative_to(ROOT)}: {error}") from error
    actual = snapshot(profile)
    if actual != expected:
        expected_text = json.dumps(expected, indent=2, sort_keys=True)
        actual_text = json.dumps(actual, indent=2, sort_keys=True)
        raise DistributionContractError(
            f"{profile} distribution contract drifted\nEXPECTED\n{expected_text}\nACTUAL\n{actual_text}"
        )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dump", choices=PROFILES, help="print the selected contract instead of validating")
    parser.add_argument(
        "--profile",
        action="append",
        choices=PROFILES,
        help="validate only this profile (repeatable; defaults to both)",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        if args.dump:
            print(json.dumps(snapshot(args.dump), indent=2, sort_keys=True))
            return 0
        for profile in args.profile or PROFILES:
            validate(profile)
            print(f"distribution contract: {profile} passed")
    except (DistributionContractError, OSError, ValueError) as error:
        print(f"distribution contract failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
