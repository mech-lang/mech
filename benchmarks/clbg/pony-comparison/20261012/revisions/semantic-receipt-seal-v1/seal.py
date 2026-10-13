"""Seal reviewed original Pony work/allocation metadata; execute no native work."""
import argparse
from contextlib import ExitStack
import copy
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
NATIVE = P.with_name('runtime-target-continuation-v1')
ACTUAL = NATIVE / 'jobs/remaining-scored-evidence-v1'
OWNER = C / 'owned-jobs/pony-runtime-target-remaining-evidence-v1'
ROOT = C / 'jobs/pony-runtime-target-closed-review-v1/receipt.json'
ROOT_SHA = 'ef0f4a313d76473bc52dad938be6f6a8245ef0c16b2a592cdd50fd5d5cfe0f0b'
RESULT_SHA = 'c0895574ca921e74941b012def48e0396746447b5cf92320c75b45cc35158f12'
BODY_SHA = 'b5dc136bbef864d67d350a925a7bd96d96db6fd1cb1909f799618cf137bdc450'
OWNER_SHA = '04b15344fe06675b0066df55acf4173f05a17b5cce05636ed922a9bd1b2e14df'
NATIVE_SHA = '799873683afb97f64f8d69f9d9755fe5456f53072b2dd9d6f84a366188deae48'
NATIVE_NOTE_SHA = 'd8f0ca46b57ff7601b019bcfa4493616be3e3e842579847ddbd0e624fb9f7a95'
NATIVE_PLAN_SHA = 'bccfd11f2229cf5287c7ec72c13ece24ca443ade17e1c8477d4e7ee4d87588c9'
PRIOR_ROOT_SHA = '63f6fdffa6aabb772cd9b136819dc46e856869ef965f10efd1df5ff969a8f8b7'
PYTHON = '/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3.12'
SEMANTIC = P.with_name('scored-tree-semantic-review-v1') / 'REVIEW.md'
FAILED = H / 'work-qualification/private-v8-work-v5'
FAILED_SHA = 'b1336e55e14b5db6cf1b7bd6241c550da99d2e086ad07e2cb027c9a89bb1fd83'
MANIFEST_SHA = 'febd942e8c5de659b3dce114d0e7d5224598386059b207c72f82c97603aa37cb'
Q = H / 'qualification/private-v8/report.json'
Q_SHA = '4da05f6dd351609923f5f1ca227d3db2b33a354925614b74f9c5dae39839d159'
PREREQUISITE = P.with_name('work-full-v5') / 'full_prerequisite.py'
PREREQUISITE_SHA = '87c0dcaf5c952366e487e2c0205fc39aacbe3c6ef2c2b39bec8fe99a39a49239'
GATES = ('leaf_path_allocates_fresh_node', 'interior_path_allocates_fresh_node',
         'no_static_or_shared_tree_graph', 'long_root_storage_retained_through_worker_joins')
SYMBOLS = ['Node_ref_create_pZo', 'Node_Alloc', 'Node_box_check_W', 'Node_Trace',
           'Worker_tag__one_o', 'Worker_Trace', 'Main_tag_create_ioo', 'Main_tag__start_o',
           'Main_tag_done_pWo', 'Main_Trace', 'pony_alloc_small', 'ponyint_heap_alloc_small']


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def bind(path, digest, pins):
    path = Path(path).absolute()
    assert re.fullmatch('[a-f0-9]{64}', digest) and sha(path) == digest, str(path)
    assert str(path) not in pins or pins[str(path)] == digest
    pins[str(path)] = digest
    return path


def read(path, digest, pins):
    return json.loads(bind(path, digest, pins).read_bytes())


def new(path, value):
    with path.open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')


