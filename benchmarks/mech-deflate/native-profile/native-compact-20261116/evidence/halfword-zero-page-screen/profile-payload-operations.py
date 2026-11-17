"""Time the backing allocator and page mapping calls of the pinned benchmark."""

from hashlib import sha256
import json
from pathlib import Path
from shutil import copy2, copytree, ignore_patterns
from statistics import median
import subprocess
import sys


here = Path(__file__).resolve().parent
q = here.parents[1]
candidate = (q / sys.argv[1]).resolve(strict=True)
runtime = json.loads((candidate / "runtime.json").read_text())
link = runtime["link"]
library, = (Path(arg.split("=", 1)[1]) for arg in link if arg.startswith("mech_gpu="))
worker_source, = (Path(arg) for arg in link if arg.endswith("/mech-worker.rs"))
source = library.parent
trace = here / "observed"
assert not trace.exists()
copytree(source, trace, ignore=ignore_patterns("*.rlib", "build.json"), dirs_exist_ok=True)


def replace(contents, old, new):
    assert contents.count(old) == 1, old
    return contents.replace(old, new)


implementation = trace / "compact_runtime/mod.rs"
contents = implementation.read_text()
for old, new in (
    ("    pub scratch_poison: Option<u32>,", "    pub scratch_poison: Option<u32>,\n    pub payload_time_ns: [u64; 4],\n    pub payload_calls: [usize; 4],\n    pub payload_bytes: [usize; 4],\n    pub mapping_sizes: [usize; 16],"),
    ("        let mapping = if zeroed && stride != 1 {", "        let payload_start = std::time::Instant::now();\n        let mapping = if zeroed && stride != 1 {"),
    ("        if data.is_null() {\n            return Err(ALLOCATION);", "        let kind = usize::from(mapped);\n        self.payload_time_ns[kind] += payload_start.elapsed().as_nanos() as u64;\n        if mapped && let Some(slot) = self.mapping_sizes.get_mut(self.payload_calls[1]) { *slot = bytes; }\n        self.payload_calls[kind] += 1;\n        self.payload_bytes[kind] += bytes;\n        if data.is_null() {\n            return Err(ALLOCATION);"),
    ("        if record.is_mapped() {\n            // SAFETY: this exact mapping", "        let mapped = record.is_mapped();\n        let payload_start = std::time::Instant::now();\n        if mapped {\n            // SAFETY: this exact mapping"),
    ("        // SAFETY: this owner belongs to the heap and its payload is retired.", "        let kind = 2 + usize::from(mapped);\n        self.payload_time_ns[kind] += payload_start.elapsed().as_nanos() as u64;\n        self.payload_calls[kind] += 1;\n        self.payload_bytes[kind] += bytes;\n        // SAFETY: this owner belongs to the heap and its payload is retired."),
):
    contents = replace(contents, old, new)
implementation.write_text(contents)
entry = trace / "runtime-lib.rs"
entry.write_text(entry.read_text().replace(str(source), str(trace)))
command = ["rustc", "+nightly-2026-03-03", "--edition=2024", "--crate-name", "mech_gpu",
           "--crate-type=rlib", "-C", "opt-level=3", str(entry), "-o", str(trace / library.name)]
profile_lib = trace / library.name
assert Path(command[-1]) == profile_lib
subprocess.run(command, check=True)

code = worker_source.read_text()
code = replace(code, "        drop(heap);", "        let payload_times = heap.payload_time_ns;\n        let payload_calls = heap.payload_calls;\n        let payload_bytes = heap.payload_bytes;\n        let mapping_sizes = heap.mapping_sizes;\n        drop(heap);")
code = replace(code, '"payload_allocation_requests":payload_requests',
               '"payload_time_ns":payload_times,"payload_calls":payload_calls,"payload_bytes":payload_bytes, "mapping_sizes":mapping_sizes,"payload_allocation_requests":payload_requests')
profile_source = trace / "worker.rs"
profile_source.write_text(code)
copy2(worker_source.with_name("pinned_allocator.rs"), trace / "pinned_allocator.rs")
worker = trace / "mech-worker"
subprocess.run([
    {str(worker_source): str(profile_source), f"mech_gpu={library}": f"mech_gpu={profile_lib}",
     str(candidate / "mech-worker"): str(worker)}.get(item, item) for item in link
], check=True)

fixtures = q / "fixtures.json"
indices = {item["name"]: index for index, item in enumerate(json.loads(fixtures.read_text()))}
cases = sys.argv[2:] or ["320-overlap", "random-1024", "prefix-periodic-1024", "prefix-random-4096",
                          "prefix-random-8192", "prefix-natural-4096", "prefix-natural-8192", "prefix-random-1048576"]
def measure(executable, name):
    result = subprocess.run([executable, candidate / "metadata.json", fixtures],
                            input=f"C\t{indices[name]}\t16\n" + "W" * 2 + "N" * 18 + "Q\n",
                            capture_output=True, text=True, check=True)
    assert result.stderr.endswith("CLEANUP private_payload_bytes=0 live_owner_records=0\n")
    calls = [json.loads(line) for line in result.stdout.splitlines() if line.startswith("{")]
    selected = [row for row in calls if not row["warmup"] and not row["profile"]]
    assert len(selected) == 16
    for row in selected:
        assert row["case"] == name and not row["nonreturned_private_bytes_at_return"]
        assert all(row[key] for key in ("exact_reference", "independent_inflation", "input_alias_unchanged",
                                        "borrowed_output_alias_unchanged"))
    return selected


report = {}
for case in cases:
    baseline = measure(candidate / "mech-worker", case)
    profiled = measure(worker, case)
    counts = {tuple(row[field] for field in ("payload_allocation_requests", "requested_payload_bytes",
               "initialized_bytes_copied", "zeroed_bytes", "peak_private_payload_bytes"))
              for row in baseline + profiled}
    assert len(counts) == 1
    times = [median(row["payload_time_ns"][i] for row in profiled) for i in range(4)]
    row = {"baseline_microseconds": 1e6 * median(r["seconds"] for r in baseline),
           "instrumented_microseconds": 1e6 * median(r["seconds"] for r in profiled),
           "backing_microseconds": dict(zip(("alloc", "mmap", "dealloc", "munmap"),
                                                  (value / 1e3 for value in times), strict=True)),
           "successful_calls": len(baseline) + len(profiled)}
    row["calls"] = {"control": baseline, "profile": profiled}
    report[case] = row
    matrix = {(tuple(r['payload_calls']), tuple(r['payload_bytes']), tuple(filter(None, r['mapping_sizes']))) for r in profiled}
    assert len(matrix) == 1
    calls, sizes, mappings = matrix.pop()
    row['payload_call_counts'] = calls
    row['payload_bytes_by_operation'] = sizes
    row['individual_mapping_sizes'] = mappings
    print(case, {k:v for k,v in row.items() if k != 'calls'}, flush=True)
(trace / "timing.json").write_text(json.dumps({"object_sha256": sha256((candidate / "program.o").read_bytes()).hexdigest(),
                                                "metadata_sha256": sha256((candidate / "metadata.json").read_bytes()).hexdigest(),
                                                "ordinary_worker_sha256": sha256((candidate / "mech-worker").read_bytes()).hexdigest(),
                                                "profile_worker_sha256": sha256(worker.read_bytes()).hexdigest(),
                                                "cases": report}, indent=2) + "\n")
