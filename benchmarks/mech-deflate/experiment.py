#!/usr/bin/env python3
"""Archive, retrieve, replay and measure native-compression experiments.

Raw evidence belongs in the ignored .experiments directory or external storage.
Only the compact index and hash-addressed archive receipts belong in source Git.
The explicitly requested compiler-profile-20261114 bundles are a documented
exception under benchmarks/mech-deflate/evidence/; see EXPERIMENTS.md.
Historical scripts remain in the verified archives; this is the maintained entry.
"""

import argparse
import contextlib
import gzip
import hashlib
import io
import itertools
import json
import math
import os
from pathlib import Path, PurePosixPath
import select
import shutil
import statistics
import subprocess
import sys
import tarfile
import time
from urllib.parse import unquote, urlparse


RESULTS = "benchmarks/mech-deflate/results/"


def sha256(path):
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def write_json(path, value):
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def git(root, *args):
    return subprocess.check_output(["git", *args], cwd=root)


def safe_path(value):
    path = PurePosixPath(value)
    if path.is_absolute() or ".." in path.parts or str(path) != value:
        raise ValueError(f"unsafe archive path: {value}")
    return path


def pack(args):
    root = args.root.resolve()
    revision = git(root, "rev-parse", args.revision).decode().strip()
    records = []
    tracked = git(root, "ls-tree", "-r", "-z", revision).split(b"\0")
    if args.input:
        root = args.input.resolve()
        if args.output.resolve().is_relative_to(root):
            raise ValueError("archive output must be outside its input directory")
        tracked = []
        for path in sorted(root.rglob("*")):
            if path.is_symlink():
                raise ValueError(f"raw evidence must contain regular files: {path}")
            if not path.is_file():
                continue
            data = path.read_bytes()
            blob = hashlib.sha1(f"blob {len(data)}\0".encode() + data).hexdigest()
            mode = "100755" if path.stat().st_mode & 0o111 else "100644"
            tracked.append(f"{mode} blob {blob}\t{path.relative_to(root).as_posix()}".encode())
    for row in tracked:
        if not row:
            continue
        metadata, name = row.split(b"\t", 1)
        mode, kind, blob = metadata.decode().split()
        path = name.decode()
        license_file = Path(path).name.upper().startswith(("LICENSE", "NOTICE", "COPYING"))
        if not args.input and not path.startswith(RESULTS) and not license_file:
            continue
        safe_path(path)
        if kind != "blob" or mode not in {"100644", "100755"}:
            raise ValueError(f"unsupported archive entry: {mode} {path}")
        data = (root / path).read_bytes()
        actual_blob = hashlib.sha1(f"blob {len(data)}\0".encode() + data).hexdigest()
        if actual_blob != blob:
            raise ValueError(f"working file differs from {revision}: {path}")
        records.append({"path": path, "bytes": len(data), "git_blob": blob,
                        "sha256": hashlib.sha256(data).hexdigest(), "mode": mode})
    if not records or (not args.input and not any(row["path"].startswith(RESULTS) for row in records)):
        raise ValueError("no tracked historical evidence found")
    manifest = {"format": 1, "source_revision": revision,
                "evidence_root": "." if args.input else RESULTS.rstrip("/"), "files": records,
                "file_count": len(records), "bytes": sum(row["bytes"] for row in records)}
    encoded = (json.dumps(manifest, indent=2, sort_keys=True) + "\n").encode()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("xb") as output:
        with gzip.GzipFile(filename="", mode="wb", fileobj=output, mtime=0, compresslevel=1) as compressed:
            with tarfile.open(fileobj=compressed, mode="w|") as archive:
                def add(name, data, mode=0o644):
                    entry = tarfile.TarInfo(name)
                    entry.size, entry.mode, entry.mtime = len(data), mode, 0
                    archive.addfile(entry, io.BytesIO(data))
                add("MANIFEST.json", encoded)
                for row in records:
                    add(row["path"], (root / row["path"]).read_bytes(), int(row["mode"], 8) & 0o777)
    receipt = {"format": 1, "source_revision": revision,
               "archive_name": args.output.name, "bytes": args.output.stat().st_size,
               "sha256": sha256(args.output), "manifest_sha256": hashlib.sha256(encoded).hexdigest(),
               "files": len(records), "uncompressed_bytes": manifest["bytes"],
               "evidence_files": len(records) if args.input else sum(row["path"].startswith(RESULTS) for row in records)}
    write_json(args.output.with_suffix(args.output.suffix + ".json"), receipt)
    print(json.dumps(receipt, indent=2))


