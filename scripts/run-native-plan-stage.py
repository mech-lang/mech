#!/usr/bin/env python3
"""Run one bounded native CI stage, retaining output and resource evidence as it runs."""

from __future__ import annotations

import argparse
import json
import os
import signal
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path


CLEANUP_SECONDS = 30
RESOURCE_INTERVAL_SECONDS = 15


def resource_snapshot(destination: Path, stage: str, event: str) -> None:
    """Unavailable counters are evidence too; do not infer an OOM from runner loss."""
    with destination.open("a", encoding="utf-8") as output:
        output.write(
            f"stage={stage} event={event} utc={datetime.now(timezone.utc).isoformat()}\n"
        )
        try:
            process_table = subprocess.run(
                (["ps", "-axo", "pid,ppid,pgid,rss,vsz,%cpu,state,command"]
                 if sys.platform == "darwin" else
                 ["ps", "-eo", "pid,ppid,pgid,rss,vsz,pcpu,stat,args", "--forest"]),
                capture_output=True, text=True, timeout=5, check=False,
            )
            output.write(f"process_tree_exit_status={process_table.returncode}\n")
            output.write(process_table.stdout)
            output.write(process_table.stderr)
        except (OSError, subprocess.TimeoutExpired) as error:
            output.write(f"process_tree_unavailable={error}\n")

        files = [Path("/proc/meminfo"), Path("/proc/pressure/memory")]
        cgroup_root = Path("/sys/fs/cgroup")
        try:
            membership = Path("/proc/self/cgroup").read_text(encoding="utf-8")
            output.write(f"cgroup_membership={membership}")
            for line in membership.splitlines():
                hierarchy, controllers, relative = line.split(":", 2)
                if hierarchy == "0" and not controllers:
                    candidate = cgroup_root / relative.lstrip("/")
                elif "memory" in controllers.split(","):
                    candidate = cgroup_root / "memory" / relative.lstrip("/")
                else:
                    continue
                if candidate.is_dir():
                    cgroup_root = candidate
                    break
        except (OSError, ValueError) as error:
            output.write(f"cgroup_membership_unavailable={error}\n")
        files.extend(cgroup_root / name for name in (
            "memory.current", "memory.peak", "memory.max", "memory.events",
            "memory.events.local", "memory.stat", "memory.pressure",
            "memory.usage_in_bytes", "memory.max_usage_in_bytes", "memory.limit_in_bytes",
            "memory.failcnt", "memory.oom_control",
        ))
        for path in files:
            try:
                output.write(f"resource_file={path}\n{path.read_text(encoding='utf-8')}\n")
            except OSError as error:
                output.write(f"resource_file={path} unavailable={error}\n")


def record_test_executable(log: Path, target: str, destination: Path) -> None:
    executables = set()
    for line in log.read_text(encoding="utf-8", errors="replace").splitlines():
        try:
            message = json.loads(line)
        except json.JSONDecodeError:
            continue
        if (message.get("reason") == "compiler-artifact"
                and message.get("target", {}).get("name") == target
                and message.get("profile", {}).get("test")
                and message.get("executable")):
            executables.add(message["executable"])
    if len(executables) != 1:
        raise ValueError(f"expected one compiled test executable for {target}, got {executables}")
    executable = Path(executables.pop())
    if not executable.is_file():
        raise ValueError(f"compiled test executable is missing: {executable}")
    destination.write_text(str(executable.resolve()) + "\n", encoding="utf-8")


