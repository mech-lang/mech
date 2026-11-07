#!/usr/bin/env python3
"""Verify evidence, its historical compiler, and any current integration inventory."""
import gzip
import hashlib
import json
import subprocess
from pathlib import Path

packet = Path(__file__).resolve().parent
root = packet.parents[1]
manifest = json.loads((packet / 'review/manifest.json').read_text())

def verify(base, name, expected):
    path = (base / name).resolve()
    if not path.is_relative_to(base.resolve()):
        raise SystemExit(f'Invalid manifest path: {name}')
    if not path.is_file():
        raise SystemExit(f'Missing published file: {name}')
    actual = hashlib.sha256(path.read_bytes()).hexdigest()
    if actual != expected:
        raise SystemExit(f'Hash mismatch: {name}')

for name, item in manifest['files'].items():
    verify(packet, name, item['sha256'])
    if item.get('compression') == 'gzip':
        assert hashlib.sha256(gzip.decompress((packet/name).read_bytes())).hexdigest() == item['uncompressed_sha256'], name
# The current branch includes newer compiler work. Historical timings belong
# to the recorded Git tree, not to whatever compiler happens to be checked out.
revision = manifest['measured_compiler']
assert len(revision) == 40 and all(c in '0123456789abcdef' for c in revision)
names = list(manifest['compiler_files'])
for name in names:
    assert (root/name).resolve().is_relative_to(root.resolve()), name
objects = subprocess.run(
    ['git', 'cat-file', '--batch'], cwd=root,
    input=''.join(f'{revision}:{name}\n' for name in names).encode(),
    capture_output=True, check=True,
).stdout
at = 0
for name in names:
    end = objects.index(b'\n', at)
    header = objects[at:end].split()
    if len(header) != 3 or header[1] != b'blob':
        raise SystemExit(f'Historical compiler blob unavailable: {revision}:{name}')
    size = int(header[2])
    at = end + 1
    data = objects[at:at+size]
    assert len(data) == size and objects[at+size:at+size+1] == b'\n', name
    assert hashlib.sha256(data).hexdigest() == manifest['compiler_files'][name], name
    at += size + 1
assert at == len(objects)

integration = packet / 'integration/source-inventory.json'
current_files = 0
if integration.exists():
    current = json.loads(integration.read_text())
    for name, expected in current['files'].items():
        verify(root, name, expected)
    current_files = len(current['files'])

# Validate the corrected audit from raw reports, not a printed success label.
evidence = packet / 'integration/evidence'
audit_path = evidence / 'funding-audit-audit-summary.json'
if audit_path.exists():
    audit = json.loads(audit_path.read_text())
    assert audit['status'] == 'passed'
    def control_rows(text):
        return [{k: v for k, v in json.loads(line).items()
                 if k not in ('seconds', 'seconds_per_evaluation')}
                for line in text.splitlines() if line.startswith('{')]
    current = control_rows(json.loads((evidence / 'current-control-one-kib-run.json').read_text())['stdout'])
    for name in ('ir226-control-one-kib-run.json', 'funding-disabled-one-kib-run.json'):
        assert control_rows(json.loads((evidence / name).read_text())['stdout']) == current, name
    assert control_rows((evidence / 'funding-audit-ir226-repeat.stdout.log').read_text()) == current
    assert len(current) == 5
    old_comparison = json.loads((evidence / 'ir226-control-one-kib-comparison.json').read_text())
    assert old_comparison['all_nontiming_fields_equal_IR225'] is False
    assert len(old_comparison['differences']) == 3
    assert json.loads((evidence / 'funding-audit-audit.json').read_text())['exit'] == 0

screen_path = evidence / 'current-screen-report.json'
if screen_path.exists():
    screen = json.loads(screen_path.read_text())
    for phase, count in (('initial', 405), ('confirmation', 675),
                         ('extended', 1395), ('resources', 357)):
        report = json.loads(gzip.decompress((evidence / f'current-{phase}.json.gz').read_bytes()))
        assert len(report['samples']) == count
        assert all(row['exit'] == 0 and row['exact_rust_output']
                   for row in report['samples'])
        if phase == 'extended':
            assert report['summary'] == screen['extended_31_rounds']
            assert report['matched_native_reference'] == screen['references']['matched']
            assert report['whole_process_reference'] == screen['references']['whole']
    assert screen['qualification']['cohort_outputs_exact'] == 2832
    assert screen['qualification']['genuine_old_objects_rejected'] == 25
    assert screen['paired']['extended']['mech-before-4']['wall_seconds']['wins'] == 19
    assert screen['selected'] is False and screen['goal_complete'] is False

