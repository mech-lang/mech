#!/usr/bin/env python3
"""Generate every native Cargo project twice and compare frozen files."""

from __future__ import annotations

import json
import os
import queue
import signal
import subprocess
import sys
import tempfile
import threading
import time
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
FILES = [
    "Cargo.lock",
    "Cargo.toml",
    "build-plan.json",
    "program.mecb",
    "rust-toolchain.toml",
    "src/main.rs",
    "src/catalog.rs",
    "src/runtime.rs",
]
EXPECTED_LAYOUT = set(FILES)
EXPECTED_ENTRIES = EXPECTED_LAYOUT | {"src"}
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
GENERATOR_TIMEOUT_SECONDS = int(
    os.environ.get("MECH_NATIVE_DETERMINISM_GENERATOR_TIMEOUT_SECS", "1800")
)
GENERATOR_HEARTBEAT_SECONDS = 30
PROJECT_MARKER = "MECH_NATIVE_PROJECT_CASE="
PROJECTS_ROOT_ENV = "MECH_NATIVE_GENERATED_PROJECTS_ROOT"
CASE_SELECTOR_ENV = "MECH_NATIVE_GENERATED_CASE"


def generate(projects_root: Path, generation: str) -> dict[str, tuple[str, Path]]:
    command = [
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
    ]
    environment = os.environ.copy()
    environment.pop(CASE_SELECTOR_ENV, None)
    environment[PROJECTS_ROOT_ENV] = str(projects_root)
    environment["CARGO_TERM_COLOR"] = "never"
    output = run_streamed(command, environment, generation)

    projects: dict[str, tuple[str, Path]] = {}
    binaries: set[str] = set()
    roots: set[Path] = set()
    for line in output:
        if PROJECT_MARKER not in line:
            continue
        marker = line.split(PROJECT_MARKER, 1)[1].strip()
        fields = marker.split("\t")
        if len(fields) != 3:
            raise RuntimeError(f"invalid generated project marker: {marker!r}")
        case, binary, raw_path = fields
        if case in projects:
            raise RuntimeError(f"generated case {case!r} was reported more than once")
        if binary in binaries:
            raise RuntimeError(f"generated binary {binary!r} was reported more than once")
        path = Path(raw_path)
        if path in roots:
            raise RuntimeError(f"generated project root {path} was reported more than once")
        expected_binary = EXPECTED_CASES.get(case)
        if expected_binary is None:
            raise RuntimeError(f"unexpected generated case {case!r}")
        if binary != expected_binary:
            raise RuntimeError(
                f"{case}: generated binary {binary!r} != {expected_binary!r}"
            )
        plan = json.loads((path / "build-plan.json").read_text(encoding="utf-8"))
        if plan["binary_name"] != binary:
            raise RuntimeError(
                f"{case}: marker binary {binary!r} != plan binary {plan['binary_name']!r}"
            )
        expected_path = projects_root / plan["plan_sha256"]
        if path != expected_path:
            raise RuntimeError(
                f"{case}: project root {path} != plan-addressed root {expected_path}"
            )
        projects[case] = (binary, path)
        binaries.add(binary)
        roots.add(path)
    actual_cases = set(projects)
    expected_cases = set(EXPECTED_CASES)
    if actual_cases != expected_cases:
        raise RuntimeError(
            "generated case identity mismatch: "
            f"missing={sorted(expected_cases - actual_cases)} "
            f"unexpected={sorted(actual_cases - expected_cases)}"
        )
    if binaries != set(EXPECTED_CASES.values()):
        raise RuntimeError("generated binary identity set did not match the fixture contract")
    return projects


