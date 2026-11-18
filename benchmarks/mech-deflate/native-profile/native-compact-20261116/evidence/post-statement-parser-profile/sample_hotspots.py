"""Attribute independent stack samples to the current canonical parser body."""

from collections import Counter
from hashlib import file_digest, sha1
import json
from pathlib import Path
import platform
import re
from statistics import median
import subprocess
import sys

study = Path(__file__).resolve().parent
root = Path("/private/tmp/mech-shared-cold-epilogues.2L6obP")
qualification = root / ".qualification"
checkout = qualification / "native-review-pr/checkout"
head = "e8b2bb4f2d8c44390e9a67dfe054c6ff40341249"
packet = checkout / "benchmarks/mech-deflate/native-profile/native-compact-20261116"
source = packet / "program.mec"
source_root = Path("src/syntax/src")
control = study / "plain"
control_source = study / "pinned" / source_root
binary = study / "sampler-probe"
former = json.loads((study / "sampler-build.json").read_text())
observed = json.loads((study / "observed/00-plain.json").read_text())
expected = {"probe_sha256": former["binary"], "exact_tree_and_work": observed}
fields = ("source_sha256", "source_bytes", "tree_sha256", "parser_steps", "events")
sample_count = 4
sample_duration_seconds = 2
sample_interval_ms = 1
hot_fraction = 0.05
hot_repetitions = 3
sample_minimum = 600