def retain(pins):
    root = read(ROOT, ROOT_SHA, pins)
    assert root['status'] == 'closed-complete-Pony-original-scored-evidence-awaiting-semantic-review'
    assert root['verified_pins'] == 10198 and root['profiles'] == 2
    assert (root['result_sha256'], root['body_terminal_sha256'], root['durable_terminal_sha256']) == (RESULT_SHA, BODY_SHA, OWNER_SHA)
    assert root['application_executions'] == root['fixture_reexecutions'] == root['benchmark_runs'] == 0
    assert root['successful_compact_compile_and_entry_decode_reused'] is True
    body = read(ACTUAL / 'terminal.json', BODY_SHA, pins)
    assert body['status'] == 'complete-remaining-scored-evidence-awaiting-semantic-review'
    assert body['body_error'] is None and body['integrity_errors'] == [] and body['common_mutex_held'] is True
    assert body['original_failures_preserved'] is True and body['source_work_qualified'] is False
    assert all(body[key] == 0 for key in ('application_executions', 'fixture_reexecutions',
        'full_performance_runs', 'compact_compile_reexecutions', 'compact_entry_decode_reexecutions'))
    assert all(body['available_evidence_pins'].get(path) == digest for path, digest in body['pins'].items())
    for path, digest in body['available_evidence_pins'].items():
        bind(path, digest, pins)
    ended = read(OWNER / 'terminal.json', OWNER_SHA, pins)
    request, launch, worker, started = [read(OWNER / name, sha(OWNER / name), pins) for name in
        ('request.json', 'launch.json', 'worker-started.json', 'command-started.json')]
    command = [PYTHON, '-I', '-S', '-B', '-u', str(NATIVE / 'continue.py'),
        '--worker-sha256', NATIVE_SHA, '--review-sha256', NATIVE_NOTE_SHA,
        '--plan-sha256', NATIVE_PLAN_SHA, '--closed-review-sha256', PRIOR_ROOT_SHA]
    assert all(item['command'] == command and item['cwd'] == str(H.parents[3]) for item in (request, launch, worker, started))
    assert ended['status'] == 'command-exited-zero-awaiting-report-review' and type(ended['exit_code']) is int and ended['exit_code'] == 0
    assert ended['actual_command_started'] is True and ended['stop_signals'] == [] and not ended.get('error')
    assert ended['automatic_retry'] is ended['qualification_granted'] is request['automatic_retry'] is request['qualification_granted'] is False
    assert request['common_mutex_owned_by_target_command'] is worker['common_mutex_owned_by_target_command'] is True
    assert worker['inherited_private_lease'] is worker['detached_session'] is True
    assert ended['worker_identity'] == worker['worker_identity'] and ended['command_identity'] == started['command_identity']
    assert ended['command_identity_observation_missing'] is started['command_identity_observation_missing'] is False
    assert ended['command_pid'] == started['command_pid'] == 42856 and worker['worker_identity']['pid'] == 42852
    listing = started['command_identity']['listing'].split(None, 7)
    assert listing[:2] == ['42856', '42852'] and listing[7].split() == command
    assert launch['worker_identity']['pid'] == 42852 and launch['worker_identity']['listing'].split()[2:] == worker['worker_identity']['listing'].split()[2:]
    result = read(ACTUAL / 'result.json', RESULT_SHA, pins)
    assert result['status'] == 'remaining-scored-IR-and-ARM-captured-awaiting-semantic-review'
    assert result['original_failures_preserved'] is True and result['source_work_qualified'] is False
    assert result['application_executions'] == result['fixture_reexecutions'] == result['full_performance_runs'] == 0
    assert result['actual_runtime_bitcode']['actual_target'] == root['runtime_merged_target'] == 'arm64-apple-macosx26.0.0'
    assert root['requested_target'] == 'aarch64-apple-macosx14.4.0'
    assert len(root['native_commands']) == 6
    for item in root['native_commands']:
        receipt = read(item['path'], item['sha256'], pins)
        assert receipt['command'] == item['command'] and receipt['inherited_common_mutex'] is True
        assert type(receipt['exit_status']) is type(receipt['final_exit_status']) is int and receipt['exit_status'] == receipt['final_exit_status'] == 0
        assert receipt['leader_reaped'] is receipt['cleanup_complete'] is True
        assert not any(receipt.get(key) for key in ('error', 'error_type', 'cleanup_errors', 'remaining_group_members'))
    return result


