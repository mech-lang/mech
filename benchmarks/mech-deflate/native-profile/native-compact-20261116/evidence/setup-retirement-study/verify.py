"""Portable verification of every stored diagnostic batch and object binding."""
from hashlib import sha256
import importlib.util
import json
from pathlib import Path
import tarfile
import zlib


def digest(data):
    return sha256(data).hexdigest()


def main():
    here = Path(__file__).resolve().parent
    manifests = json.loads((here / "archives.json").read_text())
    data = {}
    for name, entry in manifests.items():
        path = here / name
        assert path.stat().st_size == entry["bytes"] and digest(path.read_bytes()) == entry["sha256"]
        with tarfile.open(path) as archive:
            files = {m.name: archive.extractfile(m).read() for m in archive.getmembers() if m.isfile()}
        assert set(files) == set(entry["members"])
        for name, blob in files.items():
            assert entry["members"][name] == {"sha256": digest(blob), "bytes": len(blob)}
        data[path.name] = files
    proofs = data["runtime-native-proofs.tar.gz"]
    observations = data["diagnostic-observations.tar.gz"]
    read = lambda files, name: json.loads(files[name])
    config = read(proofs, "config.json")
    build = read(proofs, "diagnostics/build.json")
    result = read(observations, "summary.json")
    assert result == json.loads((here / "summary.json").read_text())
    assert build["complete"] and result["complete"]
    assert len(result["rows"]) == 72
    assert result["plan"]["config_sha256"] == build["config_sha256"] == digest(proofs["config.json"])
    assert result["plan"]["build_sha256"] == digest(proofs["diagnostics/build.json"])
    assert digest((here / "acquire.py").read_bytes()) == result["plan"]["controller_sha256"]
    for name in ("probe.c", "probe.rs", "worker.rs", "instrument.py", "build.py"):
        assert digest((here / name).read_bytes()) == build["script_inputs"][name]
    for job in build["builds"]:
        assert digest(proofs["diagnostics/" + job["label"] + ".log"]) == job["log_sha256"]
        if job["label"].endswith("-worker"):
            command = job["command"]
            assert "link-arg=" + config["selected_object"] in command
            assert all("link-arg=" + path in command for path in config["c_objects"])

    def module(name):
        spec = importlib.util.spec_from_file_location(name, here / (name + ".py"))
        value = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(value)
        return value

    acquire, instrument = module("acquire"), module("instrument")
    generated = instrument.instrument(proofs["original-runtime/mod.rs"].decode()).encode()
    assert generated == proofs["diagnostics/compact_runtime/mod.rs"]
    assert proofs["diagnostics/compact_runtime/setup_probe.rs"] == (here / "probe.rs").read_bytes()
    assert proofs["diagnostics/worker.rs"] == (here / "worker.rs").read_bytes()
    for name in ("mod.rs", "terminal_appends.rs", "zero_pages.rs"):
        assert digest(proofs["original-runtime/" + name]) == config["inputs_sha256"][config["runtime_source"] + "/" + name]
    for name in ("terminal_appends.rs", "zero_pages.rs"):
        assert proofs["original-runtime/" + name] == proofs["diagnostics/compact_runtime/" + name]
    for name, pin in build["generated_runtime_sources"].items():
        assert digest(proofs["diagnostics/compact_runtime/" + name]) == pin
    for role, value in build["roles"].items():
        assert digest(proofs["diagnostics/" + role]) == value["worker_sha256"]
        if role == "mech-control":
            library = proofs["original-runtime/runtime.rlib"]
        else:
            library = proofs["diagnostics/" + Path(value["runtime_library"]).name]
        assert digest(library) == value["runtime_sha256"]
    assert digest(proofs["selected/program.o"]) == config["inputs_sha256"][config["selected_object"]]
    assert digest(proofs["selected/program.o"]) == "6b7a6066980489e586890372344de6ec011e06ef2c2bbb5e546ebf5ba8e0ace1"
    assert digest(proofs["selected/metadata.json"]) == "b4c0eeb760aad34e98e6733bef6b8e5db6935dc4a1df5ed5fe4ae8736fcda82f"
    assert digest(proofs["selected/program.mecb"]) == "c6529d934f020f07f8e59f13fab40927e066649e202269818ad3f009a942a8c3"
    for index, path in enumerate(config["c_objects"]):
        assert digest(proofs["libdeflate/" + str(index) + "-" + Path(path).name]) == config["inputs_sha256"][path]

    fixtures = read(observations, "fixtures-portable.json")
    oracle = {v["case"]: v for v in read(observations, "C-oracle.json")}
    inventory = {v["name"]: v for v in read(observations, "fixtures-original.json")}
    assert digest(observations["fixtures-original.json"]) == config["inputs_sha256"][config["fixtures"]]
    assert digest(observations["C-oracle.json"]) == config["inputs_sha256"][config["C_oracle"]]
    seen, checked, signature_records = set(), 0, 0
    counts = {}
    for row in result["rows"]:
        name, actor, order = row["case"], row["actor"], row["order"]
        key = name, actor, order
        assert key not in seen
        seen.add(key)
        folder = row["folder"] + "/"
        raw = observations[folder + "stdout.jsonl"]
        assert digest(raw) == row["stdout_sha256"]
        calibration, *values = [json.loads(line) for line in raw.splitlines()]
        assert len(values) == 42 and sum(v["warmup"] for v in values) == 10
        expected_hash = oracle[name]["output_sha256"] if actor == "libdeflate" else inventory[name]["expected_sha256"]
        output = observations[folder + "output.bin"]
        assert digest(output) == row["output_sha256"] == expected_hash
        fixture = fixtures[name]
        before, prefix, expected = [observations[fixture[p]["member"]] for p in ("input", "prefix", "expected")]
        assert digest(before) == fixture["input"]["sha256"] == inventory[name]["input_sha256"]
        assert digest(expected) == fixture["expected"]["sha256"] == inventory[name]["expected_sha256"]
        assert digest(prefix) == fixture["prefix"]["sha256"]
        assert output.startswith(prefix) and zlib.decompress(output[len(prefix):], -15) == before
        if actor != "libdeflate":
            assert output == expected
        for value in values:
            assert value["case"] == name and value["actor"] == actor
            assert value["exact_stable_bytes"] and value["independent_inflation"] and value["borrowed_aliases_unchanged"]
            assert all(t >= 0 for t in value["phases_ns"].values())
            assert value["phases_ns"]["total"] == sum(t for k, t in value["phases_ns"].items() if k != "total")
            if actor == "libdeflate":
                assert value["counts"]["context_allocation_requests"] == value["counts"]["context_releases"] == 1
            else:
                assert value["counts"]["nonreturned_private_bytes_at_return"] == 0
                if name in counts:
                    assert counts[name] == value["counts"]
                counts[name] = value["counts"]
            if actor in ("mech-counts", "mech-clocks"):
                assert acquire.validate_events(value) == result["event_signatures"][name]
                signature_records += 1
                if actor == "mech-counts":
                    assert all(e["total_ns"] == e["payload_ns"] == 0 for e in value["events"])
            checked += 1
        recalculated = acquire.summarize(values)
        assert all(row[k] == value for k, value in recalculated.items())
        assert row["page_bytes"] == calibration["page_bytes"] == 16384
    assert seen == {(c, a, o) for c in result["plan"]["cases"]
                    for a in ("mech-control", "mech-counts", "mech-clocks", "libdeflate")
                    for o in ("forward", "reverse")}
    receipt = {"passed": True, "batches": len(seen), "checked_saved_calls": checked,
        "validated_runtime_event_streams": signature_records, "selected_default_changed": False,
        "C_next_target": 64, "runtime_scoreboard": "81 Roc / 63 C; no new win qualification",
        "scope": "Portable raw-data, source-generation, object identity and independent-inflation checks. VM faults are observations, not physical-clearing byte counts or isolated first-touch elapsed time."}
    (here / "verification.json").write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps(receipt, indent=2))


if __name__ == "__main__":
    main()