latest_path = evidence / 'ir228-screen-report.json'
if latest_path.exists():
    latest = json.loads(latest_path.read_text())
    assert latest['identity'] == 'IR228/work115/ABI41/image9'
    for phase, count in (('initial', 405), ('confirmation', 675), ('resources', 357)):
        report = json.loads(gzip.decompress((evidence / f'ir228-{phase}.json.gz').read_bytes()))
        assert len(report['samples']) == count
        assert all(row['exit'] == 0 and row['exact_rust_output']
                   for row in report['samples'])
        if phase == 'confirmation':
            assert report['summary'] == latest['confirmation_15_rounds']
    assert latest['qualification']['genuine_old_objects_rejected'] == 26
    assert latest['qualification']['cohort_outputs_exact'] == 1437
    assert all(row['complete_native_text_byte_identical']
               for row in latest['native_text_equivalence'].values())
    for kind in ('scalar', 'one-kib'):
        result = json.loads((evidence / f'ir228-control-{kind}-comparison.json').read_text())
        assert result['all_nontiming_fields_equal_IR227']
    assert latest['selected'] is False and latest['goal_complete'] is False

current_status = packet / 'integration/ir231-review-status.json'
if current_status.exists():
    status = json.loads(current_status.read_text())
    assert status['identity'] == 'IR231/work115/ABI41/image9'
    assert status['target_pr'] == 973
    assert status['gpu_passed'] == 841
    assert len(status['inherited_gpu_failures']) == 2
    assert status['current_performance_cohort_run'] is True
    assert status['new_performance_gain_claim'] is False
    assert status['selected'] is False and status['goal_complete'] is False
    before = json.loads((evidence / 'ir231-unfixed-confirmation.json').read_text())
    assert before['inherited_miscompile_reproduced']
    assert json.loads((evidence / 'ir231-unfixed-reproducer.json').read_text())['exit'] == 101
    after = json.loads((evidence / 'ir231-gpu-focus-ordered_reduction.json').read_text())
    assert after['exit'] == 0
    full = json.loads((evidence / 'ir231-gpu-test-run.json').read_text())
    assert full['expected_failures_exact'] and not full['new_failures']
    for kind, count in [('scalar', 128), ('one-kib', 5)]:
        def rows(path):
            return [{k: v for k, v in json.loads(line).items()
                     if k not in ('seconds', 'seconds_per_evaluation')}
                    for line in json.loads(path.read_text())['stdout'].splitlines()
                    if line.startswith('{')]
        old = rows(evidence / f'ir228-control-{kind}-run.json')
        new = rows(evidence / f'ir231-control-{kind}-run.json')
        assert len(old) == len(new) == count and old == new
    native = json.loads((evidence / 'ir231-native-text-equivalence.json').read_text())
    assert len(native) == 4
    assert all(x['complete_native_text_byte_identical'] for x in native.values())

experiments = evidence / 'boolean-experiments'
if experiments.exists():
    experiment = json.loads((experiments / 'summary.json').read_text())
    assert experiment['selected'] is False and experiment['goal_complete'] is False
    assert experiment['qualification']['IR232_gpu_passed'] == 843
    assert experiment['qualification']['IR233_full_gpu_and_compression_controls_not_run']
    total = 0
    for label, expected in [('IR232-latency', 248), ('IR232-resources', 63),
                            ('IR233-latency', 279), ('IR233-resources', 70)]:
        run = json.loads(gzip.decompress((experiments / (label+'.json.gz')).read_bytes()))
        assert len(run['samples']) == expected
        assert all(row['exit'] == 0 and row['exact_rust_output'] for row in run['samples'])
        assert run['execution_order']['mode'] == 'randomized'
        assert len(run['execution_order']['rounds']) == (31 if 'latency' in label else 7)
        total += expected
    assert total == 660
    latest = json.loads(gzip.decompress((experiments / 'IR233-latency.json.gz').read_bytes()))
    assert latest['matched_native_reference']['variant'] == 'rust-mech-control-u8-ids-u32'
    assert latest['whole_process_reference']['variant'] == 'rust-mech-control-u32-casts'
    assert latest['summary']['mech-parent-4']['median_post_json_seconds'] == .002961916
    assert latest['summary']['mech-width-4']['median_post_json_seconds'] == .002954208
    assert experiment['paired']['mech-after-4']['mech-parent-4']['post_json_seconds']['wins'] == 14
    assert experiment['paired']['mech-width-4']['mech-parent-4']['post_json_seconds']['wins'] == 17