def project(args, output, pins):
    result = retain(pins)
    prior = read(FAILED / 'report.json', FAILED_SHA, pins)
    manifest = read(FAILED / 'manifest.json', MANIFEST_SHA, pins)
    q = read(Q, Q_SHA, pins)
    assert prior['status'] == 'stopped' and prior['error_type'] == 'AssertionError' and prior['error'] == 'Exact source IR output must be unambiguous'
    assert prior['manifest'] == str(FAILED / 'manifest.json') and prior['manifest_sha256'] == MANIFEST_SHA
    assert prior['full_performance_runs'] == 0 and prior['reused_actual_work_traces'] == 4 and prior['fresh_actual_work_traces'] == 2
    assert prior['old_attempts_relabelled_as_new'] is False and prior['attempts'] == result['original_six_trace_records']
    assert manifest['qualification'] == str(Q) and manifest['qualification_sha256'] == Q_SHA
    assert manifest['source_work_profiles'] == 20 and manifest['source_work_trace_profiles'] == 6
    assert manifest['reused_fixture_tests'] == 14 and manifest['fresh_fixture_tests'] == 0
    assert manifest['production_operations_preserved'] is True and manifest['workload_code_delegated_to_native'] is False
    assert q['status'] == 'bounded-output-primitives-passed-awaiting-work-worker-and-LOC-review'
    assert all(pins.get(path) == digest for group in (q['pins'], manifest['pins'], prior['pins']) for path, digest in group.items())
    rows = {row['id']: row for row in q['profiles']}
    reviewed = {row['id']: row for row in manifest['reviewed_sources']}
    assert len(rows) == len(q['profiles']) == len(reviewed) == len(manifest['reviewed_sources']) == 20 and set(rows) == set(reviewed)
    for key, row in rows.items():
        assert (reviewed[key]['source'], reviewed[key]['source_sha256']) == (row['source'], row['source_sha256'])
        bind(row['source'], row['source_sha256'], pins); bind(row['command'][0], row['binary_sha256'], pins)
    attempts = prior['attempts']
    assert len(attempts) == len({row['id'] for row in attempts}) == 6
    assert all(sum(row['benchmark'] == benchmark for row in attempts) == 2 for benchmark in ('binarytrees', 'pidigits', 'knucleotide'))
    for row in attempts:
        assert row['qualified'] is row['complete_output_match'] is row['proof']['complete_work_trace'] is True
        assert row['source'] == rows[row['id']]['source'] and row['source_sha256'] == rows[row['id']]['source_sha256']
        bind(row['trace'], row['trace_sha256'], pins); bind(row['trace_source'], row['trace_source_sha256'], pins)
    expected = {key for key, row in rows.items() if row['benchmark'] == 'binarytrees'}
    assert set(result['profiles']) == expected and len(expected) == 2
    profiles = {}
    for key in sorted(expected):
        actual, row = result['profiles'][key], rows[key]
        assert all(actual[gate] is None for gate in GATES), 'Raw pending result must remain unchanged'
        assert actual['source'] == row['source'] and actual['source_sha256'] == row['source_sha256']
        assert actual['original_binary'] == row['command'][0] and actual['original_binary_sha256'] == row['binary_sha256']
        build = read(actual['production_build_receipt'], q['pins'][actual['production_build_receipt']], pins)
        assert build['command'] == actual['production_command'] and build['command'][0] == q['toolchain']['compiler']
        assert build['command'][-1] == str(Path(row['source']).parent) and '--pass' not in build['command']
        assert build['inherited_common_mutex'] is True and type(build['exit_status']) is type(build['final_exit_status']) is int and build['exit_status'] == build['final_exit_status'] == 0
        assert not build.get('cleanup_error') and not build.get('error')
        common = dict(source_sha256=row['source_sha256'], compiler_sha256=q['toolchain']['compiler_sha256'],
            production_compile_command=build['command'], reviewed_symbols=SYMBOLS[:10],
            annotated_leaf_control_flow=['Parent independently allocates each child before depth-minus-one constructor call.',
                'Depth-zero constructor stores None children into its already fresh Node.',
                'Final scored recursive constructor preserves both allocator calls and child pointer stores.',
                'Nonescaping stretch/worker root is compiler-promoted to aligned stack descriptor and child registers; no universal heap claim.',
                'Main long-root heap pointer is stored/traced at320/0x140; final check follows zero pending joins.'],
            semantic_review=dict(path=str(SEMANTIC), sha256=args.semantic_sha256))
        profiles[key] = dict(binary=row['command'][0], binary_sha256=row['binary_sha256'],
            source=row['source'], source_sha256=row['source_sha256'], compiler_sha256=common['compiler_sha256'],
            actual_scored_binary=True, diagnostic_binary_substituted=False,
            production_build_receipt=dict(path=actual['production_build_receipt'], sha256=pins[actual['production_build_receipt']]),
            arm_disassembly=dict(common, path=actual['original_scored_ARM'], sha256=pins[actual['original_scored_ARM']],
                binary_sha256=row['binary_sha256'], reviewed_runtime_symbols=SYMBOLS[10:]),
            llvm_ir=dict(common, path=actual['authored_IR'], sha256=pins[actual['authored_IR']], actual_ir_command=actual['diagnostic_command']),
            temporary_root_storage='compiler-promoted stack descriptor and scalar child pointers; fresh authored Node semantics',
            universal_heap_allocation_claim=False, requested_target=actual['requested_target'],
            actual_runtime_merged_target=actual['actual_runtime_merged_target'], minimum_OS_compatibility_proven=False,
            **dict.fromkeys(GATES, True))
    allocation_path = output / 'allocation.json'
    allocation = dict(status='root-reviewed-unchanged-scored-binary-allocation', receipt_path=str(allocation_path),
        common_mutex_held=True, semantic_review=dict(path=str(SEMANTIC), sha256=args.semantic_sha256),
        closed_native_review=dict(path=str(ROOT), sha256=ROOT_SHA), profiles=profiles,
        separately_reviewed_runtime_bitcode=copy.deepcopy(result['actual_runtime_bitcode']),
        fields_mean_fresh_authored_objects_with_compiler_storage_optimization=True,
        native_reexecutions=0, chart_admission=False, full_performance_runs=0)
    new(allocation_path, allocation); allocation_sha = sha(allocation_path); bind(allocation_path, allocation_sha, pins)
    work = dict(status='native-work-v2-passed-awaiting-scored-binary-allocation-review',
        source_work_profiles=20, source_work_trace_profiles=6, attempts=copy.deepcopy(attempts),
        reviewed_sources=copy.deepcopy(manifest['reviewed_sources']), pins=dict(pins),
        original_failed_work=dict(path=str(FAILED / 'report.json'), sha256=FAILED_SHA, status='stopped'),
        original_manifest=dict(path=str(FAILED / 'manifest.json'), sha256=MANIFEST_SHA),
        retained_historical_trace_counts=dict(original_reused=4, original_new=2, reexecuted_now=0),
        old_attempts_relabelled_as_new=False, fresh_actual_work_traces=0, reused_actual_work_traces=6,
        semantic_review=dict(path=str(SEMANTIC), sha256=args.semantic_sha256),
        scored_tree_raw_allocation_evidence=dict(path=str(ACTUAL / 'result.json'), sha256=RESULT_SHA),
        reviewed_tree_allocation=dict(path=str(allocation_path), sha256=allocation_sha),
        native_commands=0, fixture_reexecutions=0, full_performance_runs=0, chart_admission=False)
    bind(PREREQUISITE, PREREQUISITE_SHA, pins)
    name = 'pony_reviewed_pure_full_prerequisite'
    assert name not in sys.modules
    raw = PREREQUISITE.read_bytes(); assert hashlib.sha256(raw).hexdigest() == PREREQUISITE_SHA
    spec = importlib.util.spec_from_file_location(name, PREREQUISITE)
    helper = importlib.util.module_from_spec(spec); sys.modules[name] = helper
    exec(compile(raw, str(PREREQUISITE), 'exec'), helper.__dict__)
    assert Path(helper.__file__).resolve() == PREREQUISITE.resolve()
    gate = helper.require_tree_proof(q, work, allocation, allocation_sha, sha, lambda path: read(path, pins[str(path)], pins))
    work['pins'] = dict(pins)
    new(output / 'report.json', work)
    new(output / 'outcome.json', dict(status='complete-reviewed-original-work-and-allocation-metadata',
        work_report=dict(path=str(output / 'report.json'), sha256=sha(output / 'report.json')),
        allocation_receipt=dict(path=str(allocation_path), sha256=allocation_sha), tree_prerequisite=gate,
        original_failures_preserved=True, source_work_profiles=20, retained_trace_profiles=6,
        native_commands=0, fixture_reexecutions=0, full_performance_runs=0, chart_admission=False))


