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
PROGRESS_INTERVAL_SECONDS = 15
CONSOLE_WRITE_LIMIT_BYTES = 16 * 1024


def mirror_console(message: str | bytes, *, error: bool = False) -> None:
    """Best-effort display must never stop deadline polling on a full pipe.

    Disk logs retain complete output. Limit each console attempt and discard
    bytes that cannot be written immediately rather than buffering a backlog.
    Restore the inherited descriptor mode for the invoking shell afterward.
    """
    descriptor = 2 if error else 1
    data = message.encode("utf-8") if isinstance(message, str) else message
    try:
        was_blocking = os.get_blocking(descriptor)
        os.set_blocking(descriptor, False)
    except OSError:
        return
    try:
        os.write(descriptor, data[:CONSOLE_WRITE_LIMIT_BYTES])
    except OSError:
        pass
    finally:
        try:
            os.set_blocking(descriptor, was_blocking)
        except OSError:
            pass


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


def cleanup_owned_group(process: subprocess.Popen) -> None:
    """Finish the owned group even after its leader has exited."""
    try:
        os.killpg(process.pid, signal.SIGKILL)
    except ProcessLookupError:
        process.wait(timeout=5)
        return
    process.wait(timeout=5)
    deadline = time.monotonic() + 5
    while True:
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise subprocess.TimeoutExpired("owned process-group cleanup", 5)
        members = subprocess.run(
            ["ps", "-axo", "pgid=,stat="], capture_output=True, text=True,
            timeout=min(1, remaining), check=True,
        )
        live = any(
            fields[0] == str(process.pid) and fields[1][0] not in "ZXx"
            for line in members.stdout.splitlines()
            if len(fields := line.split()) == 2
        )
        if not live:
            return
        time.sleep(min(0.05, max(0, deadline - time.monotonic())))


def run_stage(stage: str, command: list[str], timeout: float, log_dir: Path) -> int:
    log_dir.mkdir(parents=True, exist_ok=True)
    log = log_dir / f"{stage}.log"
    resources = log_dir / f"{stage}.resources.log"
    started = time.monotonic()
    deadline = started + timeout
    next_progress = started + PROGRESS_INTERVAL_SECONDS
    resource_snapshot(resources, stage, "before")
    reason = "exit"
    process = None
    cleaned = False
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
            mirror_console(header)
            reader.seek(len(header.encode()))
            process = subprocess.Popen(command, stdout=output, stderr=subprocess.STDOUT,
                                       start_new_session=True)
            while process.poll() is None:
                pending = reader.read(CONSOLE_WRITE_LIMIT_BYTES)
                if pending:
                    mirror_console(pending)
                now = time.monotonic()
                if now >= next_progress:
                    next_progress = now + PROGRESS_INTERVAL_SECONDS
                    mirror_console(
                        f"MECH_NATIVE_CI_PROGRESS stage={stage} elapsed_seconds={now-started:.1f}\n"
                    )
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
                    break
                time.sleep(0.1)
            cleanup_owned_group(process)
            cleaned = True
            # Display a bounded tail if output arrived faster than the console
            # mirror consumed it. The retained disk log remains complete.
            reader.seek(max(reader.tell(), os.fstat(reader.fileno()).st_size - CONSOLE_WRITE_LIMIT_BYTES))
            pending = reader.read(CONSOLE_WRITE_LIMIT_BYTES)
            if pending:
                mirror_console(pending)
            status = (128 + received_signal if received_signal is not None else 124) \
                if reason != "exit" else process.returncode
            if status < 0:
                status = 128 - status
            elapsed = time.monotonic() - started
            summary = {"stage": stage, "reason": reason, "command_exit_status": status,
                       "owned_group_cleanup": "complete",
                       "elapsed_seconds": elapsed, "timeout_seconds": timeout,
                       "cleanup_seconds": CLEANUP_SECONDS}
            output.write((json.dumps(summary) + "\n").encode())
            (log_dir / f"{stage}.summary.json").write_text(json.dumps(summary, indent=2) + "\n",
                                                        encoding="utf-8")
            mirror_console(json.dumps(summary) + "\n")
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
        mirror_console(diagnostic, error=True)
        return 1
    finally:
        if process is not None and not cleaned:
            try:
                cleanup_owned_group(process)
            except (OSError, subprocess.SubprocessError) as error:
                with log.open("ab", buffering=0) as output:
                    output.write(f"stage={stage} diagnostic=cleanup-error error={error}\n".encode())
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
            mirror_console(diagnostic, error=True)
            return 1
    return status


if __name__ == "__main__":
    raise SystemExit(main())
