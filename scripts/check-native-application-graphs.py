#!/usr/bin/env python3
"""Validate exact generated native-application dependency graphs with Cargo metadata."""

from __future__ import annotations

import json
import os
import queue
import signal
import subprocess
import sys
import threading
import time
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
GENERATOR_TIMEOUT_SECONDS = int(
    os.environ.get("MECH_NATIVE_APPLICATION_GENERATOR_TIMEOUT_SECS", "1320")
)
METADATA_TIMEOUT_SECONDS = int(
    os.environ.get("MECH_NATIVE_APPLICATION_METADATA_TIMEOUT_SECS", "120")
)
GENERATOR_HEARTBEAT_SECONDS = 30
PROJECT_MARKER = "MECH_NATIVE_PROJECT_CASE="
EXPECTED_FEATURES = {
    "generated_native_literal": {
        "mech-core": {"f64", "program"},
        "mech-engine": {"f64", "runtime"},
        "mech-runtime": {"f64", "resident-routing", "runtime", "string"},
    },
    "generated_native_scalar": {
        "mech-core": {"f64", "program"},
        "mech-engine": {"f64", "runtime"},
        "mech-runtime": {"f64", "resident-routing", "runtime", "string"},
    },
    "generated_native_unary": {
        "mech-core": {"f64", "matrix1", "program"},
        "mech-engine": {"f64", "matrix1", "runtime"},
        "mech-runtime": {"f64", "matrix1", "resident-routing", "runtime", "string"},
    },
    "generated_native_ternary": {
        "mech-core": {"f64", "program", "row_vector4"},
        "mech-engine": {"f64", "row_vector4", "runtime"},
        "mech-runtime": {
            "f64",
            "resident-routing",
            "row_vector4",
            "runtime",
            "string",
        },
    },
    "generated_native_quaternary": {
        "mech-core": {"f64", "matrixd", "program", "row_vectord"},
        "mech-engine": {
            "bool",
            "f64",
            "matrixd",
            "row_vectord",
            "runtime",
            "vectord",
        },
        "mech-runtime": {
            "f64",
            "matrixd",
            "resident-routing",
            "row_vectord",
            "runtime",
            "string",
        },
    },
    "generated_native_variadic": {
        "mech-core": {"f64", "program", "row_vectord"},
        "mech-engine": {
            "bool",
            "f64",
            "row_vectord",
            "runtime",
            "vectord",
        },
        "mech-runtime": {
            "f64",
            "resident-routing",
            "row_vectord",
            "runtime",
            "string",
        },
    },
    "generated_native_integrity": {
        "mech-core": {"bool", "f64", "program", "string"},
        "mech-engine": {"bool", "f64", "runtime", "string"},
        "mech-runtime": {
            "bool",
            "f64",
            "resident-routing",
            "runtime",
            "string",
        },
    },
    # This closure follows from the retained artifact schemas in the fixture:
    # f64 and u8 scalars, one f32 2x3 matrix, and convert/kind. In particular,
    # these are direct generated-manifest features; matrix2x3 closes its
    # engine-internal concatenation features through mech-engine's manifest.
    "generated_native_canonical_artifact_features": {
        "mech-core": {"f32", "f64", "matrix2x3", "program", "u8"},
        "mech-engine": {"convert", "f32", "f64", "matrix2x3", "runtime", "u8"},
        "mech-runtime": {
            "f32",
            "f64",
            "matrix2x3",
            "resident-routing",
            "runtime",
            "string",
            "u8",
        },
    },
    "generated_native_fixed_matrix": {
        "mech-core": {"f64", "matrix2", "program"},
        "mech-engine": {"bool", "f64", "matrix2", "runtime", "vector2"},
        "mech-runtime": {
            "f64",
            "matrix2",
            "resident-routing",
            "runtime",
            "string",
        },
    },
    "generated_native_dynamic_matrix": {
        "mech-core": {"f64", "matrixd", "program"},
        "mech-engine": {"f64", "matrixd", "runtime"},
        "mech-runtime": {
            "f64",
            "matrixd",
            "resident-routing",
            "runtime",
            "string",
        },
    },
    "generated_native_cli": {
        "mech-core": {"program", "string"},
        "mech-engine": {"runtime", "string"},
        "mech-terminal": {"provider"},
        "mech-runtime": {"resident-routing", "runtime", "string"},
    },
    "generated_native_console": {
        "mech-core": {"program", "string"},
        "mech-engine": {"runtime", "string"},
        "mech-console": {"native"},
        "mech-runtime": {"resident-routing", "runtime", "string"},
    },
    "generated_native_time": {
        "mech-core": {"f64", "program", "string"},
        "mech-engine": {"f64", "runtime", "string"},
        "mech-time": {"native"},
        "mech-runtime": {"f64", "resident-routing", "runtime", "string"},
    },
    "generated_native_timer": {
        "mech-core": {"f64", "program", "string"},
        "mech-engine": {"f64", "runtime", "string"},
        "mech-timer": {"native"},
        "mech-runtime": {"f64", "resident-routing", "runtime", "string"},
    },
    "generated_native_scene": {
        "mech-core": {"f64", "program", "record", "string"},
        "mech-engine": {"f64", "record", "runtime", "string"},
        "mech-scene": {"native"},
        "mech-runtime": {"f64", "record", "resident-routing", "runtime", "string"},
    },
    "generated_native_robot_arm": {
        "mech-core": {"bool", "program", "string"},
        "mech-engine": {"bool", "runtime", "string"},
        "mech-robot-arm": {"provider"},
        "mech-runtime": {"bool", "resident-routing", "runtime", "string"},
    },
}
EXPECTED = {binary: set(packages) for binary, packages in EXPECTED_FEATURES.items()}
EXPECTED_CASES = {
    "literal": "generated_native_literal",
    "scalar": "generated_native_scalar",
    "unary": "generated_native_unary",
    "ternary": "generated_native_ternary",
    "quaternary": "generated_native_quaternary",
    "variadic": "generated_native_variadic",
    "integrity": "generated_native_integrity",
    "canonical-artifact-features": "generated_native_canonical_artifact_features",
    "fixed-matrix": "generated_native_fixed_matrix",
    "dynamic-matrix": "generated_native_dynamic_matrix",
    "cli": "generated_native_cli",
    "console": "generated_native_console",
    "time-once": "generated_native_time",
    "timer-once": "generated_native_timer",
    "scene": "generated_native_scene",
    "robot-arm": "generated_native_robot_arm",
}
FORBIDDEN_FEATURES = {"source", "compiler", "native-link", "native-plan"}
FORBIDDEN_PACKAGES = {"mech-stdlib", "mech-syntax", "mech-bytecode", "mech-build"}


