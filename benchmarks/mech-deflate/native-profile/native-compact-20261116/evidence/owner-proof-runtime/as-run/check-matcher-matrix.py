"""Compare exact compression and the checked fallback on admitted matrix views."""
from hashlib import file_digest
from pathlib import Path
from statistics import median
import argparse
import json
import subprocess
from action_origin_witness import check_witness

q = Path(__file__).resolve().parent
parser = argparse.ArgumentParser()
parser.add_argument("control", type=Path)
parser.add_argument("candidate", type=Path)
parser.add_argument("output", type=Path)
parser.add_argument("--extra-shapes", action="store_true")
parser.add_argument("--source-map", type=Path)
args = parser.parse_args()
output = args.output.resolve()
output.mkdir()


def sha(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


cases = {case["name"]: case for case in json.loads((q / "fixtures.json").read_text())}
selection = {
    "eight-literals": 2, "eight-overlap": 4, "128-biased-dynamic": 2,
    "32-high-entropy-stored": 4, "random-64": 2, "random-256": 4,
    "random-1024": 2, "repeated-1024": 4, "prefix-natural-8192": 4,
    "16384-distribution-change": 4, "prefix-repeated-16384": 2,
    "prefix-periodic-65536": 4,
}
if args.extra_shapes:
    selection.update({"prefix-periodic-32769": 3, "heldout-mixed-131073": 3})
selected = []
for name, rows in selection.items():
    case = cases[name]
    assert case["input_bytes"] % rows == 0 and case["input_bytes"] // rows > 1
    selected.append({**case, "matrix_rows": rows})
matrix_cases = output / "fixtures.json"
matrix_cases.write_text(json.dumps(selected, indent=2) + "\n")

original = q / "pinned-host-link/mech-worker.rs"
source = original.read_text()


def replace(before, after):
    global source
    assert source.count(before) == 1, before
    source = source.replace(before, after)


replace('#[path = "pinned_allocator.rs"]', f'#[path = "{original.with_name("pinned_allocator.rs")}"]')
replace("    capacity: usize,\n}", "    capacity: usize,\n    matrix_rows: usize,\n}")
replace('    let prefix = std::fs::read(&case.prefix).unwrap();', '''
    let rows = case.matrix_rows;
    assert!(rows > 1 && input.len() % rows == 0);
    let columns = input.len() / rows;
    assert!(columns > 1);
    let mut matrix = vec![0; input.len()];
    for (ordinal, &byte) in input.iter().enumerate() {
        let column = ordinal / rows;
        let row = ordinal % rows;
        matrix[row * columns + column] = byte;
    }
    let matrix_alias = matrix.clone();
    let prefix = std::fs::read(&case.prefix).unwrap();''')
replace('''                    input.as_ptr() as u64,
                    input.len() as u64,
                    input.capacity() as u64,
                    1,
                    input.len() as u64,''', '''                    matrix.as_ptr() as u64,
                    matrix.len() as u64,
                    matrix.capacity() as u64,
                    rows as u64,
                    columns as u64,''')
replace("        assert_eq!(current, prefix);", """        assert_eq!(current, prefix);
        assert_eq!(matrix, matrix_alias, "borrowed matrix changed");""")
replace('''        assert_eq!(
            status.status, 0,
            "{}: compact status {} at origin {}",
            case.name, status.status, status.origin
        );''', '''        if status.status != 0 {
            assert!(returned.is_none());
            assert_eq!(output, [0; 6], "a rejected matrix published an output");
            assert_eq!(current, prefix);
            assert_eq!(current.as_ptr(), prior_pointer);
            assert_eq!(matrix, matrix_alias, "a rejected matrix changed borrowed input");
            return serde_json::json!({"case":case.name,"sample":sample,
                "seconds":seconds,"status":status.status,"origin":status.origin});
        }''')
worker_source = output / "matrix-worker.rs"
worker_source.write_text(source)


def build(actor, folder):
    folder = folder.resolve(strict=True)
    record = json.loads((folder / "runtime.json").read_text())
    worker = output / actor
    old_worker = str(folder / "mech-worker")
    command = [str(worker_source) if arg == str(original) else str(worker)
               if arg == old_worker else arg for arg in record["link"]]
    assert command.count(str(worker_source)) == command.count(str(worker)) == 1
    subprocess.run(command, check=True)
    metadata = folder / "metadata.json"
    return dict(command=[str(worker), str(metadata), str(matrix_cases)], link=command,
                object_sha256=sha(folder / "program.o"), worker_sha256=sha(worker),
                metadata_sha256=sha(metadata))


actors = dict(control=build("control", args.control), candidate=build("candidate", args.candidate))
origin_tables = {}
origin_map = {}
if args.source_map:
    origin_report = json.loads(args.source_map.read_text())
    origin_map = origin_report["source_mapping"]
    for actor, receipt in actors.items():
        origin_tables[actor] = json.loads(Path(receipt["command"][1]).read_text())["origins"]


def source_outcomes(streams):
    outcomes = {actor: {(row.get("status", 0), row.get("origin", 0)) for row in samples}
                for actor, samples in streams.items()}
    assert all(len(values) == 1 for values in outcomes.values()), outcomes
    (status, origin), = outcomes["control"]
    (new_status, new_origin), = outcomes["candidate"]
    assert status == new_status, outcomes
    matched = None
    if status and origin_map:
        old = origin_tables["control"][origin]
        current = origin_tables["candidate"][new_origin]
        if not check_witness(origin_report, args.control, args.candidate, origin, new_origin):
            offsets = {route["site"] - int(site) for site, route in origin_map.items()
                       if route is not None and "matching_run" in route
                       and route["matching_run"][0] <= origin < route["matching_run"][1]}
            if direct := origin_map.get(str(origin)):
                offsets.add(direct["site"] - origin)
            assert len(offsets) == 1 and origin + offsets.pop() == new_origin, outcomes
            assert (old["node"], old["local"]) == (current["node"], current["local"])
        matched = [old, current]
    else:
        assert origin == new_origin, outcomes
    return status, origin, new_origin, matched


results = []
logs = {}
processes = {}


def line(process, expected):
    actual = process.stdout.readline().rstrip("\n")
    assert actual == expected, (expected, actual, process.poll())


def request(process, command):
    process.stdin.write(command)
    process.stdin.flush()


try:
    for name, receipt in actors.items():
        logs[name] = (output / f"{name}.stderr").open("w")
        process = subprocess.Popen(receipt["command"], stdin=subprocess.PIPE,
                                   stdout=subprocess.PIPE, stderr=logs[name], text=True)
        processes[name] = process
        line(process, "SESSION_READY")
    for index, case in enumerate(selected):
        name = case["name"]
        samples = 20
        streams = {actor: [] for actor in actors}
        for actor, process in processes.items():
            request(process, f"C\t{index}\t{samples}\n")
            line(process, f"CASE_READY\t{name}")
        for sample in range(samples + 2):
            order = list(actors) if sample % 2 == 0 else list(reversed(actors))
            for actor in order:
                process = processes[actor]
                request(process, "N")
                observed = process.stdout.readline()
                assert observed, (name, actor, process.poll(), logs[actor].name)
                value = json.loads(observed)
                assert (value["case"], value["sample"]) == (name, sample)
                if value.get("status", 0) == 0:
                    assert all(value[key] for key in ("exact_reference", "independent_inflation",
                                                      "input_alias_unchanged", "borrowed_output_alias_unchanged"))
                    assert value["nonreturned_private_bytes_at_return"] == 0
                streams[actor].append(value)
        for process in processes.values():
            line(process, f"CASE_DONE\t{name}")
        status, origin, candidate_origin, matched = source_outcomes(streams)
        medians = {actor: median(row["seconds"] for row in rows[1:-1])
                   for actor, rows in streams.items()}
        ratio = medians["candidate"] / medians["control"]
        results.append(dict(case=name, matrix_rows=case["matrix_rows"],
                            matrix_columns=case["input_bytes"] // case["matrix_rows"],
                            status=status, origin=origin, candidate_origin=candidate_origin,
                            matched_source_origins=matched, medians=medians,
                            candidate_over_control=ratio, observations=streams))
        print(name, f"{ratio:.4f}× control matrix time", f"status={status} origin={origin}", flush=True)
    for process in processes.values():
        request(process, "Q\n")
        assert process.wait(timeout=30) == 0
finally:
    for process in processes.values():
        if process.poll() is None:
            process.terminate()
            process.wait(timeout=10)
    for log in logs.values():
        log.close()
assert all("CLEANUP private_payload_bytes=0 live_owner_records=0" in
           (output / f"{actor}.stderr").read_text() for actor in actors)
(output / "report.json").write_text(json.dumps(dict(
    fixtures_sha256=sha(matrix_cases), actors=actors, cases=results,
    exact_calls=sum(len(samples) for case in results for samples in case["observations"].values()),
), indent=2) + "\n")
