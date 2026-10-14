"""Seal exact existing Pony inventory/count ownership; no replay or native work."""
import argparse
from contextlib import ExitStack
from copy import deepcopy
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import signal
import sys
import traceback

P = Path(__file__).resolve().parent
H = P.parents[1]
C = H.parents[1] / 'continuation'
LOCK = Path('/tmp/mech-clbg-timing.lock')
PYTHON = '/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3.12'
I = H / 'source-counts/compiler-input-v1'
R = H / 'source-counts/loc18-replay-v1'
I_SHA = '008340b7e28051a6b042c15de713c6e35462cb8a2f108ed5114ed9342d52fb92'
R_SHA = '582d986ddccd9dd93b762f83ad8dca87445600a35286ff09b370b43faad2a511'
Q = H / 'qualification/private-v8/report.json'
Q_SHA = '4da05f6dd351609923f5f1ca227d3db2b33a354925614b74f9c5dae39839d159'
REUSE = H / 'proposal-seals/private-v8/proof-reuse.json'
REUSE_SHA = '0c0c923880cfd5bcea264870919bb23b00b5bd5eb3ffa9b91a69a9474013218c'
FIXTURE_SHA = 'b99a6abfc8880d0647a0a0d737e16f9881fc7fa56d17d23c22ac1dcb5b8d38d0'
LOC18 = H.parents[1] / 'loc-normalization/20261012/jobs/boundary-audit-18/fixture-receipt.json'
LOC18_SHA = 'b00afe351c40fb38e03a99ec81fe3d71bd69a82726f337aa87dfb40cfcaa6b6a'
OWNER = C / 'owned-jobs/pony-reviewed-work-metadata-v1'
WORK = H / 'work-qualification/private-v8-work-reviewed-v1'
ROOT = C / 'jobs/pony-reviewed-work-metadata-closed-review-v1/receipt.json'
ROOT_SHA = '5e434ba59db0bbacaa8aae9bdbfbfbbc99f24ce74842c498724fe2b5de5898d6'
BODY_SHA = 'c9efdff45838c300dacdf822c4e546e7bcd8cd738bd5f886a75b04c1e0e5a1b6'
OWNER_SHA = '951548a6bc39540a27b7117b816e3338cc2521a517dcfbf364ac2719461fb18d'
REQUEST_SHA = '0fb3c3b1e6e2481cad23b4f449fef8708d76ea2af6d79a65248d30dcd08f57a7'
REPORT_SHA = 'aa7786b7cd16565a5ef21cb0bb56e8e0b7ccf77ddf5e98cbcfd1710b6b6aa421'
FRAMEWORK = '/opt/homebrew/Cellar/python@3.14/3.14.8/Frameworks/Python.framework/Versions/3.14/Resources/Python.app/Contents/MacOS/Python'
MODULES = {'inventory': ('input_inventory.py', '6b998e0e9506a186a59fb0a578e0e774fb966a4d894bcd65f3cd0db8b138cd22'),
           'overlay': ('overlay_candidate.py', 'd200064f2b4f05360a4d063c1c517adb681fa0e2c5236ea4981aadf62b620cd7')}
FIXTURE_IDS = tuple('test_loc_inputs.InventoryAndOverlay.' + name for name in (
    'test_package_records_keep_both_streams_and_reject_duplicates', 'test_entire_loaded_authored_package_is_inventoried',
    'test_available_but_unloaded_search_package_does_not_enter_bundle', 'test_unpinned_extra_authored_file_stops_inventory',
    'test_unknown_loaded_package_is_never_a_vendor_exception', 'test_changed_build_or_binary_identity_and_mutex_gate_stop',
    'test_diagnostic_disposition_cannot_override_or_waive_an_unknown_owner', 'test_partial_cohort_and_boolean_counts_cannot_create_overlay'))


def sha(path):
    with Path(path).open('rb') as stream: return hashlib.file_digest(stream, 'sha256').hexdigest()


def bind(path, digest, pins):
    path = Path(path).absolute()
    assert '..' not in path.parts and re.fullmatch('[a-f0-9]{64}', digest) and sha(path) == digest, str(path)
    assert str(path) not in pins or pins[str(path)] == digest
    pins[str(path)] = digest
    return path


