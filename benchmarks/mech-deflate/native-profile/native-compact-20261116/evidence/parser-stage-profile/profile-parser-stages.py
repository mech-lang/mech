"""Attribute the optimized parser's wall time in a separate instrumented build."""

from hashlib import file_digest
import json
import os
from pathlib import Path
import shutil
from statistics import median
import subprocess
import sys

q = Path(__file__).resolve().parent
checkout = q / "native-review-pr/checkout"
vector = q / "equivalent-vector-phases"
cache_profile = sys.argv[1:] == ["--cache"]
assert len(sys.argv) == 1 or cache_profile
output = vector / ("parser-cache-stages" if cache_profile else "parser-stages")
target = q / "host-opt-levels/opt3-cargo/release"
deps = target / "deps"
fingerprints = target / ".fingerprint"
packet = checkout / "benchmarks/mech-deflate/native-profile/native-compact-20261116"
source = packet / "program.mec"
driver = q / "parser-journal-batch-study/probe.rs"
output.mkdir()
(output / "build").mkdir()


def sha(path):
    with path.open("rb") as file:
        return file_digest(file, "sha256").hexdigest()


def dump(path, value):
    path.write_text(json.dumps(value, indent=2) + "\n")


def library(crate, code):
    pin, = (path for path in fingerprints.glob(f"*/lib-{crate}")
            if path.read_text().strip() == code.to_bytes(8, "little").hex())
    suffix = pin.parent.name.rsplit("-", 1)[1]
    file, = (path for extension in ("rlib", "dylib")
             if (path := deps / f"lib{crate}-{suffix}.{extension}").is_file())
    return file


def externs(args):
    return [part for name, path in args.items() for part in ("--extern", f"{name}={path}")]