def run_stage(stage: str, command: list[str], timeout: float, log_dir: Path) -> int:
    log_dir.mkdir(parents=True, exist_ok=True)
    log = log_dir / f"{stage}.log"
    resources = log_dir / f"{stage}.resources.log"
    started = time.monotonic()
    deadline = started + timeout
    next_snapshot = started + RESOURCE_INTERVAL_SECONDS
    resource_snapshot(resources, stage, "before")
    reason = "exit"
    process = None
    received_signal = None

    def interrupted(signum, _frame):
        nonlocal received_signal
        received_signal = signum

    previous_handlers = {
        signum: signal.signal(signum, interrupted)
        for signum in (signal.SIGTERM, signal.SIGINT)
    }
    try:
        with log.open("wb", buffering=0) as output, log.open("rb") as reader:
            header = (
                f"stage={stage} timeout_seconds={timeout} cleanup_seconds={CLEANUP_SECONDS} "
                f"command={command!r}\n"
            )
            output.write(header.encode())
            print(header, end="", flush=True)
            reader.seek(len(header.encode()))
            process = subprocess.Popen(command, stdout=output, stderr=subprocess.STDOUT,
                                       start_new_session=True)
            while process.poll() is None:
                pending = reader.read()
                if pending:
                    sys.stdout.buffer.write(pending)
                    sys.stdout.buffer.flush()
                now = time.monotonic()
                if now >= next_snapshot:
                    resource_snapshot(resources, stage, "running")
                    next_snapshot = now + RESOURCE_INTERVAL_SECONDS
                    print(f"MECH_NATIVE_CI_PROGRESS stage={stage} elapsed_seconds={now-started:.1f}",
                          flush=True)
                if received_signal is not None or now >= deadline:
                    reason = "interrupted" if received_signal is not None else "timeout"
                    output.write(f"stage={stage} diagnostic={reason} pid={process.pid}\n".encode())
                    resource_snapshot(resources, stage, f"before-{reason}-cleanup")
                    try:
                        os.killpg(process.pid, signal.SIGTERM)
                    except ProcessLookupError:
                        pass
                    try:
                        process.wait(timeout=CLEANUP_SECONDS)
                    except subprocess.TimeoutExpired:
                        pass
                    # Reap descendants even when their group leader exited on TERM.
                    try:
                        os.killpg(process.pid, signal.SIGKILL)
                    except ProcessLookupError:
                        pass
                    try:
                        process.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        output.write(f"stage={stage} diagnostic=process-reap-timeout pid={process.pid}\n".encode())
                    break
                time.sleep(0.1)
            pending = reader.read()
            if pending:
                sys.stdout.buffer.write(pending)
                sys.stdout.buffer.flush()
            status = (128 + received_signal if received_signal is not None else 124) \
                if reason != "exit" else process.returncode
            if status < 0:
                status = 128 - status
            elapsed = time.monotonic() - started
            summary = {"stage": stage, "reason": reason, "command_exit_status": status,
                       "elapsed_seconds": elapsed, "timeout_seconds": timeout,
                       "cleanup_seconds": CLEANUP_SECONDS}
            output.write((json.dumps(summary) + "\n").encode())
            (log_dir / f"{stage}.summary.json").write_text(json.dumps(summary, indent=2) + "\n",
                                                        encoding="utf-8")
            print(json.dumps(summary), flush=True)
            return status
    except (OSError, subprocess.SubprocessError) as error:
        diagnostic = f"stage={stage} diagnostic=stage-error error={error}\n"
        with log.open("ab", buffering=0) as output:
            output.write(diagnostic.encode())
        summary = {"stage": stage, "reason": "stage-error", "command_exit_status": 1,
                   "elapsed_seconds": time.monotonic() - started, "timeout_seconds": timeout,
                   "cleanup_seconds": CLEANUP_SECONDS}
        (log_dir / f"{stage}.summary.json").write_text(json.dumps(summary, indent=2) + "\n",
                                                    encoding="utf-8")
        print(diagnostic, end="", file=sys.stderr, flush=True)
        return 1
    finally:
        if process is not None and process.poll() is None:
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                pass
        for signum, handler in previous_handlers.items():
            signal.signal(signum, handler)
        resource_snapshot(resources, stage, "after")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--stage", required=True)
    parser.add_argument("--timeout-secs", required=True, type=float)
    parser.add_argument("--log-dir", type=Path, default=Path("target/native-plan-logs"))
    parser.add_argument("--record-test-executable")
    parser.add_argument("--test-executable-file", type=Path)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    if args.timeout_secs <= 0:
        parser.error("timeout must be positive")
    command = args.command[1:] if args.command[:1] == ["--"] else args.command
    if args.test_executable_file and not args.record_test_executable:
        command.insert(0, args.test_executable_file.read_text(encoding="utf-8").strip())
    if not command:
        parser.error("stage command is required")
    status = run_stage(args.stage, command, args.timeout_secs, args.log_dir)
    if status == 0 and args.record_test_executable:
        if not args.test_executable_file:
            parser.error("recording an executable requires --test-executable-file")
        try:
            record_test_executable(args.log_dir / f"{args.stage}.log", args.record_test_executable,
                                   args.test_executable_file)
        except (OSError, ValueError) as error:
            diagnostic = f"stage={args.stage} diagnostic=test-executable-selection error={error}\n"
            with (args.log_dir / f"{args.stage}.log").open("a", encoding="utf-8") as output:
                output.write(diagnostic)
            summary_path = args.log_dir / f"{args.stage}.summary.json"
            summary = json.loads(summary_path.read_text(encoding="utf-8"))
            summary.update(reason="test-executable-selection", command_exit_status=1)
            summary_path.write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
            print(diagnostic, end="", file=sys.stderr, flush=True)
            return 1
    return status


if __name__ == "__main__":
    raise SystemExit(main())