def read(path, digest, pins): return json.loads(bind(path, digest, pins).read_bytes())


def absorb(values, pins):
    assert type(values) is dict and values
    for path, digest in values.items():
        assert Path(path).is_absolute()
        bind(path, digest, pins)


def new(path, value):
    with path.open('x') as stream: json.dump(value, stream, indent=2); stream.write('\n')


def cold(name, filename, digest, pins):
    path = bind(I / 'packet' / filename, digest, pins)
    assert name not in sys.modules
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec); sys.modules[name] = module
    exec(compile(path.read_bytes(), str(path), 'exec'), module.__dict__)
    assert Path(module.__file__).resolve() == path.resolve()
    return module


def work_closure(pins):
    root = read(ROOT, ROOT_SHA, pins)
    assert root['status'] == 'closed-qualified-Pony-original-work-and-reviewed-allocation' and root['verified_pins'] == 10217
    assert (root['body_terminal_sha256'], root['durable_terminal_sha256'], root['request_sha256'], root['report_sha256']) == (BODY_SHA, OWNER_SHA, REQUEST_SHA, REPORT_SHA)
    assert root['source_work_profiles'] == 20 and root['retained_trace_profiles'] == 6 and root['canonical_loc'] is None
    assert root['source_work_qualified'] is root['original_failure_preserved'] is root['compiler_promoted_temporary_roots_disclosed'] is root['common_mutex_held'] is True
    assert root['chart_admission'] is False and all(type(root[k]) is int and root[k] == 0 for k in ('new_traces', 'native_processes', 'fixture_reexecutions', 'benchmark_runs'))
    ended = read(OWNER / 'terminal.json', OWNER_SHA, pins)
    request = read(OWNER / 'request.json', REQUEST_SHA, pins)
    launch, worker, started = [read(OWNER / name, sha(OWNER / name), pins) for name in ('launch.json', 'worker-started.json', 'command-started.json')]
    command = [PYTHON, '-I', '-S', '-B', '-u', str(P.with_name('semantic-receipt-seal-v1') / 'seal.py'),
        '--worker-sha256', 'ba6222adef32c934741a2feeb0c9a0b8fc5cbdcb5f96a4a3b309a73d3a6ed4e4',
        '--review-sha256', 'ed6632fadb81dd8a7315f6bad91b26b97a21507e85a588a082837cac58fef491',
        '--semantic-sha256', 'afa38d5056cafc28d3514fcd7ce5e7ceee7c22a567cfd521f729718f241054e4']
    assert all(item['command'] == command and item['cwd'] == str(H.parents[3]) for item in (request, launch, worker, started))
    assert ended['status'] == 'command-exited-zero-awaiting-report-review' and type(ended['exit_code']) is int and ended['exit_code'] == 0
    assert ended['actual_command_started'] is True and ended['stop_signals'] == [] and not ended.get('error')
    assert ended['automatic_retry'] is ended['qualification_granted'] is request['automatic_retry'] is request['qualification_granted'] is False
    assert request['common_mutex_owned_by_target_command'] is worker['common_mutex_owned_by_target_command'] is worker['inherited_private_lease'] is worker['detached_session'] is True
    assert ended['worker_identity'] == worker['worker_identity'] and ended['command_identity'] == started['command_identity']
    assert ended['command_identity_observation_missing'] is started['command_identity_observation_missing'] is False
    assert root['worker_pid'] == launch['worker_identity']['pid'] == worker['worker_identity']['pid'] == 51895
    assert root['command_pid'] == ended['command_pid'] == started['command_pid'] == 51899
    first, last = [item['worker_identity']['listing'].split(None, 7) for item in (launch, worker)]
    assert first[0] == last[0] == '51895' and first[2:] == last[2:]
    assert last[7].split() == [FRAMEWORK, '-B', '-u', str(C / 'launch_durable_v1.py'), '--worker', str(OWNER), '--lease-fd', '3', '--go-fd', '4']
    target = started['command_identity']['listing'].split(None, 7)
    assert target[:2] == ['51899', '51895'] and target[7].split() == command
    bind(FRAMEWORK, 'b3e2d3162eb789101425bbe76810fc5eb0d412d7cd5867d27568dd180ccf5a22', pins)
    body = read(WORK / 'terminal.json', BODY_SHA, pins)
    assert body['status'] == 'complete-reviewed-work-and-allocation-metadata' and body['original_failures_preserved'] is True and body['chart_admission'] is False
    assert body['body_error'] is None and body['integrity_errors'] == [] and body['common_mutex_held'] is True
    assert all(type(body[key]) is int and body[key] == 0 for key in ('native_commands','fixture_reexecutions','full_performance_runs'))
    assert all(body['available_evidence_pins'].get(path) == digest for path, digest in body['pins'].items())
    absorb(body['available_evidence_pins'], pins)
    work = read(WORK / 'report.json', REPORT_SHA, pins)
    assert work['source_work_profiles'] == 20 and work['source_work_trace_profiles'] == 6 and len(work['reviewed_sources']) == 20
    assert work['reused_actual_work_traces'] == 6 and work['fresh_actual_work_traces'] == work['native_commands'] == work['fixture_reexecutions'] == work['full_performance_runs'] == 0
    assert work['old_attempts_relabelled_as_new'] is False
    absorb(work['pins'], pins)
    return work