def verify(args):
    receipt = json.loads(args.receipt.read_text())
    if args.archive.stat().st_size != receipt["bytes"] or sha256(args.archive) != receipt["sha256"]:
        raise ValueError("archive size or SHA-256 mismatch")
    if args.extract:
        args.extract.mkdir(parents=True, exist_ok=False)
    seen, checked, total = set(), 0, 0
    with tarfile.open(args.archive, "r|gz") as archive:
        first = archive.next()
        if first is None or first.name != "MANIFEST.json" or not first.isfile():
            raise ValueError("archive must start with MANIFEST.json")
        encoded = archive.extractfile(first).read()
        if hashlib.sha256(encoded).hexdigest() != receipt["manifest_sha256"]:
            raise ValueError("manifest SHA-256 mismatch")
        manifest = json.loads(encoded)
        if manifest["source_revision"] != receipt["source_revision"]:
            raise ValueError("manifest source revision mismatch")
        expected = {row["path"]: row for row in manifest["files"]}
        if len(expected) != len(manifest["files"]):
            raise ValueError("duplicate manifest entries")
        if args.extract:
            (args.extract / "MANIFEST.json").write_bytes(encoded)
        for member in iter(archive.next, None):
            safe_path(member.name)
            if not member.isfile() or member.name not in expected or member.name in seen:
                raise ValueError(f"unexpected archive entry: {member.name}")
            row = expected[member.name]
            data = archive.extractfile(member).read()
            blob = hashlib.sha1(f"blob {len(data)}\0".encode() + data).hexdigest()
            if (len(data), hashlib.sha256(data).hexdigest(), blob) != (row["bytes"], row["sha256"], row["git_blob"]):
                raise ValueError(f"content mismatch: {member.name}")
            if member.mode != int(row["mode"], 8) & 0o777:
                raise ValueError(f"mode mismatch: {member.name}")
            if args.extract:
                target = args.extract / member.name
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(data)
                target.chmod(member.mode)
            seen.add(member.name)
            checked += 1
            total += len(data)
    if seen != set(expected) or (checked, total) != (manifest["file_count"], manifest["bytes"]):
        raise ValueError("incomplete evidence archive")
    result = {"archive_sha256": receipt["sha256"], "source_revision": receipt["source_revision"],
              "verified_files": checked, "verified_bytes": total, "all_git_blobs_match": True}
    if args.result:
        write_json(args.result, result)
    print(json.dumps(result, indent=2))


def fetch(args):
    receipt = json.loads(args.receipt.read_text())
    args.output.mkdir(parents=True, exist_ok=True)
    subprocess.run(["gh", "release", "download", receipt["release_tag"], "--repo", receipt["repository"],
                    "--pattern", receipt["archive_name"], "--dir", str(args.output)], check=True)
    args.archive = args.output / receipt["archive_name"]
    args.extract = args.result = None
    verify(args)


def replay(args):
    args.output.mkdir(parents=True, exist_ok=False)
    result = {}
    for number in args.rounds:
        matches = list(args.evidence.glob(f"{RESULTS}**/compression-round-{number:02d}/reconstruct.py"))
        if len(matches) != 1:
            raise ValueError(f"expected exactly one replay for round {number}: {matches}")
        completed = subprocess.run([sys.executable, str(matches[0].resolve()),
                                    str((args.output / f"round-{number:02d}").resolve())],
                                   capture_output=True, text=True)
        (args.output / f"round-{number:02d}.stdout").write_text(completed.stdout)
        (args.output / f"round-{number:02d}.stderr").write_text(completed.stderr)
        completed.check_returncode()
        result[str(number)] = json.loads(completed.stdout)
        print(f"round {number}: {len(result[str(number)]['byte_identical_outputs'])} byte-identical outputs", flush=True)
    write_json(args.output / "replay.json", result)


