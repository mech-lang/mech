#!/usr/bin/env python3
"""Record external kill parsing with signal zero and every kill syscall blocked."""

import argparse
import json
import os
import re
import shutil
import subprocess
from pathlib import Path


def probe_command(strace: str, executable: str, target: int, separated: bool) -> list[str]:
    return [strace, "-e", "trace=kill", "-e", "inject=kill:error=EPERM",
            executable, "-0", *(["--"] if separated else []), str(-target)]


def blocked_targets(trace: str) -> list[int]:
    calls = re.findall(r"kill\((-?\d+), 0\)\s*=\s*-1 EPERM[^\n]*\(INJECTED\)", trace)
    if not calls:
        raise ValueError("trace did not establish syscall-blocked signal-zero probing")
    return [int(target) for target in calls]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--log-dir", type=Path, default=Path("target/native-plan-logs"))
    args = parser.parse_args()
    args.log_dir.mkdir(parents=True, exist_ok=True)
    executable = shutil.which("kill")
    strace = shutil.which("strace")
    if not executable or not strace:
        raise RuntimeError("external kill and strace must be available on the Ubuntu runner")
    version = subprocess.run(
        ["dpkg-query", "-W", "-f=${binary:Package} ${Version}\n", "procps"],
        capture_output=True, text=True, check=True, timeout=10,
    ).stdout.strip()
    package = subprocess.run(
        ["dpkg-query", "-S", os.path.realpath(executable)],
        capture_output=True, text=True, check=True, timeout=10,
    ).stdout.strip()
    utility_version = subprocess.run(
        [executable, "--version"], capture_output=True, text=True,
        check=True, timeout=10,
    ).stdout.strip()
    evidence = {"executable": executable, "resolved_executable": os.path.realpath(executable),
                "executable_package": package, "utility_version": utility_version,
                "procps_package": version, "os_release": Path("/etc/os-release").read_text(),
                "image_version": os.environ.get("ImageVersion"), "probes": []}
    for target in (1_443_247, 2_345_678, 91_234):
        for separated in (False, True):
            command = probe_command(strace, executable, target, separated)
            result = subprocess.run(command, capture_output=True, text=True, timeout=10)
            trace_file = args.log_dir / f"kill-probe-{target}-{'separator' if separated else 'legacy'}.log"
            trace_file.write_text(result.stdout + result.stderr)
            observed = blocked_targets(result.stderr)
            record = {"command": command, "exit_status": result.returncode,
                      "separator": separated, "requested_target": -target,
                      "observed_targets": observed, "trace": str(trace_file)}
            evidence["probes"].append(record)
            # Retain the complete comparison even if a safe-form assertion fails.
            (args.log_dir / "kill-targeting-probe.json").write_text(json.dumps(evidence, indent=2) + "\n")
            if separated and observed != [-target]:
                raise RuntimeError(f"separator form changed the owned group target: {record}")
    print(json.dumps(evidence, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
