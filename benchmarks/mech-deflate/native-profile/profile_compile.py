#!/usr/bin/env python3
"""Capture macOS compile phases, process time and optional CPU samples.

Usage: profile_compile.py BINARY SOURCE OUTPUT_DIRECTORY MODE [sample]
SOURCE accepts compile_profile's benchmark/fixture keywords or a source path.
Omit sample for an unprofiled timing.
"""

import argparse
import json
import re
import subprocess
import threading
import time
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary")
    parser.add_argument("source")
    parser.add_argument("output_directory", type=Path)
    parser.add_argument("mode", choices=("compact", "managed", "frontend"))
    parser.add_argument("sampling", nargs="?", choices=("sample",))
    args = parser.parse_args()
    out = args.output_directory
    out.mkdir(parents=True, exist_ok=True)
    started = time.monotonic()
    stages = []
    compiler_pid = []
    samples = 0
    sampling = args.sampling == "sample"
    sampling_error = None

    with (out / "stdout.jsonl").open("w") as stdout:
        process = subprocess.Popen(
            ["/usr/bin/time", "-lp", args.binary, args.source, str(out), args.mode],
            stdout=stdout,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1,
        )

        def read_stages():
            with (out / "stderr.log").open("w") as raw, (
                out / "stages.jsonl"
            ).open("w") as stamped:
                for line in process.stderr:
                    raw.write(line)
                    raw.flush()
                    row = {
                        "elapsed": time.monotonic() - started,
                        "line": line.rstrip(),
                    }
                    stages.append(row)
                    stamped.write(json.dumps(row) + "\n")
                    stamped.flush()
                    match = re.search(r"compile-profile: pid=(\d+)", line)
                    if match:
                        compiler_pid.append(int(match.group(1)))
                    if line.startswith("{") or any(
                        phase in line
                        for phase in (
                            "compact-stage: canonical",
                            "compact-stage: preparation",
                            "compact-stage: native module",
                        )
                    ):
                        print(json.dumps(row), flush=True)

        reader = threading.Thread(target=read_stages)
        reader.start()
        while process.poll() is None:
            if sampling and compiler_pid:
                samples += 1
                path = out / f"sample-{samples:03d}.txt"
                with (out / f"sample-{samples:03d}.log").open("w") as log:
                    result = subprocess.run(
                        [
                            "/usr/bin/sample",
                            str(compiler_pid[0]),
                            "20",
                            "5",
                            "-mayDie",
                            "-file",
                            str(path),
                        ],
                        stdout=log,
                        stderr=subprocess.STDOUT,
                    )
                if result.returncode != 0 or not path.exists():
                    # Preserve the failed attempt and finish timing compilation.
                    # Do not repeatedly retry a denied or unavailable sampler.
                    sampling = False
                    sampling_error = f"sample attempt {samples} exited {result.returncode}"
                print(
                    json.dumps(
                        {
                            "elapsed": time.monotonic() - started,
                            "sample": samples,
                            "last_stage": stages[-1]["line"] if stages else None,
                        }
                    ),
                    flush=True,
                )
            else:
                time.sleep(0.2)
        reader.join()
        status = process.wait()

    receipt = {
        "exit_code": status,
        "elapsed_seconds": time.monotonic() - started,
        "samples": samples,
        "sampling_error": sampling_error,
        "binary": args.binary,
        "source": args.source,
        "mode": args.mode,
    }
    (out / "process.json").write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps(receipt), flush=True)
    return status


if __name__ == "__main__":
    raise SystemExit(main())