def run_streamed(
    command: list[str], environment: dict[str, str], generation: str
) -> list[str]:
    creationflags = 0
    start_new_session = os.name != "nt"
    if os.name == "nt":
        creationflags = subprocess.CREATE_NEW_PROCESS_GROUP
    process = subprocess.Popen(
        command,
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
            f"{generation} generator interrupted by signal {signum}; "
            f"last progress: {last_progress}"
        )

    previous_sigterm = signal.signal(signal.SIGTERM, interrupted)
    try:
        while True:
            now = time.monotonic()
            if now >= deadline:
                raise RuntimeError(
                    f"{generation} generator exceeded {GENERATOR_TIMEOUT_SECONDS}s; "
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
                    f"native determinism progress: generation={generation} "
                    f"elapsed={now - started:.1f}s last={last_progress!r}",
                    file=sys.stderr,
                    flush=True,
                )
                next_heartbeat = now + GENERATOR_HEARTBEAT_SECONDS

        returncode = process.wait(timeout=5)
        if returncode:
            raise RuntimeError(
                f"{generation} project generation failed with exit status {returncode}; "
                f"last progress: {last_progress}"
            )
        return output
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


def snapshot(
    projects: dict[str, tuple[str, Path]],
) -> dict[str, dict[str, bytes]]:
    result: dict[str, dict[str, bytes]] = {}
    for case, (binary, project) in projects.items():
        if project.is_symlink() or not project.is_dir():
            raise RuntimeError(f"{case}: generated project root is not a real directory")
        for path in project.rglob("*"):
            if path.is_symlink():
                raise RuntimeError(f"{binary}: generated layout contains symlink {path}")
        actual = {
            str(path.relative_to(project))
            for path in project.rglob("*")
        }
        if actual != EXPECTED_ENTRIES:
            raise RuntimeError(
                f"{binary}: generated layout {sorted(actual)} != {sorted(EXPECTED_ENTRIES)}"
            )
        if not (project / "src").is_dir():
            raise RuntimeError(f"{binary}: generated src entry is not a directory")
        for name in EXPECTED_LAYOUT:
            if not (project / name).is_file():
                raise RuntimeError(f"{binary}: generated {name} is not a regular file")
        files = {name: (project / name).read_bytes() for name in FILES}
        forbidden_paths = {
            str(ROOT).encode(),
            str(ROOT.resolve()).encode(),
            str(project).encode(),
            str(project.parent).encode(),
        }
        for name, contents in files.items():
            for forbidden_path in forbidden_paths:
                if forbidden_path and forbidden_path in contents:
                    raise RuntimeError(
                        f"{binary}: {name} contains absolute path "
                        f"{forbidden_path.decode(errors='replace')!r}"
                    )
        result[case] = files
    return result


def remove_frozen_files(projects: dict[str, tuple[str, Path]]) -> None:
    """Force the second process to recreate every required project file."""

    for case, (binary, project) in projects.items():
        for name in sorted(EXPECTED_LAYOUT):
            path = project / name
            if path.is_symlink() or not path.is_file():
                raise RuntimeError(
                    f"{case}/{binary}: refusing to remove non-regular {path}"
                )
            path.unlink()


def main() -> int:
    try:
        if GENERATOR_TIMEOUT_SECONDS <= 0:
            raise RuntimeError(
                "MECH_NATIVE_DETERMINISM_GENERATOR_TIMEOUT_SECS must be positive"
            )
        exclusive_parent = ROOT / "target/mech-native/determinism"
        exclusive_parent.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(
            prefix="exclusive-", dir=exclusive_parent
        ) as temporary:
            projects_root = Path(temporary) / "projects"
            projects_root.mkdir()
            first_projects = generate(projects_root, "first")
            first = snapshot(first_projects)
            remove_frozen_files(first_projects)
            second_projects = generate(projects_root, "second")
            second = snapshot(second_projects)
            if first_projects != second_projects:
                raise RuntimeError("plan-addressed project roots changed between processes")
            if first != second:
                for case in sorted(first):
                    for name in FILES:
                        if first[case][name] != second[case][name]:
                            raise RuntimeError(
                                f"{case}: {name} changed between generations"
                            )
                raise RuntimeError("generated project bytes changed between generations")
    except (
        OSError,
        ValueError,
        KeyError,
        RuntimeError,
        subprocess.SubprocessError,
    ) as error:
        print(f"generated project determinism contract failed: {error}", file=sys.stderr)
        return 1
    print(
        "generated project determinism contract passed "
        f"({len(EXPECTED_CASES)} exact cases, 2 processes, exclusive root)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
