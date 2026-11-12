from pathlib import Path
import hashlib, json

U = Path(__file__).parent
read = lambda p: json.loads(p.read_text())
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
reports = sorted(U.glob('edges-*/edge-fixtures.json')) + sorted(U.glob('*/edges/edge-fixtures.json'))
assert len(reports) == 14, len(reports)
fixtures = {}
for path in reports:
    rows = read(path)
    assert len(rows) == 31
    assert all(row['exit'] == 0 and row.get('exact_expected_output', row.get('exact_rust_output', False)) for row in rows)
    fixtures[str(path.relative_to(U))] = dict(cases=len(rows), sha256=sha(path))

repair = U/'compact-4-full-catalog'
failed = read(repair/'host-build-run.json.failed-disk-full')
assert failed['exit'] != 0 and 'ld: write() failed, errno=28' in failed['stdout']+failed['stderr']
assert read(U/'qualify-loaders-run.json')['exit'] == 1
assert read(repair/'host-build-run.json')['exit'] != 0
assert read(repair/'host-build-retry-run.json')['exit'] == 0
assert read(repair/'runner-aot.build.json.failed-disk-full')['exit'] != 0
assert read(repair/'runner-aot.build.json')['exit'] == 0
assert read(repair/'audit-run.json')['exit'] == 0
assert read(U/'compact-4-full-catalog-edges-run.json')['exit'] == 0
assert read(U/'qualify-other-run.json')['exit'] == 0

denials = successes = 0
for family in ['budget', 'compact-budget']:
    report = read(U/family/'summary.json')
    assert report['all_eight_denials_exact']
    denial_rows = [row for row in report['cases'] if row['case'] != 'success']
    success_rows = [row for row in report['cases'] if row['case'] == 'success']
    assert len(denial_rows) == 8 and len(success_rows) == 1 and success_rows[0]['full_output_exact']
    for row in denial_rows:
        assert row['denial_exact'] and set(row['results']) == {'bound', 'independent', 'full-catalog'}
        assert all(x['output_unpublished'] and x['retained_bytes'] == 0 and x['retained_nodes'] == 0 for x in row['results'].values())
        denials += 3
    for loader in ['bound', 'independent', 'full-catalog']:
        for size in ['first-10', 'full-5000']:
            result = read(U/family/loader/('success-rust-'+size+'-run.json'))
            assert result['exit'] == 0 and result['exact_standard_rust_output']
            successes += 1

frontier = read(U/'frontier-image-probe/run.json')
assert frontier['exit'] == 0
frontier = json.loads(frontier['stdout'])
assert frontier['genuine_legacy_independent_bound_and_compiled_module_accepted'] and frontier['native_entry_never_invoked']
assert len(frontier['forged_cases']) == 8
assert all(x['independent_rejected'] and x['object_bound_rejected'] for x in frontier['forged_cases'])
decoded = read(U/'original-decoded-correspondence.json')
assert len(decoded) == 4 and all(x['all13_bundle_files_byte_identical'] and len(x['files']) == 13 for x in decoded.values())
assert denials == 48 and successes == 12
summary = dict(status='passed_with_completed_disk_full_repair', loader_fixtures_exact=sum(x['cases'] for x in fixtures.values()),
    fixtures=fixtures, decoded_bundle_files_exact=52, finite_denials_exact=denials,
    finite_small_full_successes_exact=successes, forged_frontier_rejections=16,
    initial_driver_exit=1, initial_failure='Last full-catalog host link exhausted disk; original driver and failed receipts remain unchanged.',
    repair='compact-4-full-catalog/host-build-retry-run.json', all_completed_checks_asserted=True)
assert summary['loader_fixtures_exact'] == 434
(U/'qualification-summary.json').write_text(json.dumps(summary, indent=2)+'\n')
print(json.dumps(summary | {'fixtures':len(fixtures)}), flush=True)
