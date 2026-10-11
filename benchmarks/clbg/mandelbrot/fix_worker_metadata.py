#!/usr/bin/env python3
"""Correct source worker labels without changing commands or measured samples."""
import argparse
import hashlib
import json
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("report", type=Path)
    args = parser.parse_args()
    report = json.loads(args.report.read_text())
    if report.get("metadata_corrections"): raise SystemExit("already corrected")
    corrections = []
    for row in report["programs"]:
        language, variant = row["language"], row.get("variant")
        label = "serial" if language == "cpp" and variant == 2 else f"OpenMP threads: {report['protocol']['requested_threads']}" if language == "c" and variant == 9 else None
        if label and label != row["worker_policy"]:
            corrections.append({"id": row["id"], "before": row["worker_policy"], "after": label,
                                "reason": "Pinned C++ #2 has no parallel region; C #9 uses two OpenMP parallel loops. Source, build flags, invocations and timing samples are unchanged."})
            row["worker_policy"] = label
    if not corrections:
        print("Worker labels already correct")
        return
    raw = args.report.with_suffix(".raw.json")
    if raw.exists(): raise SystemExit("raw report already exists")
    raw.write_bytes(args.report.read_bytes())
    report["metadata_corrections"] = corrections
    report["uncorrected_report"] = {"path": raw.name, "sha256": hashlib.sha256(raw.read_bytes()).hexdigest()}
    args.report.write_text(json.dumps(report, indent=2) + "\n")
    print(f"Corrected {len(corrections)} worker labels; untouched original retained as {raw.name}")


if __name__ == "__main__": main()
