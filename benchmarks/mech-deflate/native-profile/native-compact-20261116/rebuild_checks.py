"""Checked worker success and allocation-failure observations for a rebuilt encoder."""

import json
import subprocess


CLEANUP = 'CLEANUP private_payload_bytes=0 live_owner_records=0\n'


def observations(worker, metadata, fixture_file, cases, log, env, samples=0):
    rows = []
    command = [str(worker), str(metadata), str(fixture_file)]
    with log.open('w') as error, subprocess.Popen(
        command, env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=error, text=True,
    ) as child:
        try:
            assert child.stdout.readline().strip() == 'SESSION_READY'
            for index, name in cases:
                child.stdin.write(f'C\t{index}\t{samples}\n' + 'N' * (samples + 2))
                child.stdin.flush()
                assert child.stdout.readline().strip() == f'CASE_READY\t{name}'
                for _ in range(samples + 2):
                    row = json.loads(child.stdout.readline())
                    assert row['case'] == name
                    rows.append(row)
                assert child.stdout.readline().strip() == f'CASE_DONE\t{name}'
            child.stdin.write('Q\n')
            child.stdin.flush()
            assert child.wait(timeout=30) == 0
        finally:
            if child.poll() is None:
                child.terminate()
                child.wait(timeout=10)
    assert log.read_text().endswith(CLEANUP), 'private heap cleanup was not observed'
    return rows


def check_successes(worker, metadata, fixture_file, fixtures, log, env):
    cases = [(index, entry['name']) for index, entry in enumerate(fixtures)]
    rows = observations(worker, metadata, fixture_file, cases, log, env)
    assert len(rows) == 162
    for row in rows:
        assert all(row[key] for key in (
            'exact_reference', 'independent_inflation', 'input_alias_unchanged',
            'borrowed_output_alias_unchanged',
        ))
    return rows


def write_fault_probe(worker_source, out):
    source = worker_source.read_text()
    allocator = worker_source.with_name('pinned_allocator.rs').read_text()
    allocation = '    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {\n'
    assert allocator.count(allocation) == 1
    allocator = allocator.replace(allocation, allocation + """        if layout.align() == 1 && FAIL_CAPACITY.compare_exchange(
            layout.size(), 0, Ordering::Relaxed, Ordering::Relaxed,
        ).is_ok() {
            return std::ptr::null_mut();
        }
""")
    allocator += """
use std::sync::atomic::{AtomicUsize, Ordering};
pub static FAIL_CAPACITY: AtomicUsize = AtomicUsize::new(0);
"""
    (out / 'fault_allocator.rs').write_text(allocator)
    changes = (
        ('"pinned_allocator.rs"', '"fault_allocator.rs"'),
        ('        let status = unsafe {', """        let inject_failure = sample % 2 == 0;
        let input_snapshot = inject_failure.then(|| input.clone());
        use std::sync::atomic::Ordering;
        if inject_failure {
            pinned_allocator::FAIL_CAPACITY.store(case.capacity, Ordering::Relaxed);
        }
        let status = unsafe {"""),
    )
    for old, new in changes:
        assert source.count(old) == 1, old
        source = source.replace(old, new)
    recovery = '        let returned = if status.status == 0 {'
    assert source.count(recovery) == 1
    source = source.replace(recovery, """        if let Some(snapshot) = input_snapshot {
            assert_eq!(pinned_allocator::FAIL_CAPACITY.swap(0, Ordering::Relaxed), 0);
            assert_ne!(status.status, 0, "allocation failure did not reach the caller");
            assert_eq!(current, prefix);
            assert_eq!(snapshot, input);
            assert_eq!(current.as_ptr(), prior_pointer);
            assert_eq!(output, [0; 6], "failed call published output");
            drop(heap);
            return serde_json::json!({
                "case": case.name, "status": status.status, "origin": status.origin,
            });
        }
""" + recovery)
    fault_source = out / 'fault_worker.rs'
    fault_source.write_text(source)
    return fault_source


def check_faults(directory, out, link_command, metadata, fixture_file, fixtures, env, run):
    policy = json.loads((directory / 'qualification-policy-v3.json').read_text())
    expected = policy['checked_failures']
    worker_source = directory / 'worker.rs'
    probe_source = write_fault_probe(worker_source, out)
    probe = out / 'fault-worker'
    command = list(link_command)
    command[command.index(worker_source)] = probe_source
    command[command.index('-o') + 1] = probe
    link = run(command, out / 'fault-link.log', env)
    cases = [(index, entry['name']) for index, entry in enumerate(fixtures) if entry['name'] in expected]
    assert len(cases) == len(expected)
    rows = observations(probe, metadata, fixture_file, cases, out / 'fault-run.log', env, samples=2)
    failures = rows[::2]
    recoveries = rows[1::2]
    for failure, recovery in zip(failures, recoveries, strict=True):
        witness = expected[failure['case']]
        assert failure['status'] == witness['status'] and failure['origin'] == witness['origin'], failure
        assert recovery['case'] == failure['case'] and recovery['exact_reference'], recovery
        assert recovery['independent_inflation'] and recovery['input_alias_unchanged'], recovery
        assert recovery['borrowed_output_alias_unchanged'], recovery
        del recovery['seconds']
    (out / 'checked-failures.json').write_text(json.dumps(failures, indent=2) + '\n')
    (out / 'checked-recoveries.json').write_text(json.dumps(recoveries, indent=2) + '\n')
    return {
        'link': link, 'cases': len(cases), 'observations': len(failures),
        'exact_recoveries': len(recoveries),
    }