current_status = packet / 'integration/ir234-review-status.json'
if current_status.exists():
    status = json.loads(current_status.read_text())
    assert status['identity'] == 'IR234/work115/ABI41/image9'
    assert status['compiler'] == 'c4649860a5e445c1ce1bdec8503870e8a455da17'
    assert status['incoming'] == '176d8134133befb8e35a6413fb8f98387bacebb3'
    assert status['target_pr'] == 973 and status['gpu_passed'] == 844
    assert len(status['inherited_gpu_failures']) == 2
    assert status['current_performance_cohort_run']
    assert not status['selected'] and not status['goal_complete']
    tests = json.loads((evidence / 'ir234-gpu-test-run.json').read_text())
    assert tests['expected_failures_exact'] and not tests['new_failures']
    assert '844 passed; 2 failed' in tests['summary'][0]
    for kind, count in [('scalar', 128), ('one-kib', 5)]:
        def rows(path):
            return [{k: v for k, v in json.loads(line).items()
                     if k not in ('seconds', 'seconds_per_evaluation')}
                    for line in json.loads(path.read_text())['stdout'].splitlines()
                    if line.startswith('{')]
        old = rows(evidence / f'ir231-control-{kind}-run.json')
        new = rows(evidence / f'ir234-control-{kind}-run.json')
        assert len(old) == len(new) == count and old == new
    qualification = json.loads((evidence / 'ir234-qualification-summary.json').read_text())
    assert qualification['all_completed_checks_asserted']
    assert qualification['loader_fixtures_exact'] == 434
    assert qualification['decoded_bundle_files_exact'] == 52
    assert qualification['finite_denials_exact'] == 48
    assert qualification['finite_small_full_successes_exact'] == 12
    assert qualification['forged_frontier_rejections'] == 16
    assert qualification['initial_driver_exit'] == 1
    assert json.loads((evidence / 'ir234-qualify-loaders-run.json').read_text())['exit'] == 1
    assert json.loads((evidence / 'ir234-repaired-last-loader-host-build-retry-run.json').read_text())['exit'] == 0
    for name in qualification['fixtures']:
        data = json.loads((evidence / ('ir234-edges-'+name.replace('/', '-'))).read_text())
        assert len(data) == 31
        assert all(x['exit'] == 0 and x.get('exact_rust_output', x.get('exact_expected_output', False)) for x in data)
    native = json.loads((evidence / 'ir234-native-text-equivalence.json').read_text())
    assert len(native) == 4 and all(x['complete_native_text_byte_identical'] for x in native.values())
    old_objects = json.loads((evidence / 'ir234-old-object-summary.json').read_text())
    assert old_objects['fresh_old_objects'] == 3
    assert all(x['old_entry_never_invoked'] and x['object_bound_rejected'] and x['compiled_module_rejected'] for x in old_objects['rows'])
    source_dir = evidence / 'ordered-source-experiments'
    source = json.loads((source_dir / 'summary.json').read_text())
    assert source['exact_cohort_outputs'] == 933 and source['fixture_outputs'] == 93
    assert not source['selected'] and not source['goal_complete']
    for label, latency, resources in [('ordered', 217, 56), ('fused', 248, 63), ('reused-masks', 279, 70)]:
        for phase, expected in [('latency', latency), ('resources', resources)]:
            run = json.loads(gzip.decompress((source_dir / f'{label}-{phase}.json.gz').read_bytes()))
            assert len(run['samples']) == expected
            assert all(row['exit'] == 0 and row['exact_rust_output'] for row in run['samples'])
            assert run['execution_order'] == source['cohorts'][label+'-'+phase]['execution_order']
            assert run['execution_order']['mode'] == 'randomized'
        assert hashlib.sha256((source_dir / (label+'.mec')).read_bytes()).hexdigest() == source['sources'][label]['source_sha256']
    assert source['paired']['fused']['mech-fused-4']['post_json_seconds']['wins'] == 21
    assert source['paired']['fused']['mech-fused-4']['wall_seconds']['wins'] == 23
    selection = json.loads((source_dir / 'reused-masks-native-selection.json').read_text())
    assert len(selection['compiled_numeric_functions']) == 50
    assert next(x for x in selection['numeric_tree_candidates'] if x['iteration'] == 3)['rejection'] == 'numeric tree child returns an older array version'

