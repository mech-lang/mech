"""Recompute both exclusive native trace summaries from their committed receipts."""
import gzip
from hashlib import sha256
import json
from pathlib import Path
import sys
from tempfile import TemporaryDirectory


root = Path(__file__).resolve().parent
sys.dont_write_bytecode = True
from replay import fact_delta, report


def read(path):
    return json.loads(path.read_bytes())


def check():
    original = read(root / "exclusive-replay.json")
    artifact_pins = original["unchanged_artifacts"]
    plan = read(root / "plan-inputs.json")
    for name, expected in plan["patches"].items():
        assert sha256(gzip.decompress((root / (name + ".gz")).read_bytes())).hexdigest() == expected
    results = {}
    flags = {}
    with TemporaryDirectory(prefix="mech-native-trace-replay-") as scratch:
        for role in ("old", "shared"):
            captured = root / role
            metadata = gzip.decompress((captured / "metadata.json.gz").read_bytes())
            assert sha256(metadata).hexdigest() == artifact_pins["metadata.json"]
            inputs = captured / "capture-inputs.json"
            provenance = read(inputs)
            capture = original["captures"][role]
            assert sha256(inputs.read_bytes()).hexdigest() == capture["capture_sha256"]
            assert provenance["compiler_sha256"] == capture["compiler_sha256"]
            flags[role] = dict(provenance["flags"])
            assert flags[role].pop("MECH_NATIVE_EXCLUSIVE_TRACE").endswith(f"run-{role}/native-exclusive.json")
            compile_report = captured / "compile.json"
            observed = read(compile_report)
            assert observed["source_sha256"] == provenance["source_sha256"]
            assert observed["bytecode_sha256"] == artifact_pins["program.mecb"]
            assert observed["object_sha256"] == artifact_pins["program.o"]
            instance = Path(scratch) / role
            instance.mkdir()
            (instance / "compile.json").write_bytes(compile_report.read_bytes())
            (instance / "native-exclusive.json").write_bytes(
                gzip.decompress((captured / "native-exclusive.json.gz").read_bytes())
            )
            results[role] = json.loads(json.dumps(report(instance)))
            assert results[role] == original[role]
    assert flags["old"] == flags["shared"]
    assert fact_delta(results["old"], results["shared"]) == original["materialization_delta_by_thread"]
    return {role: result["native_wall_elapsed_ns"] / 1e6 for role, result in results.items()}


if __name__ == "__main__":
    print(json.dumps({"status": "verified recorded traces and receipts", "native_wall_ms": check()}, indent=2))