def main():
    parser = argparse.ArgumentParser()
    for name in ('worker', 'review', 'semantic'):
        parser.add_argument('--' + name + '-sha256', required=True)
    args = parser.parse_args()
    assert sys.dont_write_bytecode and Path(sys.executable).resolve() == Path(PYTHON).resolve()
    assert all(re.fullmatch('[a-f0-9]{64}', value) for value in vars(args).values())
    def interrupted(signum, frame):
        raise KeyboardInterrupt('Pony metadata seal interrupted by signal ' + str(signum))
    for signum in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP): signal.signal(signum, interrupted)
    print('WAIT common mutex: Pony reviewed metadata only; zero native commands', flush=True)
    with LOCK.open('r+') as common, ExitStack() as leases:
        fcntl.flock(common, fcntl.LOCK_EX)
        opened, named = os.fstat(common.fileno()), LOCK.stat()
        assert (opened.st_dev, opened.st_ino) == (named.st_dev, named.st_ino) and opened.st_ino == 79859134
        for owner in (OWNER, C / 'owned-jobs/pony-scored-bitcode-recovery-v2',
                C / 'owned-jobs/pony-original-evidence-capture-v1', C / 'owned-jobs/pony-two-tree-reviewed-next-stage-v2',
                C / 'owned-jobs/pony-llvm23-reader-qualification-v1'):
            lease = leases.enter_context((owner / 'owner.lock').open('rb'))
            fcntl.flock(lease, fcntl.LOCK_EX | fcntl.LOCK_NB)
            opened, named = os.fstat(lease.fileno()), (owner / 'owner.lock').stat()
            assert (opened.st_dev, opened.st_ino) == (named.st_dev, named.st_ino)
        output = H / 'work-qualification/private-v8-work-reviewed-v1'
        assert not output.exists() and not output.is_symlink()
        assert all(not path.is_symlink() for path in (output.parent, H))
        output.mkdir(parents=True, exist_ok=False)
        pins, failure = {}, None
        try:
            for path, digest in ((P / 'seal.py', args.worker_sha256), (P / 'REVIEW.md', args.review_sha256), (SEMANTIC, args.semantic_sha256)):
                assert not path.stat().st_mode & 0o222
                bind(path, digest, pins)
            project(args, output, pins)
        except BaseException as error:
            failure = dict(error=str(error) or type(error).__name__, error_type=type(error).__name__)
            try:
                with (output / 'error.log').open('x') as stream: stream.write(traceback.format_exc())
            except BaseException as secondary: failure['error_log_write_failure'] = str(secondary)
            raise
        finally:
            available, errors = {}, []
            candidates = dict(pins)
            candidates.update({str(path): None for path in output.rglob('*') if path.is_file() and str(path) not in candidates})
            for path, expected in candidates.items():
                try:
                    available[path] = sha(path)
                    assert expected is None or available[path] == expected
                except BaseException as error: errors.append(dict(path=path, error=str(error) or type(error).__name__))
            new(output / 'terminal.json', dict(status='failed-terminal-integrity' if errors else 'failed-metadata-seal' if failure else 'complete-reviewed-work-and-allocation-metadata',
                body_error=failure, integrity_errors=errors, pins=pins, available_evidence_pins=available,
                common_mutex_held=True, native_commands=0, fixture_reexecutions=0, full_performance_runs=0,
                original_failures_preserved=True, chart_admission=False))
            if errors: raise RuntimeError('Pony metadata terminal integrity failed; available bytes preserved')


if __name__ == '__main__':
    main()
