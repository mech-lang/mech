"""Measure synchronous phase durations inside the instrumented checked worker."""
import json
import statistics
import subprocess
import sys
from pathlib import Path


qualification = Path(__file__).resolve().parent
profile = qualification / "cranelift-root-phase-call-profile"
baseline = qualification / "cranelift-fused-append-backing-byte-registers-native/final-candidate"
image = qualification / "shared-huffman-packed-writer-shell-sort-keys-reachable-unmapped-native/frontend/program.mecb"
subprocess.run([
    sys.executable, "-B", qualification / "probe-literal-reuse.py",
    "--emit-only", "--native-reference", baseline,
    "--emitter", qualification / "emit_compact_with_phase_call_profile",
    profile, image, "MECH_COMPACT_PROFILE_PHASE_CALLS=1",
], check=True)

runtime = json.loads((profile / "runtime.json").read_text())
link = runtime["link"]
source, = [Path(argument) for argument in link if Path(argument).name == "mech-worker.rs"]
original = source.read_text()
allocator = source.with_name("pinned_allocator.rs")
assert allocator.is_file()
hook = "        let seconds = start.elapsed().as_secs_f64();"
assert original.count(hook) == 1
worker = original.replace('#[path = "pinned_allocator.rs"]', f"#[path = {json.dumps(str(allocator))}]")
worker = worker.replace(hook, hook + "\n        record_phase_times(sample);")
worker += r'''
type PhaseTimes = (Option<(u64, Instant)>, std::collections::BTreeMap<u64, (u64, u128)>);

thread_local! {
    static PHASE_TIMES: std::cell::RefCell<PhaseTimes> = Default::default();
}

#[unsafe(no_mangle)]
extern "C" fn mech_compact_profile_phase_call(header: u64, event: u64) {
    let now = Instant::now();
    PHASE_TIMES.with_borrow_mut(|(active, totals)| {
        if event == 0 {
            assert!(active.is_none());
            *active = Some((header, Instant::now()));
        } else {
            assert_eq!(event, 1);
            let (entry_header, start) = active.take().expect("phase entry");
            assert_eq!(entry_header, header);
            let (calls, nanoseconds) = totals.entry(header).or_default();
            *calls += 1;
            *nanoseconds += now.duration_since(start).as_nanos();
        }
    });
}

fn record_phase_times(sample: usize) {
    PHASE_TIMES.with_borrow_mut(|(active, totals)| {
        assert!(active.is_none());
        if sample != usize::MAX {
            let observation = serde_json::json!({"sample": sample, "phases": totals});
            eprintln!("PHASE_TIMES\t{observation}");
        }
        for (calls, nanoseconds) in totals.values_mut() {
            *calls = 0;
            *nanoseconds = 0;
        }
    });
}
'''
profile_worker = profile / "mech-phase-profile.rs"
profile_worker.write_text(worker)
link[link.index(str(source))] = str(profile_worker)
for previous, arg in zip(link, link[1:]):
    if previous == "--extern":
        assert Path(arg.partition("=")[2]).is_file(), arg
(profile / "instrumented-link.json").write_text(json.dumps(link, indent=2) + "\n")
subprocess.run(link, check=True)
fixtures = json.loads((qualification / "fixtures.json").read_text())
worker_args = [profile / "mech-worker", profile / "metadata.json", qualification / "fixtures.json"]
names = ["320-overlap", "prefix-random-1048576", "random-1024",
         "prefix-natural-8192", "prefix-periodic-1024"]
samples = 31
group = []
for name in names:
    index, = [index for index, fixture in enumerate(fixtures) if fixture["name"] == name]
    session = f"C\t{index}\t{samples}\n" + "N" * (samples + 2) + "Q\n"
    result = subprocess.run(worker_args, input=session, text=True, capture_output=True, check=True)
    lines = result.stdout.splitlines()
    assert lines[:2] == ["SESSION_READY", f"CASE_READY\t{name}"]
    assert lines[-1] == f"CASE_DONE\t{name}"
    checks = list(map(json.loads, lines[2:-1]))
    trace = [json.loads(line.split("\t", 1)[1]) for line in result.stderr.splitlines()
             if line.startswith("PHASE_TIMES\t")]
    assert len(checks) == len(trace) == samples + 2
    assert result.stderr.splitlines()[-1] == "CLEANUP private_payload_bytes=0 live_owner_records=0"
    for a, b in zip(checks, trace):
        assert a["sample"] == b["sample"] and a["case"] == name
        assert all(a[key] for key in ("exact_reference", "independent_inflation",
                                      "input_alias_unchanged", "borrowed_output_alias_unchanged"))
        assert a["nonreturned_private_bytes_at_return"] == 0
    middle_checks = checks[1:-1]
    middle_trace = trace[1:-1]
    duration = [x["seconds"] * 1e9 for x in middle_checks]
    headers = {key for record in middle_trace for key in record["phases"]}
    phases = {}
    for key in sorted(headers, key=int):
        calls_and_times = [record["phases"].get(key, (0, 0)) for record in middle_trace]
        calls = {c for c, _ in calls_and_times}
        assert len(calls) == 1, (name, key, calls)
        phases[key] = {
            "calls": calls.pop(),
            "median_ns": statistics.median(y for _, y in calls_and_times),
            "median_fraction": statistics.median(y / t for (_, y), t in zip(calls_and_times, duration)),
        }
    row = dict(case=name, median_instrumented_ns=statistics.median(duration), phases=phases)
    group.append(dict(**row, checked_worker=checks, trace=trace))
    print(json.dumps(row), flush=True)
output = {"boundary": "admitted calls from the checked root, timed with in-process callbacks",
          "instrumented": True, "samples_per_case": samples, "cases": group}
(profile / "phase-timings.json").write_text(json.dumps(output, indent=2) + "\n")