current_status = packet / 'integration/ir235-review-status.json'
if current_status.exists():
    status = json.loads(current_status.read_text())
    assert status['identity'] == 'IR235/work115/ABI41/image9'
    assert status['compiler'] == 'cbe5b47839b4fe2300fe16ebc1fcacfcfa03f98b'
    assert status['target_pr'] == 973 and status['gpu_passed'] == 849
    assert len(status['inherited_gpu_failures']) == 2
    assert status['source_files'] == 2464 and status['cohort_outputs_exact'] == 349
    assert not status['selected'] and not status['goal_complete']
    tests = json.loads((evidence / 'ir235-gpu-test-run.json').read_text())
    assert tests['expected_failures_exact'] and not tests['new_failures']
    assert '849 passed; 2 failed' in tests['summary'][0]
    for kind, count in [('scalar', 128), ('one-kib', 5)]:
        def rows(path):
            return [{k: v for k, v in json.loads(line).items()
                     if k not in ('seconds', 'seconds_per_evaluation')}
                    for line in json.loads(path.read_text())['stdout'].splitlines()
                    if line.startswith('{')]
        old = rows(evidence / f'ir234-control-{kind}-run.json')
        new = rows(evidence / f'ir235-control-{kind}-run.json')
        assert len(old) == len(new) == count and old == new
    qualification = json.loads((evidence / 'ir235-qualification-summary.json').read_text())
    assert qualification['loader_fixtures_exact'] == 558
    assert qualification['decoded_bundle_files_exact'] == 65
    assert qualification['finite_denials_exact'] == 48
    assert qualification['finite_small_full_successes_exact'] == 12
    assert qualification['forged_frontier_rejections'] == 16
    assert len(qualification['fixtures']) == 18
    for name in qualification['fixtures']:
        data = json.loads((evidence / ('ir235-edges-'+name.replace('/', '-'))).read_text())
        assert len(data) == 31
        assert all(x['exit'] == 0 and x.get('exact_rust_output', x.get('exact_expected_output', False)) for x in data)
    native = json.loads((evidence / 'ir235-native-text-equivalence.json').read_text())
    assert len(native) == 4 and all(x['complete_native_text_byte_identical'] for x in native.values())
    old = json.loads((evidence / 'ir235-old-object-summary.json').read_text())
    assert old['fresh_old_objects'] == 3 and {x['old_typed_ir'] for x in old['rows']} == {234, 232, 233}
    assert all(x['old_entry_never_invoked'] and x['object_bound_rejected'] and x['compiled_module_rejected'] for x in old['rows'])
    audit = json.loads((evidence / 'ir235-gpu-boolean-accounting-audit.json').read_text())
    assert audit['rows'] == len(audit['observations']) == 112
    assert audit['complete_source_price_tables_equal'] and audit['all_totals_retained']
    for row in audit['observations']:
        assert row['source_prices_equal']
        assert all(after >= before for _, _, before, after in row['callback_debits'])
        assert sum(after-before for _, _, before, after in row['callback_debits']) == row['callback_work']
        assert row['callback_work'] + row['outside_callback_work'] == row['work']
    assert audit['first_case_difference_entirely_within_callbacks'] and audit['first_case_outside_callback_work_both'] == 907408
    ci = json.loads((evidence / 'ir235-gpu-ci-selected-inventory-audit.json').read_text())
    assert ci['current_selected_count'] == len(ci['selected_names']) == 354
    assert ci['all_selected_names_executed_and_passed_in_complete_gpu_run']
    assert len(ci['new_names']) == 5 and ci['filters_removed'] == []
    # This inventory belongs to the retained IR235 publication. Later
    # integrations add tests and keep their own current inventory below.
    assert subprocess.check_output(
        ['git', 'show', 'e811116a249a024088f69fe08529de45181bbb30:.github/workflows/ci-full.yml'],
        cwd=root,
    ) == (evidence / 'ir235-ci-full.yml').read_bytes()
    assert (root / 'hosts/gpu/tests/fixtures/native/numeric-boolean-mask-reuse.mec').read_bytes() == (evidence / 'ir235-native-mask-reuse.mec').read_bytes()
    experiment = evidence / 'boolean-implication-mask-reuse'
    current = json.loads((experiment / 'summary.json').read_text())
    assert current['source_unchanged_across_compilers']
    assert current['source_sha256'] == hashlib.sha256((experiment / 'source.mec').read_bytes()).hexdigest()
    for phase, expected in [('latency', 279), ('resources', 70)]:
        run = json.loads(gzip.decompress((experiment / (phase+'.json.gz')).read_bytes()))
        assert len(run['samples']) == expected and run['execution_order']['mode'] == 'randomized'
        assert all(x['exit'] == 0 and x['exact_rust_output'] for x in run['samples'])
    pairs = current['paired']['mech-current-reuse-4']['mech-current-original-4']
    assert pairs['post_json_seconds']['wins'] == 27 and pairs['wall_seconds']['wins'] == 14
    assert current['profile_invocations_exact'] == 2000
    profile = json.loads((experiment / 'profile-summary.json').read_text())
    assert set(profile['rows']) == {'before', 'after'}
    assert all(x['all1000_index_arrays_and_final_rust_output_exact'] and x['profiled_timings_excluded'] for x in profile['rows'].values())

