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

def verify_git_inventory(revision, files):
    """Historical inventories belong to their recorded tree, never today's tree."""
    assert len(revision) == 40 and all(c in '0123456789abcdef' for c in revision)
    for name in files:
        assert (root/name).resolve().is_relative_to(root.resolve()), name
    data = subprocess.check_output(['git', 'cat-file', '--batch'], cwd=root,
        input=''.join(f'{revision}:{name}\n' for name in files).encode())
    at = 0
    for name, expected in files.items():
        end = data.index(b'\n', at)
        header = data[at:end].split()
        assert len(header) == 3 and header[1] == b'blob', (revision, name)
        size = int(header[2]); at = end + 1
        blob = data[at:at+size]
        assert len(blob) == size and data[at+size:at+size+1] == b'\n', name
        assert hashlib.sha256(blob).hexdigest() == expected, (revision, name)
        at += size + 1
    assert at == len(data)

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
    assert status['source_files'] == 2465
    retained236 = json.loads((evidence / 'ir236-source-inventory.json').read_text())['files']
    assert len(retained236) == 2465
    verify_git_inventory(status['compiler'], retained236)
    assert status['gpu_passed'] == 853 and status['ci_selected_names_passed'] == 358
    assert not status['metadata_patch_applied'] and not status['borrowed_json_applied']
    assert not status['selected'] and not status['goal_complete']
    assert subprocess.check_output(['git', 'show', status['compiler'] + ':.github/workflows/ci-full.yml'], cwd=root) == (evidence / 'ir236-ci-full.yml').read_bytes()
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
            assert retained236[name] == expected, name
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