def execute(
    arguments: list[str], environment: dict[str, str] | None = None, *, label: str
) -> str:
    if METADATA_TIMEOUT_SECONDS <= 0:
        raise RuntimeError(
            "MECH_NATIVE_APPLICATION_METADATA_TIMEOUT_SECS must be positive"
        )
    creationflags = 0
    if os.name == "nt":
        creationflags = subprocess.CREATE_NEW_PROCESS_GROUP
    process = subprocess.Popen(
        arguments,
        cwd=ROOT,
        env=environment,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        start_new_session=os.name != "nt",
        creationflags=creationflags,
    )

    def interrupted(signum: int, _frame: object) -> None:
        raise RuntimeError(f"{label} interrupted by signal {signum}")

    previous_sigterm = signal.signal(signal.SIGTERM, interrupted)
    try:
        try:
            stdout, stderr = process.communicate(timeout=METADATA_TIMEOUT_SECONDS)
        except subprocess.TimeoutExpired as error:
            raise RuntimeError(
                f"{label} exceeded {METADATA_TIMEOUT_SECONDS}s"
            ) from error
        if process.returncode:
            details = "\n".join(
                output.strip() for output in (stdout, stderr) if output.strip()
            )
            raise RuntimeError(f"{label} failed:\n{details}")
        return stdout
    except BaseException:
        terminate_process_tree(process)
        raise
    finally:
        signal.signal(signal.SIGTERM, previous_sigterm)
        if process.stdout is not None:
            process.stdout.close()
        if process.stderr is not None:
            process.stderr.close()


