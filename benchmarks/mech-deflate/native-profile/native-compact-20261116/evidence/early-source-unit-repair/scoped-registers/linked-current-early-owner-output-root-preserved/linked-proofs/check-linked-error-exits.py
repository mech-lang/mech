"""Interleave scratch workers with the same final-byte allocation failure."""
from pathlib import Path
from statistics import median
import json
import os
import subprocess
import sys
from action_origin_witness import check_witness

q = Path('/private/tmp/mech-shared-cold-epilogues.2L6obP/.qualification')
fixtures = q / 'fixtures.json'
inputs = json.loads(fixtures.read_text())
link_root = Path('/private/tmp/mech-shared-cold-epilogues.2L6obP/.qualification/consolidation-995/controller-source-boundaries/scoped-registers/linked-current-early-owner-output-root-preserved/linked-proofs/failure-support')
allocator_source = (link_root / 'pinned_allocator.rs').read_text()
allocator_source += '''
use std::sync::atomic::{AtomicUsize, Ordering};
pub static FAIL_CAPACITY: AtomicUsize = AtomicUsize::new(0);
'''
old = '    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {\n'
assert allocator_source.count(old) == 1
allocator_source = allocator_source.replace(old, old + '''        if layout.align() == 1 && FAIL_CAPACITY.compare_exchange(
            layout.size(), 0, Ordering::Relaxed, Ordering::Relaxed,
        ).is_ok() {
            return std::ptr::null_mut();
        }
''')
(link_root / 'checked-exit-failure-allocator.rs').write_text(allocator_source)
source = (link_root / 'mech-worker.rs').read_text()
source = source.replace('"pinned_allocator.rs"', '"checked-exit-failure-allocator.rs"')
source = source.replace('    let mut execute=|sample:usize| {', '    let mut execute=|_sample:usize| {')
source = source.replace('        let status = unsafe {', '''        use std::sync::atomic::Ordering;
        pinned_allocator::FAIL_CAPACITY.store(case.capacity, Ordering::Relaxed);
        let status = unsafe {''')
start = source.index('        let returned = if status.status == 0 {')
end = source.index('\n    };', start)
source = source[:start] + '''        let seconds = start.elapsed().as_secs_f64();
        let remaining = pinned_allocator::FAIL_CAPACITY.swap(0, Ordering::Relaxed);
        assert_eq!(remaining, 0, "{}: allocation failure was not reached", case.name);
        assert_ne!(status.status, 0, "{}: allocation failure was lost", case.name);
        assert_eq!(current, prefix);
        assert_eq!(input_alias, input);
        assert_eq!(current.as_ptr(), prior_pointer);
        assert_eq!(output, [0; 6], "failed call published output");
        let requests = heap.counters.payload_requests;
        let bytes = heap.counters.requested_payload_bytes;
        let live_at_failure = heap.counters.live_payload_bytes;
        drop(heap);
        serde_json::json!({"case": case.name, "status": status.status, "origin": status.origin,
                          "requests": requests, "bytes": bytes,
                          "live_at_failure": live_at_failure, "seconds": seconds})''' + source[end:]
source = source.replace('        let input_alias = &input[..];', '        let input_alias = input.clone();')
source = source.replace('        assert_eq!(input_alias, input);',
                        '        assert_eq!(input_alias, input, "failure changed borrowed input");')
worker_source = link_root / 'checked-exit-failure-worker.rs'
worker_source.write_text(source)

names = sys.argv[1:]
assert len(names) >= 2
source_map = os.environ.get('MECH_ERROR_SOURCE_MAP')
if source_map:
    assert len(names) == 2
    origin_report = json.loads(Path(source_map).read_text())
    mapping = origin_report['source_mapping']
    origins = [json.loads((q / name / 'metadata.json').read_text())['origins'] for name in names]