def run(args, name):
    env = os.environ | {"CARGO_MANIFEST_DIR": str(checkout / "src/syntax"), "CARGO_PKG_VERSION": "0.3.5"}
    with (output / name).open("wb") as log:
        subprocess.run(args, env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
    return list(map(str, args))


assert sha(source) == "653597f08f889908c48610e90aa65e94a77847fb34500d0cc3a046f83cd3d299"
syntax_file, = fingerprints.glob("mech-syntax-*/lib-mech_syntax.json")
syntax = json.loads(syntax_file.read_text())
suffix = syntax_file.parent.name.rsplit("-", 1)[1]
original = deps / f"libmech_syntax-{suffix}.rlib"
crate_deps = {name: library(name, code) for _, name, _, code in syntax["deps"]}
logs = (vector / "recipe-rebuild/compiler-build.jsonl").read_text().splitlines()
rows = [json.loads(line) for line in logs if line.startswith('{"reason":')]
app = {}
native_paths = set()
for row in rows:
    native_paths.update(row.get("linked_paths", []))
    crate = row.get("target", {}).get("name")
    if crate in ("serde_json", "sha2", "mimalloc"):
        file, = (Path(file) for file in row["filenames"] if file.endswith(".rlib"))
        app[crate] = file
assert set(app) == {"serde_json", "sha2", "mimalloc"}

copy = output / "syntax"
shutil.copytree(checkout / "src/syntax/src", copy)
parser = copy / "document/parser/mod.rs"
text = parser.read_text()


def insert_after(anchor, phase):
    global text
    assert text.count(anchor) == 1, anchor
    text = text.replace(anchor, anchor + f'\n    eprintln!("parser-stage: {phase} {{}}", started.elapsed().as_nanos());'
                        '\n    let started = std::time::Instant::now();')


anchor = "    canonical::document::parse_document_root(&mut parser);\n    let output = parser.finish();"
assert text.count(anchor) == 1
text = text.replace(anchor, '''    let started = std::time::Instant::now();
    canonical::document::parse_document_root(&mut parser);
    eprintln!("parser-stage: grammar {}", started.elapsed().as_nanos());
    let started = std::time::Instant::now();
    let output = parser.finish();
    eprintln!("parser-stage: finalization-and-cache {}", started.elapsed().as_nanos());''')
anchor = "    let sink_result = sink(&output.events, &source, ids)"
assert text.count(anchor) == 1
text = text.replace(anchor, "    let started = std::time::Instant::now();\n" + anchor)
insert_after("        .unwrap_or_else(|_| fallback_tree(&source, ids, fallback_kind));", "final-tree")
anchor = "    let mut snapshot = SyntaxSnapshot::new(source, sink_result.root, diagnostics);"
assert text.count(anchor) == 1
text = text.replace(anchor, '    eprintln!("parser-stage: diagnostics {}", started.elapsed().as_nanos());\n'
                    "    let started = std::time::Instant::now();\n" + anchor)
insert_after("    snapshot.stats.new_node_count = snapshot.nodes.node_count() as u64;", "snapshot-index")
anchor = "    snapshot.restarts = build_restart_index(&snapshot);"
assert text.count(anchor) == 1
text = text.replace(anchor, anchor + '\n    eprintln!("parser-stage: restart-index {}", started.elapsed().as_nanos());')
if cache_profile:
    anchor = '    eprintln!("parser-stage: grammar {}", started.elapsed().as_nanos());'
    assert text.count(anchor) == 1
    text = text.replace(anchor, anchor + "\n    tree_cache::print_grammar_cache_profile();")
    cache_source = copy / "document/parser/tree_cache.rs"
    cache = cache_source.read_text()
    indices = (
        ("        self.queue.push_back(Request {", "        self.peak_pending = self\n", "QUEUING"),
        ("        let mut cache = core::mem::take(&mut self.state.tree_cache);",
         "        complete\n    }\n}", "MATERIALIZATION"),
    )
    for begin, end, stat in indices:
        assert cache.count(begin) == cache.count(end) == 1
        cache = cache.replace(begin, "        let started = std::time::Instant::now();\n" + begin)
        cache = cache.replace(end, f"        {stat}.with(|value| {{\n"
                              "            let (calls, elapsed) = value.get();\n"
                              "            value.set((calls + 1, elapsed + started.elapsed().as_nanos()));\n"
                              "        });\n" + end)
    cache += '''
std::thread_local! {
    static QUEUING: std::cell::Cell<(u64, u128)> = const { std::cell::Cell::new((0, 0)) };
    static MATERIALIZATION: std::cell::Cell<(u64, u128)> = const { std::cell::Cell::new((0, 0)) };
}

pub(super) fn print_grammar_cache_profile() {
    for (name, stat) in [("queue-requests", &QUEUING), ("advance-cache", &MATERIALIZATION)] {
        let (calls, nanos) = stat.with(|value| value.get());
        eprintln!("parser-cache: {name} {calls} {nanos}");
    }
}
'''
    cache_source.write_text(cache)
parser.write_text(text)

common = ["rustc", "+nightly-2026-03-03", "--edition=2024", "-C", "opt-level=3", "-C", "codegen-units=16",
          "-C", "embed-bitcode=no", "-L", f"dependency={deps}", *syntax["rustflags"]]
flags = [part for f in json.loads(syntax["features"]) for part in ("--cfg", f'feature="{f}"')]
instrumented = output / "libmech_syntax_profile.rlib"
build = run([*common, "--crate-name", "mech_syntax", "--crate-type", "rlib", "-C", "metadata=parser-stages",
             *flags, *externs(crate_deps), copy / "lib.rs", "-o", instrumented], "build/syntax.log")
links = {}
for name, lib in (("control", original), ("instrumented", instrumented)):
    native_args = [part for dir in sorted(native_paths) for part in ("-L", dir)]
    links[name] = run([*common, *native_args, *externs(app | {"mech_syntax": lib}), driver, "-o", output / name],
                      f"build/link-{name}.log")

trials = []
orders = [("control", "instrumented"), ("instrumented", "control")] * 3
for number, order in enumerate(orders, 1):
    for actor in order:
        name = f"order-{number}-{actor}"
        result = output / (name + ".json")
        command = run([output / actor, source, result], name + ".log")
        times = {}
        cache_stats = {}
        for line in (output / (name + ".log")).read_text().splitlines():
            if line.startswith("parser-stage:"):
                _, stage, nanoseconds = line.split()
                assert stage not in times
                times[stage] = int(nanoseconds) / 1e9
            if line.startswith("parser-cache:"):
                _, stage, calls, nanoseconds = line.split()
                assert stage not in cache_stats
                cache_stats[stage] = {"calls": int(calls), "seconds": int(nanoseconds) / 1e9}
        assert len(times) == (6 if actor == "instrumented" else 0), (actor, times)
        assert len(cache_stats) == (2 if cache_profile and actor == "instrumented" else 0)
        record = json.loads(result.read_text()) | {"command": command, "actor": actor, "phases_seconds": times,
                                                  "cache_within_grammar": cache_stats}
        trials.append(record)
        print(name, record["parse_seconds"], times, flush=True)
assert len({tuple(row[key] for key in ("tree_sha256", "events", "parser_steps", "source_sha256")) for row in trials}) == 1
instrumented_trials = [row for row in trials if row["actor"] == "instrumented"]
report = {
    "scope": "Apple M1 parser-only diagnostic. Both arms use the same Rust opt-level 3 and fixed parser and allocator dependencies.",
    "build_command": build, "link_commands": links, "trials": trials, "orders": orders,
    "syntax_sha256": {"unmodified": sha(original), "instrumented": sha(instrumented)},
    "phase_medians_seconds": {stage: median(row["phases_seconds"][stage] for row in instrumented_trials)
                              for stage in instrumented_trials[0]["phases_seconds"]},
    "whole_parse_medians_seconds": {actor: median(row["parse_seconds"] for row in trials if row["actor"] == actor)
                                     for actor in ("control", "instrumented")},
}
if cache_profile:
    report["grammar_cache_medians"] = {
        name: {field: median(row["cache_within_grammar"][name][field] for row in instrumented_trials)
               for field in ("seconds", "calls")}
        for name in instrumented_trials[0]["cache_within_grammar"]
    }
dump(output / "report.json", report)
print(json.dumps(report["phase_medians_seconds"], indent=2))