def execute_streamed(
    arguments: list[str], environment: dict[str, str], label: str
) -> str:
    if GENERATOR_TIMEOUT_SECONDS <= 0:
        raise RuntimeError(
            "MECH_NATIVE_APPLICATION_GENERATOR_TIMEOUT_SECS must be positive"
        )
    creationflags = 0
    start_new_session = os.name != "nt"
    if os.name == "nt":
        creationflags = subprocess.CREATE_NEW_PROCESS_GROUP
    process = subprocess.Popen(
        arguments,
        cwd=ROOT,
        env=environment,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        bufsize=1,
        start_new_session=start_new_session,
        creationflags=creationflags,
    )
    assert process.stdout is not None
    stream_end = object()
    lines: queue.Queue[object] = queue.Queue()

    def read_output() -> None:
        try:
            for line in process.stdout:
                lines.put(line)
        finally:
            lines.put(stream_end)

    reader = threading.Thread(target=read_output, daemon=True)
    reader.start()
    started = time.monotonic()
    deadline = started + GENERATOR_TIMEOUT_SECONDS
    next_heartbeat = started + GENERATOR_HEARTBEAT_SECONDS
    output: list[str] = []
    last_progress = "generator spawned"

    def interrupted(signum: int, _frame: object) -> None:
        raise RuntimeError(
            f"{label} generator interrupted by signal {signum}; "
            f"last progress: {last_progress}"
        )

    previous_sigterm = signal.signal(signal.SIGTERM, interrupted)
    try:
        while True:
            now = time.monotonic()
            if now >= deadline:
                raise RuntimeError(
                    f"{label} generator exceeded {GENERATOR_TIMEOUT_SECONDS}s; "
                    f"last progress: {last_progress}"
                )
            try:
                item = lines.get(timeout=min(0.5, deadline - now))
            except queue.Empty:
                item = None
            if item is stream_end:
                break
            if isinstance(item, str):
                output.append(item)
                print(item, end="", flush=True)
                if item.strip():
                    last_progress = item.strip()
            now = time.monotonic()
            if now >= next_heartbeat:
                print(
                    f"native application graph progress: stage={label} "
                    f"elapsed={now - started:.1f}s last={last_progress!r}",
                    file=sys.stderr,
                    flush=True,
                )
                next_heartbeat = now + GENERATOR_HEARTBEAT_SECONDS

        returncode = process.wait(timeout=5)
        if returncode:
            raise RuntimeError(
                f"{label} generator failed with exit status {returncode}; "
                f"last progress: {last_progress}"
            )
        return "".join(output)
    except BaseException:
        terminate_process_tree(process)
        raise
    finally:
        signal.signal(signal.SIGTERM, previous_sigterm)
        reader.join(timeout=5)