def download_packet(args, index, release_tag, asset, directory):
    if len(safe_path(asset).parts) != 1:
        raise ValueError("release asset must be a filename")
    cache = getattr(args, "packet_cache", None)
    if cache is not None:
        shutil.copyfile(cache / asset, directory / asset)
    else:
        subprocess.run(["gh", "release", "download", release_tag, "--repo", index["repository"],
                        "--pattern", asset, "--dir", str(directory)], check=True)


def replay_review_packet(args, index, packet):
    """Verify the current single-packet format before running its pinned recipe."""
    url = urlparse(packet["url"])
    prefix = f"/{index['repository']}/releases/download/"
    if url.scheme != "https" or url.netloc != "github.com" or not url.path.startswith(prefix) or url.query or url.fragment:
        raise ValueError("review packet must name this repository's release asset")
    release_tag, asset = map(unquote, url.path[len(prefix):].split("/", 1))
    safe_path(release_tag)
    recipe = packet.get("replay")
    if not isinstance(recipe, dict) or not recipe.get("script") or not recipe.get("expected_outputs"):
        raise ValueError("review packet needs a declared replay recipe and expected outputs")
    script = safe_path(recipe["script"])
    expected_outputs = recipe["expected_outputs"]
    if not expected_outputs or len(expected_outputs) != packet["reports_replayed"]:
        raise ValueError("review packet needs the complete expected report inventory")
    for name in expected_outputs:
        safe_path(name)
    downloads = args.output / "downloads"
    downloads.mkdir()
    download_packet(args, index, release_tag, asset, downloads)
    archive_path = downloads / asset
    if archive_path.stat().st_size != packet["bytes"] or sha256(archive_path) != packet["sha256"]:
        raise ValueError("archive size or SHA-256 mismatch")
    evidence = args.output / "evidence"
    evidence.mkdir()
    with tarfile.open(archive_path, "r:gz") as archive:
        members = {}
        for member in archive.getmembers():
            safe_path(member.name)
            if not member.isfile() or member.name in members:
                raise ValueError(f"unexpected archive entry: {member.name}")
            members[member.name] = member
        manifest_name = "evidence/manifest.json"
        if manifest_name not in members:
            raise ValueError("review packet lacks evidence/manifest.json")
        manifest = json.load(archive.extractfile(members[manifest_name]))
        rows = manifest["files"]
        inventory = {str(safe_path(row["path"])): row for row in rows}
        if len(inventory) != len(rows) or len(rows) != packet["files"] or "manifest.json" in inventory:
            raise ValueError("review manifest count or paths differ")
        if set(members) != {manifest_name, *("evidence/" + name for name in inventory)}:
            raise ValueError("incomplete or unexpected review manifest member")
        if str(script) not in inventory or not set(expected_outputs).issubset(inventory):
            raise ValueError("replay recipe or expected output is absent from manifest")
        for name, row in inventory.items():
            member = members["evidence/" + name]
            data = archive.extractfile(member).read()
            if len(data) != row["bytes"] or hashlib.sha256(data).hexdigest() != row["sha256"]:
                raise ValueError(f"content mismatch: {name}")
            target = evidence / name
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(data)
    # Outputs are checked against the external index after execution, even when
    # the archived recipe performs its own internal consistency checks.
    subprocess.run([sys.executable, str((evidence / script).resolve())],
                   cwd=evidence, check=True)
    for name, digest in expected_outputs.items():
        if sha256(evidence / name) != digest:
            raise ValueError(f"replayed output differs from its committed hash: {name}")
    result = {"round": args.round, "schema": "review_packet", "verified_archives": 1,
              "archive_sha256": packet["sha256"], "verified_file_occurrences": len(rows),
              "unique_paths": len(rows), "byte_identical_outputs": list(expected_outputs),
              "scope": "Complete archive and internal manifest verified; declared numerical reports replayed without compiling or compressing."}
    write_json(args.output / "replay.json", result)
    print(json.dumps(result, indent=2))