def old_stage(directory, stage, terminal_sha, pin_sha, pins):
    terminal = read(directory / 'terminal-receipt.json', terminal_sha, pins)
    closed = read(directory / 'terminal-pins.json', pin_sha, pins)
    assert terminal['status'] == closed['terminal_status'] == 'completed-source-evidence-awaiting-root-review' and terminal['stage'] == stage
    assert terminal['actual_common_mutex_held'] is closed['actual_common_mutex_held'] is True
    assert terminal['canonical_LOC_admitted'] is terminal['chart_admission'] is closed['chart_admission'] is closed['automatic_retry'] is False
    assert type(terminal['native_processes']) is int and terminal['native_processes'] == 0 and not terminal.get('error')
    assert terminal['python_version'].startswith('3.14.8 ') and closed['pins'][str(directory / 'terminal-receipt.json')] == terminal_sha
    absorb(closed['pins'], pins); absorb(terminal['runtime_pins'], pins)


def meaning_sources(manifest, replay, pins):
    expected = {str(H / 'qualification' / path): value for path, value in manifest['sources'].items()}
    assert set(expected) == set(replay['source_counts']) and len(expected) == 21
    sources = {}
    for path, (digest, length, physical, canonical) in expected.items():
        raw = replay['source_counts'][path]; metric = raw['metric']
        text = bind(path, digest, pins).read_text()
        assert all(type(value) is int for value in (length, physical, canonical))
        assert raw['source_sha256'] == digest and raw['source_character_length'] == len(text) == length
        assert type(metric['physical_loc']) is type(metric['normalized_loc']) is int
        assert (metric['physical_loc'], metric['normalized_loc']) == (physical, canonical)
        assert metric['language'] == 'pony' and metric['rule_version'] == 'canonical-source-units-20261012-approved-rule-v2' and metric['eligible_for_chart'] is False
        dispositions = []
        for diagnostic in metric['diagnostics']:
            offset = diagnostic['offset']
            if path.endswith('/support-v2/ponytest/aliases.pony'):
                assert diagnostic == dict(kind='pony-extended-type-review', offset=offset,
                    message='Alias/interface/trait/struct members need dedicated source fixtures.', line={103:3,137:4,171:5,205:6}[offset], column=1)
                start, end = offset, {103:136,137:170,171:204,205:242}[offset]
                scope, fragment = 'pony-type', {103:'type PonyTest is current.PonyTest',137:'type TestList is current.TestList',171:'type UnitTest is current.UnitTest',205:'type TestHelper is current.TestHelper'}[offset]
            else:
                assert '/sources/knucleotide/' in path and diagnostic == dict(kind='pony-recover-error-review', offset=90,
                    message='Error-bearing recover requires exact capability/value ownership review.', line=6, column=5)
                start, end, scope, fragment = 90, 101, 'pony-recover', 'recover val'
                assert text[686:689] == 'end' and '          error\n' in text[90:689]
            assert text[start:end] == fragment
            event = dict(kind='header', start=start, end=end, units=1, scope=scope)
            assert event in metric['canonical_events']
            dispositions.append(dict(actual_diagnostic=deepcopy(diagnostic), status='accepted-exact-existing-owner-events',
                actual_source_span=dict(start=start, end=end if scope == 'pony-type' else 689), owner_event_evidence=[event],
                review_identity=manifest['review_identity'], count_override=False))
        assert len(dispositions) == (4 if path.endswith('/support-v2/ponytest/aliases.pony') else 1 if '/sources/knucleotide/' in path else 0)
        sources[path] = dict(status='passed', review_identity=manifest['review_identity'], source=path, source_sha256=digest,
            source_character_length=length, physical_loc=physical, normalized_loc=canonical,
            complete_selected_source_read=True, all_written_owners_retained=True, literal_data_and_callback_ownership_reviewed=True,
            actual_owner_events=deepcopy(metric['canonical_events']), actual_expansions=deepcopy(metric['expansions']),
            diagnostic_dispositions=dispositions, count_override=False, unresolved=[])
    return sources