schema_evidence = evidence / 'canonical-schema-and-boolean'
if schema_evidence.exists():
    summary = json.loads((schema_evidence / 'summary.json').read_text())
    status = json.loads((packet / 'integration/ir238-review-status.json').read_text())
    assert status['selected'] and not status['goal_complete']
    assert status['identity'] == 'IR238/work115/ABI41/image9'
    source238 = json.loads((schema_evidence / 'ir238-opt-source-inventory.json').read_text())['files']
    assert status['source_files'] == len(source238) == 2466
    verify_git_inventory(status['compiler'], source238)
    for stem, counts in [('canonical-schema-sinks', [434, 105]), ('boolean-insertion-combination', [341, 84])]:
        for phase, expected in zip(['randomized', 'resources'], counts):
            run = json.loads(gzip.decompress((schema_evidence / f'{stem}-{phase}.json.gz').read_bytes()))
            assert len(run['samples']) == expected and run['execution_order']['mode'] == 'randomized'
            assert all(r['exit'] == 0 and r['exact_rust_output'] for r in run['samples'])
            if stem == 'canonical-schema-sinks' and phase == 'randomized':
                assert run['summary'] == summary['latency_summary']
                by = {}
                for r in run['samples']:
                    by.setdefault(r['variant'], {})[r['sample']] = r
                a, b = by['mech-ir238-original-4'], by['mech-selected-ir208-original-4']
                for metric, wins in [('post_json_seconds', 31), ('wall_seconds', 21)]:
                    assert sum(a[i][metric] < b[i][metric] for i in a) == wins
    for ir, gpu, selected in [(238, 853, 358), (239, 855, 360)]:
        tests = json.loads((schema_evidence / f'ir{ir}-tests-test-run.json').read_text())
        assert tests['expected_failures_exact'] and not tests['new_failures']
        assert f'{gpu} passed; 2 failed' in tests['summary'][0]
        ci = json.loads((schema_evidence / f'ir{ir}-tests-ci-selected-inventory-audit.json').read_text())
        assert ci['current_selected_count'] == len(ci['selected_names']) == selected
        assert ci['all_selected_names_executed_and_passed_in_complete_gpu_run']
    # Only physical GlobalAlloc counters may differ. Source work, report fields,
    # errors, cleanup and every other non-timing value remain exact.
    physical = {'allocation_requests_per_100', 'requested_bytes_per_100', 'allocation_requests', 'requested_bytes'}
    for kind, count in [('scalar', 128), ('one-kib', 5)]:
        old = rows(evidence / f'ir236-regressions-{kind}-run.json')
        new = rows(schema_evidence / f'ir238-regressions-{kind}-run.json')
        assert len(old) == len(new) == count
        for a, b in zip(old, new):
            assert {k:v for k,v in a.items() if k not in physical} == {k:v for k,v in b.items() if k not in physical}
            assert all(b[k] <= a[k] for k in physical if k in a)
        if kind == 'one-kib':
            assert old == new
        assert new == rows(schema_evidence / f'ir239-regressions-{kind}-run.json')
    for label, expected in [('before', {'post-json': (3858,8013126), 'restore': (36437,13094307)}),
                            ('after', {'post-json': (3758,8012356), 'restore': (26837,12830999)})]:
        profile = json.loads(gzip.decompress((schema_evidence / f'allocation-{label}.json.gz').read_bytes()))
        assert profile['exact_outputs'] and profile['counts_and_bytes_match_current_resource_cohort']
        for phase, (requests, size) in expected.items():
            row = profile['variants'][phase]
            assert row['requests'] == requests and row['requested_bytes'] == size
            assert row['unattributed_requests'] == 0 and row['diagnostic_timings_excluded']
    prototype = json.loads((schema_evidence / 'ir239-opt-source-inventory.json').read_text())['files']
    overrides = json.loads((schema_evidence / 'ir239-source-overrides.json').read_text())
    for name, expected in prototype.items():
        if name in overrides:
            verify(schema_evidence, overrides[name], expected)
        else:
            assert source238[name] == expected, name
    assert not summary['boolean_combination']['applied_to_active_compiler']
    assert not summary['decision']['goal_complete']
    for crate, index, count in [('core', 0, 393), ('engine', 0, 138), ('engine', 1, 22), ('engine', 2, 129)]:
        stem = f'ir238-tests-mech_{crate}-run-{index}'
        assert json.loads((schema_evidence / (stem+'.json')).read_text())['exit'] == 0
        assert f'{count} passed; 0 failed' in (schema_evidence / (stem+'.stdout.log')).read_text()
    extra = list(schema_evidence.glob('ir238-edges-*rust-insert*edge-fixtures.json'))
    assert len(extra) == 4
    for path in extra:
        data = json.loads(path.read_text())
        assert len(data) == 31 and all(r['exit'] == 0 and r.get('exact_rust_output', r.get('exact_expected_output', False)) for r in data)
    for folder, ir in [(schema_evidence, 238), (evidence / 'child-entry-facts', 240)]:
        qualification = json.loads((folder / f'ir{ir}-runners-qualification-summary.json').read_text())
        assert qualification['loader_fixtures_exact'] == 558 and qualification['decoded_bundle_files_exact'] == 65
        assert qualification['finite_denials_exact'] == 48 and qualification['finite_small_full_successes_exact'] == 12
        assert qualification['forged_frontier_rejections'] == 16
        for family in ['budget', 'compact-budget']:
            budget = json.loads((folder / f'ir{ir}-{family}-summary.json').read_text())
            denials = [r for r in budget['cases'] if r['case'] != 'success']
            assert len(denials) == 8 and budget['all_eight_denials_exact']
            for row in denials:
                assert row['denial_exact'] and len(row['results']) == 3
                assert all(r['output_unpublished'] and r['retained_bytes'] == r['retained_nodes'] == 0 for r in row['results'].values())
        for name in qualification['fixtures']:
            data = json.loads((folder / f"ir{ir}-edges-{name.replace('/', '-')}").read_text())
            assert len(data) == 31
            assert all(r['exit'] == 0 and r.get('exact_rust_output', r.get('exact_expected_output', False)) for r in data)
        assert json.loads((folder / f'ir{ir}-runtime-check-summary.json').read_text())['status'] == 'passed'
        assert json.loads((folder / f'ir{ir}-feature-check-scalar-compiler.json').read_text())['exit'] == 0
        old = json.loads((folder / f'ir{ir}-old-objects-summary.json').read_text())
        assert old['fresh_old_objects'] == 2
        assert all(r['old_entry_never_invoked'] and r['object_bound_rejected'] and r['compiled_module_rejected'] for r in old['rows'])
    current = evidence / 'child-entry-facts'
    status = json.loads((packet / 'integration/ir240-review-status.json').read_text())
    assert status['source_files'] == current_files
    assert status['identity'] == 'IR240/work115/ABI41/image9' and not status['goal_complete']
    for phase, expected in [('randomized', 279), ('resources', 70)]:
        run = json.loads(gzip.decompress((current / f'child-entry-facts-integration-{phase}.json.gz').read_bytes()))
        assert len(run['samples']) == expected and run['execution_order']['mode'] == 'randomized'
        assert all(r['exit'] == 0 and r['exact_rust_output'] for r in run['samples'])
    tests = json.loads((current / 'ir240-tests-test-run.json').read_text())
    assert tests['expected_failures_exact'] and not tests['new_failures']
    assert '858 passed; 2 failed' in tests['summary'][0]
    ci = json.loads((current / 'ir240-tests-ci-selected-inventory-audit.json').read_text())
    assert ci['current_selected_count'] == len(ci['selected_names']) == 363
    assert ci['all_selected_names_executed_and_passed_in_complete_gpu_run']
    for kind, count in [('scalar', 128), ('one-kib', 5)]:
        a = rows(schema_evidence / f'ir238-regressions-{kind}-run.json')
        b = rows(current / f'ir240-regressions-{kind}-run.json')
        assert len(a) == len(b) == count and a == b

