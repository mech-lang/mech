"""Verify each archived observation and independently reproduce the scorecard.

Usage: python3 reconstruct.py EMPTY_SCRATCH_DIRECTORY
No compiler, benchmark, network access or additional packages are used.
"""
from pathlib import Path
import hashlib
import json
import subprocess
import sys
import tarfile

packet = Path(__file__).resolve().parent
scratch = Path(sys.argv[1]).resolve()
scratch.mkdir(parents=True, exist_ok=True)
assert not any(scratch.iterdir()), "Use an empty scratch directory"
unpacked = scratch / "unpacked"
unpacked.mkdir()
observations = 0
paths = set()
groups = []
for manifest_path in sorted(packet.rglob("manifest.json")):
    archive_path = manifest_path.with_name("receipts.tar.gz")
    assert archive_path.is_file(), archive_path
    manifest = json.loads(manifest_path.read_text())
    expected = {item["path"]: item for item in manifest["files"]}
    with tarfile.open(archive_path, "r:gz") as archive:
        members = archive.getmembers()
        assert {item.name for item in members} == set(expected), archive_path
        for item in members:
            assert item.isfile() and not Path(item.name).is_absolute()
            assert ".." not in Path(item.name).parts
            data = archive.extractfile(item).read()
            entry = expected[item.name]
            assert len(data) == entry["bytes"]
            assert hashlib.sha256(data).hexdigest() == entry["sha256"], (archive_path, item.name)
            target = unpacked / item.name
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(data)
            paths.add(item.name)
            observations += 1
    groups.append(str(archive_path.relative_to(packet)))
subprocess.run([sys.executable, str(packet / "allocation_attribution.py"), str(unpacked), str(scratch / "attribution.json")], check=True)
assert (scratch / "attribution.json").read_bytes() == (unpacked / "small-allocations/attribution.json").read_bytes()
subprocess.run([sys.executable, str(packet / "profile_analysis.py"), str(unpacked), str(scratch / "profile.json")], check=True, stdout=subprocess.DEVNULL)
assert (scratch / "profile.json").read_bytes() == (unpacked / "profile-numbers.json").read_bytes()
output = scratch / "recomputed"
subprocess.run([sys.executable, str(packet / "analyze.py"), str(unpacked), str(output)], check=True)
for name in ["scorecard.json", "scorecard.csv"]:
    assert (packet / name).read_bytes() == (output / name).read_bytes(), name
receipt = {"archive_groups": len(groups), "verified_file_occurrences": observations,
           "unique_paths": len(paths), "scorecard_json_byte_identical": True,
           "scorecard_csv_byte_identical": True, "archives": groups,
           "scope": "Each archive is hash-checked independently, preserving successive recorded versions of repeated scripts. No new performance observation, compiler execution or semantic test is claimed."}
(packet / "reconstruction.json").write_text(json.dumps(receipt, indent=2) + "\n")
print(json.dumps({key: value for key, value in receipt.items() if key != "archives"}, indent=2))