def sha(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def write(path, value):
    path.write_text(json.dumps(value, indent=2) + "\n")


def verify_source(base):
    records = subprocess.check_output(
        ["git", "-C", root, "ls-tree", "-r", "-z", head, source_root]
    ).split(b"\0")
    observed = set()
    for raw in records:
        if not raw:
            continue
        object_info, filename = raw.split(b"\t", 1)
        kind, obj = object_info.split()[1:]
        assert kind == b"blob", filename
        relative = Path(filename.decode()).relative_to(source_root)
        data = (base / relative).read_bytes()
        assert sha1(b"blob " + str(len(data)).encode() + b"\0" + data).hexdigest() == obj.decode(), filename
        observed.add(relative)
    assert observed
    assert observed == {file.relative_to(base) for file in base.rglob("*") if file.is_file()}
    return len(observed)


def verify_outputs(out):
    result = json.loads(out.read_text())
    for field in fields:
        assert result[field] == expected["exact_tree_and_work"][field], field
    return {field: result[field] for field in fields}


def contains(names, *fragments):
    return any(fragment in entry for entry in names for fragment in fragments)


def family(path):
    code = tuple(frame.lower() for frame in path)
    if contains(code, "restart_index"):
        return "restart-index"
    if contains(code, "syntaxsnapshot3new", "syntaxsnapshot::new", "9nodeindex", "nodeindex::"):
        return "snapshot-index"
    if contains(code, "17definition_replay", "::definition_replay::"):
        return "structural-replay"
    if contains(code, "advance_tree_cache", "5event4sink", "event::sink"):
        if contains(code, "validate_range", "textsnapshot7byte_at", "textsnapshot::byte_at",
                    "is_char_boundary", "textsnapshot8chunk_at", "textsnapshot::chunk_at"):
            return "tree-source-access"
        if contains(code, "7journal", "parser::journal"):
            return "tree-journal-access"
        if contains(code, "mi_free", "mi_heap_", "drop_slow", "greenchildren"):
            return "tree-storage"
        if contains(code, "5event4sink", "event::sink"):
            return "final-tree"
        return "subtree-cache"
    if contains(code, "5cache7enqueue", "treecache7enqueue", "treecache::enqueue"):
        return "cache-enqueue"
    if contains(code, "7journal", "parser::journal"):
        return "grammar-journal"
    if contains(code, "11recognition", "::recognition::"):
        return "recognition-memo"
    if contains(code, "12literal_scan", "::literal_scan::"):
        return "literal-scan"
    if contains(code, "13grapheme_scan", "::grapheme_scan::"):
        return "grapheme-scan"
    if contains(code, "textsnapshot", "peek_char", "starts_with"):
        return "grammar-source-access"
    if contains(code, "6rewind", "::rewind", "10checkpoint", "::checkpoint"):
        return "grammar-transactions"
    for name in reversed(code):
        for segment, bucket in (
            ("canonical4base", "base-continuation"),
            ("canonical8literals", "literal-continuation"),
            ("canonical9operators", "operator-continuation"),
            ("recursive_core10precedence", "precedence-continuation"),
            ("canonical8document", "document-continuation"),
            ("canonical::base::", "base-continuation"),
            ("canonical::literals::", "literal-continuation"),
            ("canonical::operators::", "operator-continuation"),
            ("recursive_core::precedence::", "precedence-continuation"),
            ("canonical::document::", "document-continuation"),
        ):
            if segment in name:
                return bucket
    return "other-parser"


frame = re.compile(r"^([ !:+|]+?)(\d+) (.+?)  \(in [^)]+\)")
thread = re.compile(r"^\s+\d+ Thread[_ ]")


def parse_callgraph(lines):
    stack = []
    stages = Counter()
    leaves = Counter()
    by_family = {}

    def record():
        current = stack[-1]
        weight = current["count"] - current["children"]
        path = tuple(item["name"] for item in stack)
        stack.pop()
        if weight == 0 or not contains(path, "parse_canonical_document"):
            return
        assert weight > 0, current
        category = family(path)
        stages[category] += weight
        leaves[current["name"]] += weight
        by_family.setdefault(category, Counter())[current["name"]] += weight

    started = False
    for line in lines:
        if line.startswith("Call graph:"):
            started = True
        if not started:
            continue
        if line.startswith(("Total number in stack", "Binary Images:", "Sort by top of stack")):
            break
        if thread.match(line):
            while stack:
                record()
            continue
        match = frame.match(line)
        if match is None:
            continue
        distance, count, name = len(match[1]), int(match[2]), match[3]
        while stack and distance <= stack[-1]["distance"]:
            record()
        if stack:
            stack[-1]["children"] += count
        stack.append({"distance": distance, "count": count, "name": name, "children": 0})
    while stack:
        record()
    return sum(stages.values()), stages, leaves, by_family


def run():
    assert sha(binary) == expected["probe_sha256"]
    assert sha(source) == expected["exact_tree_and_work"]["source_sha256"]
    syntax_files = verify_source(control_source)
    out = study / "samples"
    out.mkdir()
    write(out / "plan.json", {
        "scope": "Independent 1 ms stack captures of the unmodified standalone parser with statement replay.",
        "host": {"platform": platform.platform(), "architecture": platform.machine()},
        "pin": head,
        "syntax_blob_count": syntax_files,
        "source": str(source),
        "source_sha256": sha(source),
        "probe": str(binary),
        "probe_sha256": sha(binary),
        "sample_count": sample_count,
        "sampler_lead_time_ms": 400,
        "sample_duration_seconds": sample_duration_seconds,
        "sample_interval_ms": sample_interval_ms,
        "minimum_canonical_parser_samples_per_capture": sample_minimum,
        "hot_family_fraction": hot_fraction,
        "hot_family_minimum_repetitions": hot_repetitions,
        "candidate_standalone_measurement": "Eight balanced pairs, at least 2% lower paired median and six faster pairs.",
        "outputs": expected["exact_tree_and_work"],
    })
    rows = []
    for index in range(1, sample_count + 1):
        prefix = out / f"capture-{index}"
        parse = prefix.with_suffix(".json")
        raw = prefix.with_suffix(".txt")
        probe_cmd = [str(binary), str(source), str(parse)]
        with prefix.with_suffix(".log").open("wb") as log:
            with subprocess.Popen(probe_cmd, stderr=log, stdout=log) as worker:
                sample_cmd = [
                    "/usr/bin/sample", str(worker.pid), str(sample_duration_seconds),
                    str(sample_interval_ms), "-mayDie", "-file", str(raw),
                ]
                with (out / f"sampling-{index}.log").open("wb") as sampling_log:
                    subprocess.run(sample_cmd, stdout=sampling_log, stderr=sampling_log,
                                   check=True, timeout=15)
                assert worker.wait(timeout=20) == 0, probe_cmd
        identity = verify_outputs(parse)
        total, stage, leaves, by_family = parse_callgraph(raw.read_text().splitlines())
        assert total >= sample_minimum, (raw, total)
        row = {
            "probe_command": probe_cmd, "sample_command": sample_cmd, "identity": identity,
            "sample_sha256": sha(raw), "parser_samples": total,
            "family_samples": stage, "top_leaves": leaves.most_common(20),
            "top_leaves_by_family": {name: local.most_common(10) for name, local in by_family.items()},
        }
        rows.append(row)
        print(index, total, stage.most_common(7), flush=True)
    groups = sorted({family for row in rows for family in row["family_samples"]})
    fractions = {
        name: [row["family_samples"].get(name, 0) / row["parser_samples"] for row in rows]
        for name in groups
    }
    summary = {
        name: {"fractions": fractions[name], "median": median(fractions[name]),
               "hot_repetitions": sum(value >= hot_fraction for value in fractions[name])}
        for name in sorted(groups, key=lambda key: median(fractions[key]), reverse=True)
    }
    write(out / "report.json", {
        "captures": rows, "families": summary,
        "consistent_hot_families": [name for name, result in summary.items()
                                    if result["hot_repetitions"] >= hot_repetitions],
    })
    print(json.dumps(summary, indent=2), flush=True)


if __name__ == "__main__":
    if sys.argv[1:] == ["check-source"]:
        print(verify_source(control_source), "pinned syntax source blobs")
    elif len(sys.argv) == 3 and sys.argv[1] == "classify":
        total, stage, leaves, _ = parse_callgraph(Path(sys.argv[2]).read_text().splitlines())
        print(json.dumps({"total": total, "stages": stage, "top_leaves": leaves.most_common(20)}, indent=2))
    else:
        run()