experiment = evidence / 'scalar-home-and-root-phases'
if experiment.exists():
    summary = json.loads((experiment / 'summary.json').read_text())
    assert summary['identity'] == 'IR241/work115/ABI41/image9'
    assert not summary['selected'] and not summary['applied_to_public_compiler'] and not summary['goal_complete']
    assert summary['qualification']['full_AOT_loader_finite_runtime_adoption_qualification_not_run']
    for phase, expected in [('randomized',310),('resources',77)]:
        run = json.loads(gzip.decompress((experiment / (phase+'.json.gz')).read_bytes()))
        assert len(run['samples']) == expected and run['execution_order']['mode'] == 'randomized'
        assert all(r['exit'] == 0 and r['exact_rust_output'] for r in run['samples'])
        if phase == 'randomized':
            by = {}
            for r in run['samples']: by.setdefault(r['variant'],{})[r['sample']] = r
            a, b = by['mech-ir241-original-4'], by['mech-ir238-original-4']
            assert sum(a[i]['post_json_seconds'] < b[i]['post_json_seconds'] for i in a) == 22
            assert sum(a[i]['wall_seconds'] < b[i]['wall_seconds'] for i in a) == 14
    tests = json.loads((experiment / 'tests-test-run.json').read_text())
    assert tests['expected_failures_exact'] and not tests['new_failures']
    assert '861 passed; 2 failed' in tests['summary'][0]
    ci = json.loads((experiment / 'tests-ci-selected-inventory-audit.json').read_text())
    assert ci['current_selected_count'] == len(ci['selected_names']) == 366 and len(ci['new_names']) == 3
    assert ci['all_selected_names_executed_and_passed_in_complete_gpu_run']
    for kind, count in [('scalar',128),('one-kib',5)]:
        before = rows(evidence / f'child-entry-facts/ir240-regressions-{kind}-run.json')
        after = rows(experiment / f'regressions-{kind}-run.json')
        assert len(before) == len(after) == count and before == after
    for name in ['after-4','reuse-4']:
        fixtures = json.loads((experiment / (name+'-edge-fixtures.json')).read_text())
        assert len(fixtures) == 31
        assert all(r['exit'] == 0 and r.get('exact_rust_output',r.get('exact_expected_output',False)) for r in fixtures)
    base = json.loads((experiment / 'ir240-base-source-inventory.json').read_text())['files']
    verify_git_inventory('deebf08df690e37ddf763635d70fd25aaf0bedb4', base)
    prototype = json.loads((experiment / 'source-inventory.json').read_text())['files']
    assert len(prototype) == 2470
    overrides = json.loads((experiment / 'source-overrides.json').read_text())
    for name, expected in prototype.items():
        if name in overrides: verify(experiment, overrides[name], expected)
        else: assert base[name] == expected, name
    phases = json.loads(gzip.decompress((experiment / 'captures.json.gz').read_bytes()))
    details = json.loads(gzip.decompress((experiment / 'helper-captures.json.gz').read_bytes()))
    assert len(phases['rows']) == 14 and phases['all14_outputs_exact'] and phases['timings_excluded']
    assert len(details['rows']) == 7 and details['all7_outputs_exact'] and details['timings_excluded']
    for r in phases['rows'] + details['rows']:
        assert r['exact_standard_rust_output'] and not r['primary_benchmark_timings']
        total = r['profile']['post_json_hardware']['instructions_retired']
        parallel = sum(w['hardware']['instructions_retired'] for w in r['profile']['parallel_run_windows'])
        assert parallel == r['parallel_hardware_sum']['instructions_retired']
        assert total - parallel == r['outside_parallel_hardware']['instructions_retired'] >= 0
    assert all(r['profile']['site_overflow_count'] == 0 for r in details['rows'])
    proof = json.loads((experiment / 'native-profile-text-proof.json').read_text())
    for label in ['before','after','after-helpers']:
        audit = json.loads((experiment / f'profile-{label}-native-link-audit.json').read_text())
        expected = json.loads((experiment / 'reuse-4-native-link-audit.json').read_text()) if label != 'before' else json.loads((evidence / 'child-entry-facts/ir240-reuse-4-native-link-audit.json').read_text())
        assert audit['normalized_text_sha256'] == expected['normalized_text_sha256'] == proof[label]['normalized_text_sha256']