def terminate_process_tree(process: subprocess.Popen[str]) -> None:
    if os.name == "nt":
        subprocess.run(
            ["taskkill", "/PID", str(process.pid), "/T", "/F"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
        )
    else:
        process_groups = unix_process_groups(process.pid)
        started = time.monotonic()
        for process_group in process_groups:
            try:
                os.killpg(process_group, signal.SIGTERM)
            except ProcessLookupError:
                pass
        if process.poll() is None:
            try:
                process.wait(timeout=2)
            except subprocess.TimeoutExpired:
                pass
        remaining = 2 - (time.monotonic() - started)
        if remaining > 0:
            time.sleep(remaining)
        process_groups.update(unix_process_groups(process.pid))
        for process_group in process_groups:
            try:
                os.killpg(process_group, signal.SIGKILL)
            except ProcessLookupError:
                pass
    try:
        process.wait(timeout=5)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait(timeout=5)


def unix_process_groups(root_pid: int) -> set[int]:
    """Snapshot every process group currently owned by one descendant tree."""

    groups = {root_pid}
    process = subprocess.run(
        ["ps", "-axo", "pid=,ppid=,pgid="],
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        check=False,
    )
    if process.returncode:
        return groups
    children: dict[int, list[tuple[int, int]]] = {}
    for line in process.stdout.splitlines():
        fields = line.split()
        if len(fields) != 3:
            continue
        pid, parent, process_group = map(int, fields)
        children.setdefault(parent, []).append((pid, process_group))
    pending = [root_pid]
    visited = {root_pid}
    while pending:
        parent = pending.pop()
        for pid, process_group in children.get(parent, []):
            if pid in visited:
                continue
            visited.add(pid)
            pending.append(pid)
            groups.add(process_group)
    return groups


def materialize_projects() -> list[tuple[str, str, Path]]:
    environment = os.environ.copy()
    environment.pop("MECH_NATIVE_GENERATED_CASE", None)
    output = execute_streamed(
        [
            "cargo",
            "+nightly-2026-03-03",
            "test",
            "--locked",
            "--offline",
            "-p",
            "mech-build",
            "--features",
            "full-hosts",
            "--test",
            "generate_native_projects",
            "--",
            "--nocapture",
            "--test-threads=1",
        ],
        environment,
        "materialize",
    )
    projects: dict[str, tuple[str, Path]] = {}
    binaries: set[str] = set()
    paths: set[Path] = set()
    for line in output.splitlines():
        if PROJECT_MARKER not in line:
            continue
        marker = line.split(PROJECT_MARKER, 1)[1].strip()
        fields = marker.split("\t")
        if len(fields) != 3:
            raise RuntimeError(f"invalid generated project marker: {marker!r}")
        case, binary, raw_path = fields
        path = Path(raw_path)
        if case in projects or binary in binaries or path in paths:
            raise RuntimeError(
                f"duplicate generated project identity: case={case!r} "
                f"binary={binary!r} path={path}"
            )
        expected_binary = EXPECTED_CASES.get(case)
        if expected_binary is None or binary != expected_binary:
            raise RuntimeError(
                f"unexpected generated identity: case={case!r} binary={binary!r} "
                f"expected={expected_binary!r}"
            )
        projects[case] = (binary, path)
        binaries.add(binary)
        paths.add(path)
    if set(projects) != set(EXPECTED_CASES):
        raise RuntimeError(
            "generated application graph cases did not match the exact contract: "
            f"missing={sorted(set(EXPECTED_CASES) - set(projects))} "
            f"unexpected={sorted(set(projects) - set(EXPECTED_CASES))}"
        )
    return [
        (case, *projects[case])
        for case in sorted(projects)
    ]


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
    if plan["runtime_functions"]:
        raise RuntimeError(
            f"{binary}: canonical artifact plan retained legacy runtime installers"
        )
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
            ],
            label=f"stage=cargo metadata project={binary} path={project}",
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
    expected_direct_packages = set(EXPECTED[binary])
    if plan["live"]:
        expected_direct_packages.add("ctrlc")
    direct_packages = {dependency["name"] for dependency in root["dependencies"]}
    if direct_packages != expected_direct_packages:
        raise RuntimeError(
            f"{binary}: direct dependencies {sorted(direct_packages)} != "
            f"{sorted(expected_direct_packages)}"
        )
    for dependency in root["dependencies"]:
        if dependency["name"] == "ctrlc":
            if (
                dependency["req"] != "=3.5.2"
                or dependency["uses_default_features"]
                or dependency["features"]
            ):
                raise RuntimeError(
                    f"{binary}: ctrlc must be the exact no-default-feature =3.5.2 dependency"
                )
            continue
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
        forbidden = FORBIDDEN_FEATURES.intersection(actual_resolved_features)
        if forbidden:
            raise RuntimeError(
                f"{binary}: {package_name} enables forbidden features {sorted(forbidden)}"
            )
    forbidden_packages = FORBIDDEN_PACKAGES.intersection(mech_packages)
    if forbidden_packages:
        raise RuntimeError(
            f"{binary}: graph includes forbidden packages {sorted(forbidden_packages)}"
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
        observed = set()
        for index, (case, expected_binary, path) in enumerate(
            materialize_projects(), start=1
        ):
            profile = (
                "fixed"
                if case == "fixed-matrix"
                else "full"
                if case == "robot-arm"
                else "standard"
            )
            print(
                "native application graph progress: "
                f"case={case} profile={profile} stage=metadata "
                f"project={index}/{len(EXPECTED_CASES)} path={path}",
                file=sys.stderr,
                flush=True,
            )
            binary = validate_project(path)
            if binary != expected_binary:
                raise RuntimeError(
                    f"{case}: plan binary {binary!r} != marker binary {expected_binary!r}"
                )
            observed.add(binary)
            print(
                "native application graph progress: "
                f"case={case} profile={profile} stage=metadata status=complete",
                file=sys.stderr,
                flush=True,
            )
        if observed != set(EXPECTED):
            raise RuntimeError(f"missing generated graphs: {sorted(set(EXPECTED) - observed)}")
    except (
        OSError,
        ValueError,
        KeyError,
        StopIteration,
        RuntimeError,
        subprocess.SubprocessError,
    ) as error:
        print(f"native application graph contract failed: {error}", file=sys.stderr)
        return 1
    print(f"native application graph contract passed ({len(EXPECTED)} exact Cargo graphs)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