def compare_origins(outcomes):
    if not source_map:
        assert all(outcome == outcomes[0] for outcome in outcomes), outcomes
        return {}
    before, after = outcomes
    assert before['status'] == after['status'], outcomes
    current = before['origin']
    if check_witness(origin_report, q / names[0], q / names[1], current, after['origin']):
        old, new = (table[value['origin']] for table, value in zip(origins, outcomes))
        return {'matched_source_origins': [old, new], 'per_actor_outcomes': outcomes,
                'checked_action_witness': origin_report['checked_action_witnesses'][str(current)]}
    offsets = {site['site'] - int(old) for old, site in mapping.items()
               if site is not None and 'matching_run' in site
               and site['matching_run'][0] <= current < site['matching_run'][1]}
    if direct := mapping.get(str(current)):
        offsets.add(direct['site'] - current)
    assert len(offsets) == 1 and current + offsets.pop() == after['origin'], outcomes
    old, new = (table[value['origin']] for table, value in zip(origins, outcomes))
    assert (old['node'], old['local']) == (new['node'], new['local']), (old, new)
    return {'matched_source_origins': [old, new], 'per_actor_outcomes': outcomes}


actors = []
logs = []
for name in names:
    p = q / name
    worker = p / 'checked-exit-failure-worker'
    args = json.loads((p / 'runtime.json').read_text())['link']
    link = [str(worker_source) if arg.endswith('/mech-worker.rs') else str(worker)
            if arg == str(p / 'mech-worker') else arg for arg in args]
    subprocess.run(link, check=True)
    logs.append((p / 'checked-exit-failure.stderr').open('w'))
    actors.append(subprocess.Popen([worker, p / 'metadata.json', fixtures],
                                  stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                  stderr=logs[-1], text=True))

selected = ('empty', 'one-stored', 'seven-stored-prefix', 'prefix-periodic-4096',
            'prefix-repeated-16384', 'heldout-mixed-131073')
results = {}
batch = 2
try:
    for child in actors:
        assert child.stdout.readline().strip() == 'SESSION_READY'
    for name in selected:
        index = next(i for i, item in enumerate(inputs) if item['name'] == name)
        for child in actors:
            child.stdin.write(f'C\t{index}\t{batch * 2 - 1}\n')
            child.stdin.flush()
            assert child.stdout.readline().strip() == f'CASE_READY\t{name}'
        observations = [[] for _ in actors]
        for order in (range(len(actors)), reversed(range(len(actors)))):
            for j in order:
                child = actors[j]
                for _ in range(batch):
                    child.stdin.write('N')
                    child.stdin.flush()
                    observations[j].append(json.loads(child.stdout.readline()))
        outcomes = []
        for j, child in enumerate(actors):
            child.stdin.write('N')
            child.stdin.flush()
            observations[j].append(json.loads(child.stdout.readline()))
            assert child.stdout.readline().strip() == f'CASE_DONE\t{name}'
            outcome = [{k: v for k, v in row.items() if k != 'seconds'} for row in observations[j]]
            assert all(row == outcome[0] for row in outcome)
            outcomes.append(outcome[0])
        attribution = compare_origins(outcomes)
        results[name] = dict(outcome=outcomes[0], medians_seconds=[median(
            row['seconds'] for row in rows[1:-1]) for rows in observations], raw=observations,
            **attribution)
        print(name, outcomes if attribution else outcomes[0], results[name]['medians_seconds'],
              flush=True)
    for child in actors:
        child.stdin.write('Q\n')
        child.stdin.flush()
        assert child.wait(timeout=30) == 0
finally:
    for child in actors:
        if child.poll() is None:
            child.terminate()
            child.wait(timeout=10)
    for log in logs:
        log.close()
assert all('CLEANUP private_payload_bytes=0 live_owner_records=0' in
           (q / name / 'checked-exit-failure.stderr').read_text() for name in names)
(q / os.environ.get('MECH_ERROR_REPORT', 'checked-exit-linked-failures.json')).write_text(json.dumps(dict(actors=names, cases=results), indent=2) + '\n')