# Current IR242 promotion and the independently measured IR243 ablation.
current = evidence / 'conjoined-windows-and-shared-certificates'
if current.exists():
    newest = json.loads((current/'summary.json').read_text())
    assert newest['compiler'] == '4cc266ae0d05c69432a4c15ffe99d5e586685647'
    assert newest['identity'] == 'IR242/work115/ABI41/image9'
    assert newest['incoming_973'] == '65034db9ca9da4133355108e85784703cde7592b'
    assert newest['selected_source'] == 'IR242 ordered-combinadic mask reuse/four workers'
    assert not newest['goal_complete'] and not newest['unselected_IR243']['selected']
    source242 = json.loads((current/'IR242-source-inventory.json').read_text())['files']
    assert len(source242) == 2468
    verify_git_inventory(newest['compiler'], source242)
    assert source242 == json.loads(integration.read_text())['files']
    latency = json.loads(gzip.decompress((current/'randomized.json.gz').read_bytes()))
    resource = json.loads(gzip.decompress((current/'resources.json.gz').read_bytes()))
    for report, count, seed in [(latency,372,990243),(resource,91,9902437)]:
        assert len(report['samples']) == count
        assert all(row['exit']==0 and row['exact_rust_output'] for row in report['samples'])
        assert report['execution_order']['seed'] == seed
    by_variant = {}
    for row in latency['samples']:
        by_variant.setdefault(row['variant'],{})[row['sample']] = row
    import statistics
    for label, expected in newest['paired'].items():
        variant, parent = label.split(' vs ')
        for metric, evidence_pair in expected.items():
            delta = [by_variant[variant][i][metric]-by_variant[parent][i][metric]
                     for i in range(31)]
            assert evidence_pair['wins'] == sum(x<0 for x in delta)
            assert evidence_pair['median_paired_delta_seconds'] == statistics.median(delta)
    paired = newest['paired']['mech-ir242-reuse-4 vs mech-ir238-original-4']
    assert paired['post_json_seconds']['wins'] == 29
    assert paired['wall_seconds']['wins'] == 24
    selected = json.loads((current/'selected-speed-checkpoint.json').read_text())
    assert selected['timing'] == latency['summary']['mech-ir242-reuse-4']
    assert selected['resources'] == resource['summary']['mech-ir242-reuse-4-instrumented']
    assert selected['timing']['median_post_json_seconds'] == 0.002839
    assert selected['matched_reference'] == latency['matched_native_reference']
    assert selected['whole_process_reference'] == latency['whole_process_reference']
    for prefix in ['IR242','IR243']:
        tests = json.loads((current/f'{prefix}-tests-test-run.json').read_text())
        assert tests['expected_failures_exact'] and len(tests['failed_tests']) == 2
        assert '863 passed' in tests['summary'][0]
        ci = json.loads((current/f'{prefix}-tests-ci-selected-inventory-audit.json').read_text())
        assert ci['actually_executed_and_passed'] == 368
    def raw_controls(path):
        data = json.loads(path.read_text())['stdout']
        return [{k:v for k,v in json.loads(line).items()
                 if k not in ('seconds','seconds_per_evaluation')}
                for line in data.splitlines() if line.startswith('{')]
    physical = {'allocation_requests_per_100','requested_bytes_per_100',
                'allocation_requests','requested_bytes'}
    for family,count in [('scalar',128),('one-kib',5)]:
        parent = raw_controls(evidence/f'child-entry-facts/ir240-regressions-{family}-run.json')
        rows242 = raw_controls(current/f'IR242-controls-{family}-run.json')
        rows243 = raw_controls(current/f'IR243-controls-{family}-run.json')
        assert len(parent)==len(rows242)==len(rows243)==count
        assert rows242 == parent
        for before,after in zip(rows242,rows243):
            assert {k:v for k,v in before.items() if k not in physical} == {
                k:v for k,v in after.items() if k not in physical}
            assert all(after[k] <= before[k] for k in physical & before.keys())
    fixtures = list(current.glob('IR242-edges-*.json'))
    assert len(fixtures)==18
    for fixture in fixtures:
        rows=json.loads(fixture.read_text())
        assert len(rows)==31
        assert all(row['exit']==0 and row.get('exact_rust_output',
                   row.get('exact_expected_output',False)) for row in rows)
    for family in ['budget','compact-budget','mask-budget']:
        checks=json.loads((current/f'IR242-{family}__summary.json').read_text())
        assert checks['all_eight_denials_exact']
        denied=[row for row in checks['cases'] if row['case']!='success']
        assert len(denied)==8 and all(row['denial_exact'] and len(row['results'])==3
                                    for row in denied)
        for loader in ['bound','independent','full-catalog']:
            for size in ['first-10','full-5000']:
                receipt=current/f'IR242-{family}__{loader}__success-rust-{size}-run.json'
                assert json.loads(receipt.read_text())['exact_standard_rust_output']
    for prefix in ['frontier-image-probe','mask-frontier-image-probe']:
        raw=json.loads((current/f'IR242-{prefix}-run.json').read_text())
        assert raw['exit']==0
        check=json.loads(raw['stdout'])
        assert check['genuine_legacy_independent_bound_and_compiled_module_accepted']
        assert check['native_entry_never_invoked'] and len(check['forged_cases'])==8
        assert all(row['independent_rejected'] and row['object_bound_rejected']
                   for row in check['forged_cases'])
    assert json.loads((current/'IR242-old-objects-summary.json').read_text())['fresh_old_objects']==2
    runtime=json.loads((current/'IR242-runtime-summary.json').read_text())
    assert runtime['runtime_only_full5000_exact_rust'] and runtime['generated_main_executed_unmodified']
    assert json.loads((current/'IR242-features-scalar-compiler.json').read_text())['exit']==0
    proto=json.loads((current/'IR243-source-inventory.json').read_text())['files']
    overrides=json.loads((current/'IR243-source-overrides.json').read_text())
    assert len(proto)==2468 and len(overrides)==3
    for name,digest in proto.items():
        if name in overrides:
            assert hashlib.sha256((current/overrides[name]).read_bytes()).hexdigest()==digest
        else:
            assert source242[name]==digest
    for ir in [242,243]:
        for variant in ['after-4','reuse-4']:
            audit=json.loads((current/f'IR{ir}-{variant}-native-link-audit.json').read_text())
            old=json.loads((evidence/f'child-entry-facts/ir240-{variant}-native-link-audit.json').read_text())
            assert audit['normalized_text_sha256']==old['normalized_text_sha256']