def replay_round(args):
    """Retrieve every immutable packet, verify all versions, then replay numbers."""
    index = json.loads(args.index.read_text())
    round = index["rounds"][str(args.round)]
    if ("events" in round) == ("review_packet" in round):
        raise ValueError("round must declare exactly one supported evidence schema")
    args.output.mkdir(parents=True, exist_ok=False)
    if "review_packet" in round:
        return replay_review_packet(args, index, round["review_packet"])
    release_tag = round.get("release_tag", index["release_tag"])
    downloads, evidence = args.output / "downloads", args.output / "evidence"
    downloads.mkdir()
    evidence.mkdir()
    occurrences, versions = 0, {}
    for position, event in enumerate(round["events"]):
        packet = downloads / str(position)
        packet.mkdir()
        download_packet(args, index, release_tag, event["asset"], packet)
        receipt = {**event, "archive_name": event["asset"]}
        receipt_path = packet / "receipt.json"
        write_json(receipt_path, receipt)
        extracted = packet / "extracted"
        verification = argparse.Namespace(archive=packet / event["asset"], receipt=receipt_path,
                                         extract=extracted, result=None)
        with contextlib.redirect_stdout(io.StringIO()):
            verify(verification)
        manifest = json.loads((extracted / "MANIFEST.json").read_text())
        for item in manifest["files"]:
            source = extracted / item["path"]
            target = evidence / item["path"]
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, target)
            occurrences += 1
            versions.setdefault(item["path"], set()).add(item["sha256"])
        shutil.rmtree(extracted)
        print(f"verified {event['event']}: {manifest['file_count']} files", flush=True)
    analysis = round["analysis"]
    script = evidence / safe_path(analysis["script"])
    derived = args.output / "derived"
    subprocess.run([sys.executable, str(script.resolve()), str(evidence.resolve()),
                    str(derived.resolve())], check=True)
    for name, digest in analysis["expected_outputs"].items():
        if sha256(derived / safe_path(name)) != digest:
            raise ValueError(f"replayed output differs from its committed hash: {name}")
    result = {"round": args.round, "verified_archives": len(round["events"]),
              "verified_file_occurrences": occurrences, "unique_paths": len(versions),
              "paths_with_multiple_preserved_versions": sum(len(value) > 1 for value in versions.values()),
              "byte_identical_outputs": list(analysis["expected_outputs"]),
              "scope": "Every version verified before chronological overlay; numerical replay only, without compiling or compressing."}
    write_json(args.output / "replay.json", result)
    print(json.dumps(result, indent=2))