current_status = packet / 'integration/ir236-review-status.json'
if current_status.exists():
    status = json.loads(current_status.read_text())
    assert status['identity'] == 'IR236/work115/ABI41/image9'
    assert status['compiler'] == '67bdc1a3ec9e28accb2fd968f4292ca73d81b542'
    assert status['incoming'] == '40bba44acd7a60e28e3378dab012ee9d50007c66'
    assert status['source_files'] == current_files == 2465
    assert status['gpu_passed'] == 853 and status['ci_selected_names_passed'] == 358
    assert not status['metadata_patch_applied'] and not status['borrowed_json_applied']
    assert not status['selected'] and not status['goal_complete']
    assert (root / '.github/workflows/ci-full.yml').read_bytes() == (evidence / 'ir236-ci-full.yml').read_bytes()
    for ir, previous, fixture_count in [(236, 235, 558), (237, 236, 589)]:
        tests = json.loads((evidence / f'ir{ir}-tests-test-run.json').read_text())
        assert tests['expected_failures_exact'] and not tests['new_failures']
        assert '853 passed; 2 failed' in tests['summary'][0]
        ci = json.loads((evidence / f'ir{ir}-tests-ci-selected-inventory-audit.json').read_text())
        assert ci['current_selected_count'] == len(ci['selected_names']) == 358
        assert ci['all_selected_names_executed_and_passed_in_complete_gpu_run']
        for kind, count in [('scalar', 128), ('one-kib', 5)]:
            if ir == 236:
                old_path = evidence / f'ir235-control-{kind}-run.json'
            else:
                old_path = evidence / f'ir236-regressions-{kind}-run.json'
            new_path = evidence / f'ir{ir}-regressions-{kind}-run.json'
            assert len(rows(old_path)) == len(rows(new_path)) == count
            assert rows(old_path) == rows(new_path)
        qualification = json.loads((evidence / f'ir{ir}-runners-qualification-summary.json').read_text())
        assert qualification['loader_fixtures_exact'] == fixture_count
        assert qualification['decoded_bundle_files_exact'] == 65
        assert qualification['finite_denials_exact'] == 48
        assert qualification['finite_small_full_successes_exact'] == 12
        assert qualification['forged_frontier_rejections'] == 16
        for name in qualification['fixtures']:
            data = json.loads((evidence / f"ir{ir}-edges-{name.replace('/', '-')}").read_text())
            assert len(data) == 31
            assert all(x['exit'] == 0 and x.get('exact_rust_output', x.get('exact_expected_output', False)) for x in data)
        old = json.loads((evidence / f'ir{ir}-old-objects-summary.json').read_text())
        assert old['fresh_old_objects'] == 2
        assert {x['old_typed_ir'] for x in old['rows']} == {previous, previous-1}
        assert all(x['old_entry_never_invoked'] and x['object_bound_rejected'] and x['compiled_module_rejected'] for x in old['rows'])
        assert json.loads((evidence / f'ir{ir}-runtime-check-summary.json').read_text())['status'] == 'passed'
        assert json.loads((evidence / f'ir{ir}-feature-check-scalar-compiler.json').read_text())['exit'] == 0
    experiment = evidence / 'local-facts-and-metadata'
    text_proof = json.loads((experiment / 'native-function-text-correspondence.json').read_text())
    assert text_proof['functions_compared'] == 263
    assert text_proof['all_five_query_functions_byte_identical']
    assert len(text_proof['variants']) == 5
    for variant in text_proof['variants'].values():
        assert variant['native_text_saved_bytes'] == 2560
        assert len(variant['changed_headers']) == 8
        query = variant['functions'][str(variant['query_header'])]
        assert query['before_bytes'] == query['after_bytes'] == 28448
        assert query['before_sha256'] == query['after_sha256'] == variant['query_sha256']
    prototype = json.loads((evidence / 'ir237-source-inventory.json').read_text())['files']
    overrides = json.loads((experiment / 'ir237-source-overrides.json').read_text())
    assert set(overrides) == {'src/core/src/type_system/resolved_value.rs', 'src/core/src/function/specialization.rs', 'hosts/gpu/src/native_artifact/program/image/identity.rs'}
    assert len(prototype) == 2465
    for name, expected in prototype.items():
        if name in overrides:
            verify(experiment, overrides[name], expected)
        else:
            verify(root, name, expected)
    summary = json.loads((experiment / 'summary.json').read_text())
    assert summary['decision']['metadata_patch_applied_to_active_compiler'] is False
    assert summary['decision']['borrowed_json_applied_to_active_adapter'] is False
    for phase, expected in [('latency', 372), ('resources', 91)]:
        run = json.loads(gzip.decompress((experiment / (phase+'.json.gz')).read_bytes()))
        assert len(run['samples']) == expected and run['execution_order']['mode'] == 'randomized'
        assert all(x['exit'] == 0 and x['exact_rust_output'] for x in run['samples'])
    profiles = json.loads((experiment / 'profile-summary.json').read_text())
    assert all(x['all1000_index_arrays_and_final_rust_output_exact'] and x['profiled_timings_excluded'] for x in profiles['rows'].values())
    ownership = json.loads((experiment / 'borrowed-json-json-ownership-audit-run.json').read_text())
    assert ownership['exit'] == 0 and len(ownership['rows']) == 6
    assert json.loads((experiment / 'borrowed-json-escaped-id-title-output.json').read_text()) == json.loads((experiment / 'borrowed-json-escaped-id-title-expected.json').read_text())

report = json.loads((packet / 'results/comparison-native-fixed-frame-arenas-confirmation-20261106-5000-m3.json').read_text())
assert report['summary']['mech-after-4']['median_post_json_seconds'] == 0.003140833
assert report['matched_native_reference']['median_seconds'] == 0.001651875
assert report['whole_process_reference']['variant'] == 'rust-mech-control-streaming'
print(json.dumps(dict(status='verified', evidence_files=len(manifest['files']),
                      historical_compiler_files=len(manifest['compiler_files']),
                      historical_source='recorded_git_tree',
                      current_integration_source_files=current_files,
                      measured_compiler=manifest['measured_compiler'],
                      executes_benchmark=False)))