def project(args, output, pins):
    work = work_closure(pins)
    old_stage(I, 'inventory', '1e4193a924371095afd38349272b7a77a9615f0e335a5586ec80a206c4e86ad4', args.inventory_terminal_pins_sha256, pins)
    old_stage(R, 'replay', '3847bea3b5f8bfe4b04897b68841336c6dbd941481438a865d4a5d1f5bb453f8', args.replay_terminal_pins_sha256, pins)
    inventory = read(I / 'compiler-input-proposals.json', I_SHA, pins)
    replay = read(R / 'source-count-proposals.json', R_SHA, pins)
    q = read(Q, Q_SHA, pins); reuse = read(REUSE, REUSE_SHA, pins)
    assert q['status'] == 'bounded-output-primitives-passed-awaiting-work-worker-and-LOC-review'
    assert q['fresh_unscored_primitive_cases'] == 1 and q['reused_bounded_cases'] == 154
    assert reuse['status'] == 'verified-exact-old-bounded-proof' and reuse['reused_bounded_cases'] == 154
    assert inventory['qualification'] == dict(path=str(Q), sha256=Q_SHA) and inventory['reuse'] == dict(path=str(REUSE), sha256=REUSE_SHA)
    assert inventory['actual_common_mutex_held'] is replay['actual_common_mutex_held'] is True
    assert inventory['unresolved'] == [] and inventory['profile_count'] == 20
    assert replay['inventory'] == dict(path=str(I / 'compiler-input-proposals.json'), sha256=I_SHA)
    for document in (inventory, replay, q, reuse): absorb(document['pins'], pins)
    fixture = read(I / 'fixture-receipt.json', FIXTURE_SHA, pins)
    assert inventory['packet_fixture_receipt'] == replay['packet_fixture_receipt'] == dict(path=str(I / 'fixture-receipt.json'), sha256=FIXTURE_SHA)
    assert fixture['status'] == 'passed' and fixture['actual_common_mutex_held'] is True and fixture['tests_run'] == 8
    assert all(type(fixture[key]) is int and fixture[key] == 0 for key in ('failures','errors','skipped','expected_failures','unexpected_successes','native_processes'))
    assert len(fixture['started_ids']) == len(set(fixture['started_ids'])) == 8
    assert all(sorted(fixture[key]) == sorted(FIXTURE_IDS) for key in ('expected_ids','discovered_ids','started_ids'))
    grammar = read(LOC18, LOC18_SHA, pins)
    assert replay['fixture_receipt'] == dict(path=str(LOC18), sha256=LOC18_SHA, status='passed')
    assert grammar['status'] == 'passed' and grammar['tests_run'] == 672 and grammar['failures'] == grammar['errors'] == 0
    assert grammar['exclusive_mutex_held'] is True and grammar['common_timing_mutex'] == str(LOCK) and grammar['pygments_version'] == '2.19.2'
    absorb(grammar['frozen_dependency_pins'], pins); absorb(replay['grammar_snapshot_pins'], pins); absorb(replay['lexical_dependency_pins'], pins)
    manifest = read(P / 'meaning-review.json', args.meaning_sha256, pins)
    assert manifest['status'] == 'source-reviewed-meaning-awaiting-root-release' and manifest['count_override'] is False
    assert (manifest['inventory_sha256'], manifest['replay_sha256']) == (I_SHA, R_SHA)
    inventory_module = cold('pony_retained_compiler_inventory', *MODULES['inventory'], pins)
    overlay = cold('pony_retained_overlay', *MODULES['overlay'], pins)
    rows = {row['id']: row for row in q['profiles']}; reviewed = {row['id']:row for row in work['reviewed_sources']}
    assert len(rows) == len(q['profiles']) == len(reviewed) == len(work['reviewed_sources']) == len(inventory['profiles']) == 20
    assert set(rows) == set(reviewed) == set(reuse['profiles']) == {row['id'] for row in inventory['profiles']}
    def verify_file(path, bound):
        path = str(Path(path).resolve(strict=True)); digest = bound[path]
        bind(path,digest,pins)
        return digest
    profiles = {}
    for row in inventory['profiles']:
        key = row['id']; original = rows[key]
        assert reviewed[key]['source'] == row['source'] and reviewed[key]['source_sha256'] == row['source_sha256']
        receipt = Path(row['production_build_receipt']); build = read(receipt, pins[str(receipt)], pins)
        actual = inventory_module.inventory_one(original, reuse['profiles'][key], build,
            receipt.with_suffix('.stdout').read_text(), receipt.with_suffix('.stderr').read_text(), pins,
            verify_file)
        assert actual == {field:value for field,value in row.items() if field not in ('actual_build_evidence','actual_build_streams')}
        expected = [(row['source'], row['source_sha256'], 'application')]
        if row['benchmark'] == 'pidigits': expected.append((str(H / 'qualification/private-v3/source-snapshot/support-v2/ponytest/aliases.pony'), 'deccab60eac37f744832561ce145afb6bec0abce21a10c04298c7d668094bfbe', 'application-helper'))
        assert [(item['path'],item['sha256'],item['role']) for item in row['files']] == expected
        profiles[key] = dict(status='passed', review_identity=manifest['review_identity'], complete_compiler_packages_reviewed=True,
            source_sha256=row['source_sha256'], binary_sha256=row['binary_sha256'], actual_original_compile_command=deepcopy(row['actual_original_compile_command']),
            actual_package_inventory=deepcopy(row['packages']), actual_authored_files=[{field:item[field] for field in ('path','sha256')} for item in row['files']])
    review = dict(status='passed-exact-compiler-role-and-selected-source-review', inventory=deepcopy(replay['inventory']),
        replay=dict(path=str(R / 'source-count-proposals.json'), sha256=R_SHA), sources=meaning_sources(manifest, replay, pins), profiles=profiles,
        meaning_manifest=dict(path=str(P / 'meaning-review.json'), sha256=args.meaning_sha256), count_override=False, pins=dict(pins))
    assert replay['unresolved'] == [dict(source=path, diagnostic=issue) for path,value in replay['source_counts'].items() for issue in value['metric']['diagnostics']]
    assert len(replay['unresolved']) == 6
    review_path = output / 'semantic-review.json'; new(review_path, review); bind(review_path, sha(review_path), pins)
    replay['self_evidence'] = review['replay']
    payload = overlay.construct(inventory, replay, review, review['inventory'], dict(evidence_path=str(review_path), evidence_sha256=pins[str(review_path)]))
    input_path = output / 'compiler-bound-input-inventory.json'
    new(input_path, dict(status='qualified-exact-authored-compiler-inputs', language='pony', profile_count=20,
        original_compiler_inventory=review['inventory'], source_role_review=dict(path=str(review_path),sha256=pins[str(review_path)]), profiles=payload['qualified_inputs'], pins=dict(pins)))
    bind(input_path, sha(input_path), pins)
    for value in payload['qualified_inputs'].values(): value.update(evidence_path=str(input_path), evidence_sha256=pins[str(input_path)])
    overlay_path = output / 'qualified-loc-overlay.json'
    new(overlay_path, dict(schema_version=1, language='pony', status='qualified-exact-source-LOC', profile_count=20,
        complete_authored_source_bundles=True, unresolved=[], normalization_policy=replay['normalization_policy'], fixture_receipt=replay['fixture_receipt'],
        replay_receipts=[review['replay']], grammar_snapshot_pins=replay['grammar_snapshot_pins'], compiler_bound_input_inventory=payload['qualified_inputs'],
        profiles=payload['profiles'], pins=dict(pins), canonical_LOC_admitted=True, chart_admission=False))
    bind(overlay_path, sha(overlay_path), pins)
    new(output / 'outcome.json', dict(status='complete-reviewed-retained-Pony20-LOC-overlay', qualified_overlay=dict(path=str(overlay_path),sha256=pins[str(overlay_path)]),
        profiles=20, unique_authored_inputs=21, authored_bundle_entries=22, original_diagnostics_preserved=6,
        original_inventory_fixtures_reused=8, original_grammar_fixtures_reused=672, native_commands=0, count_replays=0, fixture_reexecutions=0,
        live_dataset_mutations=0, chart_admission=False, original_inputs_unchanged=True, pins=dict(pins)))