def measure(args):
    """The existing linked-worker protocol; preparation and cleanup stay outside evaluate_into."""
    spec = json.loads(args.spec.read_text())
    args.output.mkdir(parents=True, exist_ok=False)
    if git(Path.cwd(), "status", "--porcelain").strip():
        raise ValueError("commit source/harness changes before measuring")
    active = subprocess.check_output(["ps", "-Ao", "comm="], text=True).splitlines()
    busy = {"rustc", "cargo", "tests", "gpu-unit-tests", "worker", "inspect", "roc-profile", "native_frame_transport"}
    if any(Path(name.strip()).name in busy or Path(name.strip()).name.endswith("-tests") for name in active):
        raise ValueError("another compiler, test or benchmark is active")
    for pin in spec["pins"]:
        if sha256(Path(pin["path"])) != pin["sha256"]:
            raise ValueError(f"input pin mismatch: {pin['path']}")
    labels = ("before", "after")
    groups = spec.get("groups", 4)
    blocks = spec.get("blocks", 16)
    samples = spec.get("samples", 31)
    orders = list(itertools.permutations(labels))
    write_json(args.output / "spec.json", spec)
    def record(event):
        if "record_command" in spec:
            subprocess.run([*spec["record_command"], event], check=True)
    reference = spec["reference"]
    started = time.monotonic()
    completed = subprocess.run(reference["command"], capture_output=True, text=True)
    (args.output / "reference.stdout").write_text(completed.stdout)
    (args.output / "reference.stderr").write_text(completed.stderr)
    completed.check_returncode()
    rows = [line.split("\t") for line in completed.stdout.splitlines()
            if line.startswith(reference["row_prefix"])]
    if len(rows) != reference["expected_observations"]:
        raise ValueError("reference observation count differs from the declared cohort")
    reference_seconds = statistics.median(int(row[reference["nanoseconds_column"]]) for row in rows) / 1e9
    write_json(args.output / "reference.json", {"command": reference["command"], "exit": completed.returncode,
               "seconds_outside_primary": time.monotonic() - started, "observations": len(rows),
               "median_seconds": reference_seconds})
    record("primary-reference")
    summaries = []
    for group in range(groups):
        directory = args.output / f"group{group}"
        directory.mkdir()
        workers = {}
        receipt = {"group": group, "blocks": [], "all_observations": {label: [] for label in labels}}
        def request(label):
            process, journal, _ = workers[label]
            process.stdin.write("N")
            process.stdin.flush()
            while not select.select([process.stdout], [], [], 30)[0]:
                if process.poll() is not None:
                    raise RuntimeError(f"{label} exited before its observation")
                print(f"waiting for {label} observation", flush=True)
            line = process.stdout.readline()
            journal.write(line)
            journal.flush()
            row = json.loads(line)
            for key, expected in spec["expected"][label].items():
                if row[key] != expected:
                    raise ValueError(f"{label} {key}: {row[key]} != {expected}")
            receipt["all_observations"][label].append(row)
            return row
        try:
            for label in orders[group % 2]:
                journal = (directory / f"{label}.stdout").open("w")
                error = (directory / f"{label}.stderr").open("w")
                process = subprocess.Popen(spec["commands"][label], stdin=subprocess.PIPE,
                                           stdout=subprocess.PIPE, stderr=error, text=True, bufsize=1,
                                           env={**os.environ, "MECH_BENCH_SESSION": "1"})
                workers[label] = process, journal, error
                while not select.select([process.stdout], [], [], 30)[0]:
                    if process.poll() is not None:
                        raise RuntimeError(f"{label} exited during preparation")
                    print(f"preparing {label}", flush=True)
                ready = process.stdout.readline()
                journal.write(ready)
                journal.flush()
                if ready != "SESSION_READY\n":
                    raise ValueError(f"unexpected {label} greeting: {ready}")
            for label in orders[group % 2]:
                if not request(label)["warmup"]:
                    raise ValueError("missing disclosed warmup")
            for block in range(blocks):
                values = {}
                order = orders[(block + group) % 2]
                for label in order:
                    rows = [request(label) for _ in range(samples)]
                    if any(row["warmup"] or row["profile"] for row in rows):
                        raise ValueError("nonprimary observation in primary block")
                    values[label] = statistics.median(row["seconds"] for row in rows)
                receipt["blocks"].append({"block": block, "order": order, "medians": values,
                                           "after_over_before": values["after"] / values["before"]})
                write_json(directory / "results.json", receipt)
            for label in reversed(orders[group % 2]):
                if not request(label)["profile"]:
                    raise ValueError("missing untimed profile observation")
            for label, (process, journal, error) in workers.items():
                process.stdin.close()
                if process.wait() != 0:
                    raise RuntimeError(f"{label} failed during cleanup")
                journal.close()
                error.close()
                if not (directory / f"{label}.stderr").read_text().endswith("CLEANUP bytes=0 nodes=0\n"):
                    raise ValueError("incomplete managed cleanup")
        except BaseException as failure:
            receipt["failure"] = repr(failure)
            raise
        finally:
            for process, journal, error in workers.values():
                if process.poll() is None:
                    process.terminate()
                    process.wait()
                journal.close()
                error.close()
            write_json(directory / "results.json", receipt)
        medians = {label: statistics.median(row["seconds"] for row in receipt["all_observations"][label]
                                           if not row["warmup"] and not row["profile"]) for label in labels}
        summaries.append({"group": group, "medians": medians,
                          "after_over_before": medians["after"] / medians["before"]})
        print(json.dumps(summaries[-1]), flush=True)
        record(f"primary-group{group}")
    write_json(args.output / "groups.json", summaries)
    all_rows = {label: [] for label in labels}
    ratios = []
    for group in range(groups):
        data = json.loads((args.output / f"group{group}" / "results.json").read_text())
        ratios.extend(block["after_over_before"] for block in data["blocks"])
        for label in labels:
            all_rows[label].extend(row["seconds"] for row in data["all_observations"][label]
                                   if not row["warmup"] and not row["profile"])
    medians = {label: statistics.median(all_rows[label]) for label in labels}
    summary = {"primary_observations": groups * blocks * samples * 2,
               "all_observations": groups * (blocks * samples + 2) * 2,
               "reference_seconds": reference_seconds, "medians": medians,
               "multiples_of_reference": {label: medians[label] / reference_seconds for label in labels},
               "after_over_before": medians["after"] / medians["before"],
               "improving_groups": sum(group["after_over_before"] < 1 for group in summaries),
               "improving_blocks": sum(ratio < 1 for ratio in ratios),
               "adverse_blocks": sum(ratio > 1 for ratio in ratios),
               "median_block_ratio": statistics.median(ratios),
               "geometric_block_ratio": math.exp(statistics.mean(math.log(ratio) for ratio in ratios)),
               "scope": "Blocks within a prepared pair are descriptive, not independent experiments. All adverse observations retained."}
    write_json(args.output / "summary.json", summary)
    record("primary-summary")
    print(json.dumps(summary, indent=2), flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    command = commands.add_parser("pack", help="archive tracked evidence and licenses at an exact source revision")
    command.add_argument("--root", type=Path, default=Path.cwd())
    command.add_argument("--revision", default="HEAD")
    command.add_argument("--input", type=Path, help="seal a new raw packet instead of tracked historical evidence")
    command.add_argument("--output", type=Path, required=True)
    command.set_defaults(run=pack)
    command = commands.add_parser("verify", help="verify every file, Git blob and archive hash; optionally extract")
    command.add_argument("archive", type=Path)
    command.add_argument("--receipt", type=Path, required=True)
    command.add_argument("--extract", type=Path)
    command.add_argument("--result", type=Path)
    command.set_defaults(run=verify)
    command = commands.add_parser("fetch", help="retrieve the pinned external release asset and verify it")
    command.add_argument("--receipt", type=Path, required=True)
    command.add_argument("--output", type=Path, required=True)
    command.set_defaults(run=fetch)
    command = commands.add_parser("replay", help="reconstruct historical packets without compiling or compressing")
    command.add_argument("evidence", type=Path)
    command.add_argument("--rounds", type=int, nargs="+", default=[76, 77, 78])
    command.add_argument("--output", type=Path, required=True)
    command.set_defaults(run=replay)
    command = commands.add_parser("replay-round", help="retrieve and reconstruct a hash-indexed external experiment")
    command.add_argument("--index", type=Path, required=True)
    command.add_argument("--round", type=int, required=True)
    command.add_argument("--packet-cache", type=Path, help="use local release assets; all integrity checks still apply")
    command.add_argument("--output", type=Path, required=True)
    command.set_defaults(run=replay_round)
    command = commands.add_parser("measure", help="run balanced linked workers with all observations retained")
    command.add_argument("--spec", type=Path, required=True)
    command.add_argument("--output", type=Path, required=True)
    command.set_defaults(run=measure)
    args = parser.parse_args()
    args.run(args)


if __name__ == "__main__":
    main()