# Unselected measured compiler trials keep their raw gates and source provenance.
followup = packet / 'integration/evidence/demand-liveness-and-numeric-aliases'
if followup.exists():
    import statistics
    import struct
    trials = json.loads((followup / 'summary.json').read_text())
    assert not trials['selected_changed'] and trials['all_new_timing_candidates_unselected']
    assert not trials['goal_complete']
    latency = json.loads(gzip.decompress((followup / 'latency.json.gz').read_bytes()))
    resources = json.loads(gzip.decompress((followup / 'resources.json.gz').read_bytes()))
    assert len(latency['samples']) == 310 and len(resources['samples']) == 77
    assert latency['execution_order']['seed'] == 990245
    assert resources['execution_order']['seed'] == 9902457
    assert all(row['exact_rust_output'] for row in latency['samples'] + resources['samples'])
    selected_rows = {row['sample']: row for row in latency['samples']
                     if row['variant'] == 'mech-ir242-reuse-4'}
    assert len(selected_rows) == 31
    for name, trial in trials['trials'].items():
        assert trial['timing'] == latency['summary'][name]
        assert trial['resources'] == resources['summary'][name + '-instrumented']
        rows = {row['sample']: row for row in latency['samples'] if row['variant'] == name}
        assert rows.keys() == selected_rows.keys()
        for field in ['post_json_seconds', 'wall_seconds']:
            deltas = [rows[i][field] - selected_rows[i][field] for i in selected_rows]
            paired = trial['paired_vs_selected_IR242'][field]
            assert paired['wins'] == sum(delta < 0 for delta in deltas)
            assert paired['ties'] == sum(delta == 0 for delta in deltas)
            assert paired['median_paired_delta_seconds'] == statistics.median(deltas)
        if name != 'mech-ir242-reuse-4':
            assert not trial['selected']
            assert any(trial['paired_vs_selected_IR242'][field]['wins'] < 21
                       for field in ['post_json_seconds', 'wall_seconds'])
    expected_failures = {
        'batched::axis_tests::legacy_matrix_multiply_artifacts_remain_gpu_lowerable',
        'native_artifact::program::runtime::collections::text_join::compiled_map::owned_tests::complete_unchanged_knucleotide_body_uses_the_owned_emitted_map',
    }
    for ir, passed, selected_count in [(244, 868, 373), (245, 872, 377), (246, 872, 377)]:
        inventory = json.loads((followup / f'IR{ir}-source-inventory.json').read_text())['files']
        assert len(inventory) == 2470
        overrides = json.loads((followup / f'IR{ir}-source-overrides.json').read_text())
        for name, digest in inventory.items():
            if name in overrides:
                data = (followup / overrides[name]).read_bytes()
                assert hashlib.sha256(data).hexdigest() == digest
                actual = subprocess.check_output(
                    ['git', '-c', 'gc.auto=0', 'show', trials['source_commits'][str(ir)] + ':' + name],
                    cwd=packet.parent.parent)
                assert data == actual
            else:
                assert source242[name] == digest
        build = json.loads((followup / f'IR{ir}-build.json').read_text())
        assert len(build['stages']) == 12 and all(stage['exit'] == 0 for stage in build['stages'])
        assert sum(bool(stage.get('reused')) for stage in build['stages']) == 11
        test = json.loads((followup / f'IR{ir}-tests-test-run.json').read_text())
        assert test['expected_failures_exact'] and not test['new_failures']
        assert set(test['failed_tests']) == expected_failures
        stdout = gzip.decompress((followup / f'IR{ir}-tests-test-run.stdout.log.gz').read_bytes()).decode()
        assert sum(line.startswith('test ') and line.endswith(' ... ok')
                   for line in stdout.splitlines()) == passed
        actual_ci = json.loads((followup / f'IR{ir}-tests-ci-selected-inventory-audit.json').read_text())
        assert actual_ci['expected'] == actual_ci['listed'] == actual_ci['executed_passed'] == selected_count
        assert actual_ci['all_names_match'] and len(set(actual_ci['names'])) == selected_count
        for kind, count in [('scalar', 128), ('one-kib', 5)]:
            control = json.loads((followup / f'IR{ir}-controls-{kind}-comparison.json').read_text())
            assert control['rows'] == count and control['all_nontiming_fields_equal_IR240']
            assert not control['all_field_differences']
        fixtures = list(followup.glob(f'IR{ir}-edges-*.json'))
        assert len(fixtures) == (4 if ir == 245 else 2)
        for fixture in fixtures:
            rows = json.loads(fixture.read_text())
            assert len(rows) == 31 and all(row['exit'] == 0 and row.get(
                'exact_rust_output', row.get('exact_expected_output', False)) for row in rows)
        for variant in ['after-4', 'reuse-4']:
            audit = json.loads((followup / f'IR{ir}-{variant}-native-link-audit.json').read_text())
            assert audit['status'] == 'passed'
    for variant in ['after-4', 'reuse-4']:
        before = json.loads((followup / f'IR244-{variant}-native-link-audit.json').read_text())
        after = json.loads((followup / f'IR246-{variant}-native-link-audit.json').read_text())
        assert before['normalized_text_sha256'] == after['normalized_text_sha256']
    for workers in [4, 8]:
        source = gzip.decompress((followup / f'IR245-compact-reuse-{workers}-source.mec.gz').read_bytes())
        change = json.loads((followup / f'IR245-compact-reuse-{workers}-source-change.json').read_text())
        assert hashlib.sha256(source).hexdigest() == change['source_sha256']
        assert change['workers'] == workers
    for variant in ['selected-ir242', 'candidate-ir245']:
        prefix = f'profile-jittered-{variant}-'
        run = json.loads((followup / (prefix + 'run.json')).read_text())
        summary = json.loads((followup / (prefix + 'summary.json')).read_text())
        assert run['exit'] == 0 and run['iterations'] == 1500
        assert run['every_iteration_indices_equal_rust'] and run['final_json_exact_rust']
        assert run['first_and_last_full_json_checked_inside_host'] and run['profile_timings_excluded']
        stats = run['sampler_stats']
        assert stats['suspended'] == stats['resumed'] and summary['sampler_zero_failures']
        assert stats['suspend_failures'] == stats['state_failures'] == stats['resume_failures'] == 0
        samples = gzip.decompress((followup / (prefix + 'output.json.samples.bin.gz')).read_bytes())
        assert len(samples) == struct.calcsize('<5Q2I64Q') * stats['records']
        assert sum(group['samples'] for group in summary['categories'].values()) == stats['records']
        address = json.loads((followup / (prefix + 'profile-code-and-address-audit.json')).read_text())
        assert address['actual_macho_text_vmaddr'] == 0x100000000
        assert address['native_text_exact_to_normal_runner'] and address['sampler_address_formula_verified']

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