def main():
    parser = argparse.ArgumentParser()
    for name in ('worker','review','meaning','inventory-terminal-pins','replay-terminal-pins'): parser.add_argument('--'+name+'-sha256', required=True)
    args = parser.parse_args()
    assert sys.dont_write_bytecode and Path(sys.executable).resolve() == Path(PYTHON).resolve()
    assert all(re.fullmatch('[a-f0-9]{64}', value) for value in vars(args).values())
    def interrupted(signum, frame): raise KeyboardInterrupt('Pony LOC metadata interrupted by signal '+str(signum))
    for signum in (signal.SIGINT,signal.SIGTERM,signal.SIGHUP): signal.signal(signum,interrupted)
    print('WAIT common mutex: retain Pony8/672 and original source counts; no native work', flush=True)
    with LOCK.open('r+') as common, ExitStack() as leases:
        fcntl.flock(common, fcntl.LOCK_EX)
        opened, named = os.fstat(common.fileno()), LOCK.stat()
        assert (opened.st_dev,opened.st_ino) == (named.st_dev,named.st_ino) and opened.st_ino == 79859134
        for owner in (OWNER, C/'owned-jobs/pony-runtime-target-remaining-evidence-v1', C/'owned-jobs/pony-scored-bitcode-recovery-v2',
                      C/'owned-jobs/pony-original-evidence-capture-v1', C/'owned-jobs/pony-two-tree-reviewed-next-stage-v2', C/'owned-jobs/pony-llvm23-reader-qualification-v1'):
            lease = leases.enter_context((owner/'owner.lock').open('rb')); fcntl.flock(lease,fcntl.LOCK_EX|fcntl.LOCK_NB)
            opened,named = os.fstat(lease.fileno()),(owner/'owner.lock').stat()
            assert (opened.st_dev,opened.st_ino) == (named.st_dev,named.st_ino)
        output = H/'source-counts/loc-reviewed-seal-v1'
        assert not output.exists() and not any(path.is_symlink() for path in (output,output.parent,P))
        output.mkdir(exist_ok=False); pins, body = {},None
        try:
            for path,digest in ((P/'seal.py',args.worker_sha256),(P/'REVIEW.md',args.review_sha256),(P/'meaning-review.json',args.meaning_sha256)):
                assert not path.stat().st_mode & 0o222
                bind(path,digest,pins)
            project(args,output,pins)
        except BaseException as error:
            body = dict(error=str(error) or type(error).__name__, error_type=type(error).__name__)
            try:
                with (output/'error.log').open('x') as stream: stream.write(traceback.format_exc())
            except BaseException as secondary: body['error_log_write_failure']=str(secondary)
            raise
        finally:
            available, errors = {},[]
            candidates=dict(pins); candidates.update({str(path):None for path in output.rglob('*') if path.is_file() and str(path) not in candidates})
            for path,expected in candidates.items():
                try:
                    available[path]=sha(path); assert expected is None or available[path]==expected
                except BaseException as error: errors.append(dict(path=path,error=str(error) or type(error).__name__))
            new(output/'terminal.json',dict(status='failed-terminal-integrity' if errors else 'failed-Pony-LOC-metadata' if body else 'completed-Pony-LOC-metadata',
                body_error=body,integrity_errors=errors,pins=pins,available_evidence_pins=available,common_mutex_held=True,
                native_commands=0,count_replays=0,fixture_reexecutions=0,live_dataset_mutations=0,chart_admission=False))
            if errors: raise RuntimeError('Pony LOC metadata integrity failed; partial evidence retained')


if __name__ == '__main__': main()
