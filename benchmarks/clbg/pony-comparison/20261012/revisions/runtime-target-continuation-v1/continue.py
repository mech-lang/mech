"""Six missing Pony evidence commands; retain successful compact compile/decode."""
import argparse
from contextlib import ExitStack
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import sys
import traceback

P = Path(__file__).resolve().parent
H = P.parents[1]
C = H.parents[1] / 'continuation'
LOCK = Path('/tmp/mech-clbg-timing.lock')
BASE = P.with_name('native-evidence-recovery-v2')
BASE_SHA = '1dc8df79aed697a907f3dcae73b6ed37e117669e78002a3b42e2370ec53d2cfa'
BASE_NOTE_SHA = '2a80012e315e9d10367ead4b49fa18e4a5f46c408a704f56050c8b0899a83bd4'
FAILED = BASE / 'jobs/scored-bitcode-v2'
OWNER = C / 'owned-jobs/pony-scored-bitcode-recovery-v2'
ROOT_REVIEW = C / 'jobs/pony-scored-recovery-v2-closed-review/receipt.json'
BODY_SHA = '0f3248e5a2f135906c6f66588e989fcf567a88954c407fa1842fed1a0a0dcb39'
OWNER_SHA = '17904a3e495a5b6c328b25679d14d2ff75c464bdbd50b555b2984d2ff7eb46b8'
RUNTIME_SHA = '95e60711be95591e4b5f5e3b35e7daef1c28de30fb8bb5b46c74d89d2309438a'
REQUESTED = 'aarch64-apple-macosx14.4.0'
MERGED = 'arm64-apple-macosx26.0.0'
LAYOUT = 'e-m:o-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-n32:64-S128-Fn32'
CAPTURE_OUTCOME = 'fbc2c1dfd450759c893b51723a2977d16ffaf8e63c95a48fc70dcc9cca4e01e2'
CAPTURE_TERMINAL = '2fbdfe6cbadad6ab6514ae43a427d46983df716b713aab6916b2be0c4d00bc32'
READER_RECEIPT = '054c7a9e376f4dc883e5e7cab60dd6193cd67d4080ee42099d4fe7c07978edca'
READER_BODY = '529cf74c0f08e7dd632167b1288e85202817c717f8e0edd26ae3f56c3743b13f'
READER_OWNER = '5f35ef7c2985880f62519168002436288e24831c4b9d441ed6eb9bf6867b0062'


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def bind(path, digest, pins):
    path = Path(path).absolute()
    assert sha(path) == digest and (str(path) not in pins or pins[str(path)] == digest), str(path)
    pins[str(path)] = digest
    resolved = path.resolve(strict=True)
    if resolved != path:
        assert sha(resolved) == digest and (str(resolved) not in pins or pins[str(resolved)] == digest)
        pins[str(resolved)] = digest
    return path


def read(path, digest, pins):
    return json.loads(bind(path, digest, pins).read_bytes())


def verify(pins):
    for path, digest in pins.items():
        assert sha(path) == digest, ('Retained evidence changed', path)


def new(path, value):
    with path.open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')


def cold(path, digest, name, pins):
    path = bind(path, digest, pins).resolve(strict=True)
    assert name not in sys.modules
    raw = path.read_bytes()
    assert hashlib.sha256(raw).hexdigest() == digest
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    assert Path(module.__file__).resolve() == path and sys.modules[name] is module
    return module


def original_args(base):
    return argparse.Namespace(capture_outcome_sha256=CAPTURE_OUTCOME,
        capture_owner_terminal_sha256=CAPTURE_TERMINAL,
        reader_receipt=base.ORIGINAL_READER_PACKET / 'jobs/reader-llvm23-v1/reader.json',
        reader_sha256=READER_RECEIPT, reader_terminal_sha256=READER_BODY,
        reader_owner=C / 'owned-jobs/pony-llvm23-reader-qualification-v1',
        reader_owner_terminal_sha256=READER_OWNER)


def prior_failure(args, pins):
    review = read(ROOT_REVIEW, args.closed_review_sha256, pins)
    assert review['status'] == 'closed-failed-compact-IR-target-literal-check' and review['verified_pins'] == 10142
    assert review['body_terminal_sha256'] == BODY_SHA and review['durable_terminal_sha256'] == OWNER_SHA
    assert review['application_executions'] == review['benchmark_runs'] == 0
    assert review['reader_qualification_reexecuted'] is False and review['prior_successful_outputs_preserved'] is True
    assert review['actual_IR_target'] == MERGED and review['requested_production_target'] == REQUESTED
    body = read(FAILED / 'terminal.json', BODY_SHA, pins)
    assert body['status'] == 'failed-diagnostic-evidence' and body['integrity_errors'] == []
    assert body['body_error']['error_type'] == 'AssertionError' and body['common_mutex_held'] is True
    for group in (body['pins'], body['available_evidence_pins']):
        for path, digest in group.items():
            bind(path, digest, pins)
    assert all(body[name] == 0 for name in ('application_executions', 'fixture_reexecutions', 'full_performance_runs'))
    error = (FAILED / 'error.log').read_text()
    assert 'recover.py", line 289, in recover' in error and 'assert \'target triple = "' + REQUESTED + '"\' in headers' in error
    done = read(OWNER / 'terminal.json', OWNER_SHA, pins)
    request, launch, worker, start = [read(OWNER / name, sha(OWNER / name), pins) for name in
        ('request.json', 'launch.json', 'worker-started.json', 'command-started.json')]
    base = cold(BASE / 'recover.py', BASE_SHA, 'pony_target_original_recovery', pins)
    bind(BASE / 'REVIEW.md', BASE_NOTE_SHA, pins)
    oldargs = original_args(base)
    command = ['/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3.12',
        '-I', '-S', '-B', '-u', str(BASE / 'recover.py'), '--recover', '--revision', 'scored-bitcode-v2',
        '--worker-sha256', BASE_SHA, '--review-sha256', BASE_NOTE_SHA,
        '--capture-outcome-sha256', CAPTURE_OUTCOME, '--capture-owner-terminal-sha256', CAPTURE_TERMINAL,
        '--reader-receipt', str(oldargs.reader_receipt), '--reader-sha256', READER_RECEIPT,
        '--reader-terminal-sha256', READER_BODY, '--reader-owner', str(oldargs.reader_owner),
        '--reader-owner-terminal-sha256', READER_OWNER]
    assert all(item['command'] == command and item['cwd'] == str(H.parents[3]) for item in (request, launch, worker, start))
    assert done['status'] == 'command-failed' and type(done['exit_code']) is int and done['exit_code'] == 1
    assert done['actual_command_started'] is True and done['stop_signals'] == []
    assert done['automatic_retry'] is done['qualification_granted'] is False and not done.get('error')
    assert done['worker_identity'] == worker['worker_identity'] and done['command_identity'] == start['command_identity']
    assert worker['inherited_private_lease'] is worker['common_mutex_owned_by_target_command'] is True
    assert start['command_pid'] == done['command_pid'] == 33589 and worker['worker_identity']['pid'] == 33585
    assert start['command_identity_observation_missing'] is done['command_identity_observation_missing'] is False
    image = start['command_identity']['listing'].split(None, 7)
    assert image[:2] == ['33589', '33585'] and image[7].split() == command
    assert len(review['native_commands']) == 2
    verify(pins)
    return base, oldargs, review


def reused_reader(base, args, pins):
    actual = read(args.reader_receipt, READER_RECEIPT, pins)
    assert actual['status'] == 'qualified-current-LLVM23-reader-and-transitive-link-images'
    assert actual['reader'] == str(base.READER) and actual['reader_sha256'] == base.READER_SHA and actual['version'] == '23.1.3'
    end = read(args.reader_receipt.parent / 'terminal.json', READER_BODY, pins)
    assert end['status'] == 'complete-reader-evidence-awaiting-root-review' and end['body_error'] is None and end['integrity_errors'] == []
    assert end['available_evidence_pins'][str(args.reader_receipt)] == READER_RECEIPT
    command = base.closed(args.reader_owner, READER_OWNER, pins)['command']
    expected = ['/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3.12',
        '-I', '-S', '-B', '-u', str(base.ORIGINAL_READER_PACKET / 'recover.py'), '--qualify-reader',
        '--revision', 'reader-llvm23-v1', '--worker-sha256', base.ORIGINAL_READER_WORKER_SHA,
        '--review-sha256', base.ORIGINAL_READER_REVIEW_SHA, '--capture-outcome-sha256', CAPTURE_OUTCOME,
        '--capture-owner-terminal-sha256', CAPTURE_TERMINAL]
    assert command == expected
    bind(base.ORIGINAL_READER_PACKET / 'recover.py', base.ORIGINAL_READER_WORKER_SHA, pins)
    bind(base.ORIGINAL_READER_PACKET / 'REVIEW.md', base.ORIGINAL_READER_REVIEW_SHA, pins)
    for group in (actual['pins'], end['pins'], end['available_evidence_pins']):
        for path, digest in group.items():
            base.bind(path, digest, pins)
    verify(pins)


def successful_native(path, digest, command, pins):
    item = read(path, digest, pins)
    assert item['command'] == command and item['inherited_common_mutex'] is item['preparation_only'] is item['never_chart_eligible'] is True
    assert type(item['exit_status']) is type(item['final_exit_status']) is int and item['exit_status'] == item['final_exit_status'] == 0
    assert item['leader_reaped'] is item['cleanup_complete'] is True
    assert item['cleanup_errors'] == item['remaining_group_members'] == [] and not item.get('error')
    return item


def production(base, row, q, pins):
    source, binary = Path(row['source']), Path(row['command'][0])
    base.bind(source, row['source_sha256'], pins)
    base.bind(binary, row['binary_sha256'], pins)
    build = read(row['production_build_receipt'], q['pins'][row['production_build_receipt']], pins)
    assert type(build['exit_status']) is type(build['final_exit_status']) is int and build['exit_status'] == build['final_exit_status'] == 0
    assert not build.get('cleanup_error') and not build.get('error')
    command = list(build['command'])
    assert command[0] == q['toolchain']['compiler'] and '--pass' not in command
    assert command[-1] == str(source.parent) and command[command.index('--bin-name') + 1] == row['id']
    assert command[command.index('--output') + 1] == str(binary.parent)
    assert command.count('--triple') == 1 and command[command.index('--triple') + 1] == REQUESTED
    assert command.count('--sysroot') == 1 and command[command.index('--sysroot') + 1] == '/Library/Developer/CommandLineTools/SDKs/MacOSX14.4.sdk'
    base.bind(command[0], q['toolchain']['compiler_sha256'], pins)
    assert set(source.parent.glob('*.pony')) == {source}
    roots = [Path(command[0]).parent.parent / 'packages'] + [Path(command[i + 1]) for i, part in enumerate(command) if part == '--path']
    for root in roots:
        root = root.resolve(strict=True)
        current = {str(path) for path in root.rglob('*.pony') if path.is_file()}
        expected = {path for path in q['pins'] if Path(path).is_relative_to(root) and Path(path).suffix == '.pony'}
        assert current == expected, ('Changed compiler input set', str(root))
    env = base.environment(row)
    env['SDKROOT'] = command[command.index('--sysroot') + 1]
    return source, binary, build, command, env


def emitted_files(directory, source):
    bitcode = directory / 'bitcode'
    emitted = sorted(bitcode.iterdir())
    lines = (directory / 'compile/scored-bitcode.stderr').read_text().splitlines()
    assert lines.count('Building ' + str(source.parent) + ' -> ' + str(source.parent)) == 1
    assert lines.count('Merging runtime') == 1 and lines.count('Emitting 17 per-package modules') == 1
    writes = [line[8:] for line in lines if line.startswith('Writing ')]
    assert len(emitted) == len(writes) == len(set(writes)) == 17
    assert set(writes) == {str(path) for path in emitted}
    assert all(path.is_file() and not path.is_symlink() and path.suffix == '.bc' for path in emitted)
    return emitted


def headers(path, bitcode, triple, authored):
    raw = path.read_text()
    top = raw.splitlines()[:5]
    assert 'target triple = "' + triple + '"' in top and 'target datalayout = "' + LAYOUT + '"' in top
    assert 'source_filename = "' + str(bitcode) + '"' in top
    if authored:
        assert all(re.search(r'^define .*@' + name + '_', raw, re.M) for name in ('Node', 'Worker', 'Main'))
    else:
        assert not re.search(r'^define .*@(Node|Worker|Main)_', raw, re.M)
    return top


def continue_evidence(base, oldargs, review, output, fd, pins):
    original, q, prep, service, guard, harness = base.originals(oldargs, pins)
    reused_reader(base, oldargs, pins)
    rows = {row['id']: row for row in q['profiles'] if row['benchmark'] == 'binarytrees'}
    assert set(rows) == set(base.IDS) and len(rows) == 2
    prepared = {key: production(base, row, q, pins) for key, row in rows.items()}
    compiler = Path(q['toolchain']['compiler'])
    runtime = compiler.parent.parent / 'lib/armv8-a/libponyrt.bc'
    assert not (compiler.parent / 'libponyrt.bc').exists(), 'Earlier compiler-bin runtime shadow'
    bind(runtime, RUNTIME_SHA, pins)
    assert q['pins'][str(runtime)] == RUNTIME_SHA
    runtime_ir = output / 'runtime.ll'
    base.owned(prep, service, guard, harness, [str(base.READER), str(runtime), '-o', str(runtime_ir)],
        base.environment(), output / 'runtime-decode', fd, 120, 'runtime-ir')
    assert runtime_ir.is_file() and not runtime_ir.is_symlink()
    bind(runtime_ir, sha(runtime_ir), pins)
    raw_runtime = runtime_ir.read_text()
    assert 'target triple = "' + MERGED + '"' in raw_runtime.splitlines()[:5]
    assert not re.search(r'^define .*@(Node|Worker|Main)_', raw_runtime, re.M)
    assert re.search(r'^define .*@pony_alloc_small\(', raw_runtime, re.M)
    profiles = {}
    for identifier in base.IDS:
        row = rows[identifier]
        source, binary, build, command, env = prepared[identifier]
        directory = output / identifier
        if identifier == base.IDS[0]:
            retained = FAILED / identifier
            expected = list(command)
            expected[expected.index('--output') + 1] = str(retained / 'bitcode')
            expected[-1:-1] = ['--pass', 'bitcode']
            compiled, decoded = review['native_commands']
            assert compiled['path'] == str(retained / 'compile/scored-bitcode.json') and compiled['command'] == expected
            successful_native(compiled['path'], compiled['sha256'], expected, pins)
            emitted = emitted_files(retained, source)
            main = retained / 'bitcode' / (identifier + '.main.bc')
            builtin = retained / 'bitcode' / (identifier + '.builtin.bc')
            llvm = retained / 'authored-entry.ll'
            decode = [str(base.READER), str(main), '-o', str(llvm)]
            assert decoded['path'] == str(retained / 'decode/entry-ir.json') and decoded['command'] == decode
            successful_native(decoded['path'], decoded['sha256'], decode, pins)
            assert pins[str(main)] == '0d4e46f37c1b4e18ebbe6be018f7494a33526ad5fe37ba07c305418848bacc5c'
            assert pins[str(builtin)] == 'c706e91f5921a59c08e99350ab6c37f9992a1eadc8f4a9047588f75e347a8a6d'
            assert pins[str(llvm)] == '190ab8b1a59c0b98abb33193e13c8e255ba11ac88808eeb0fe1e940312d96d61'
            command = expected
        else:
            bitcode = directory / 'bitcode'
            bitcode.mkdir(parents=True)
            command[command.index('--output') + 1] = str(bitcode)
            command[-1:-1] = ['--pass', 'bitcode']
            base.owned(prep, service, guard, harness, command, env, directory / 'compile', fd, 600, 'scored-bitcode')
            emitted = emitted_files(directory, source)
            main, builtin = [bitcode / (identifier + suffix) for suffix in ('.main.bc', '.builtin.bc')]
            llvm = directory / 'authored-entry.ll'
            base.owned(prep, service, guard, harness, [str(base.READER), str(main), '-o', str(llvm)],
                base.environment(), directory / 'decode', fd, 120, 'entry-ir')
        assert main in emitted and builtin in emitted and main != builtin and sha(main) != sha(builtin)
        top = headers(llvm, main, MERGED, True)
        builtin_ir = directory / 'builtin.ll'
        directory.mkdir(parents=True, exist_ok=True)
        base.owned(prep, service, guard, harness, [str(base.READER), str(builtin), '-o', str(builtin_ir)],
            base.environment(), directory / 'decode-builtin', fd, 120, 'builtin-ir')
        builtin_top = headers(builtin_ir, builtin, REQUESTED, False)
        if identifier == base.IDS[0]:
            assert original['original_successful_scored_ARM_command']['command'] == ['/usr/bin/otool', '-tvV', str(binary)]
            arm = H / 'work-qualification/private-v8-work-v5/scored-tree-allocation' / identifier / 'arm/scored-arm.stdout'
            bind(arm, '3053732417af086c0754d2c82a7fc1c1cb92a4046f0d40b9ef9dec400e5fe289', pins)
        else:
            base.owned(prep, service, guard, harness, ['/usr/bin/otool', '-tvV', str(binary)],
                env, directory / 'arm', fd, 120, 'scored-arm')
            arm = directory / 'arm/scored-arm.stdout'
        assert arm.stat().st_size > 0
        profiles[identifier] = dict(source=str(source), source_sha256=row['source_sha256'],
            original_binary=str(binary), original_binary_sha256=row['binary_sha256'],
            production_build_receipt=row['production_build_receipt'], production_command=build['command'],
            diagnostic_command=command, emitted_bitcode_files=[str(path) for path in emitted],
            authored_entry_bitcode=str(main), separate_builtin_bitcode=str(builtin), authored_IR=str(llvm),
            module_headers=top, separate_builtin_IR=str(builtin_ir), builtin_module_headers=builtin_top,
            requested_target=REQUESTED, actual_runtime_merged_target=MERGED, original_scored_ARM=str(arm),
            compact_compile_and_entry_decode_reused=identifier == base.IDS[0],
            leaf_path_allocates_fresh_node=None, interior_path_allocates_fresh_node=None,
            no_static_or_shared_tree_graph=None, long_root_storage_retained_through_worker_joins=None)
        for path, digest in prep.pin_tree(directory).items():
            base.bind(path, digest, pins)
        verify(pins)
    verify(pins)
    return dict(status='remaining-scored-IR-and-ARM-captured-awaiting-semantic-review', profiles=profiles,
        actual_runtime_bitcode=dict(path=str(runtime), sha256=RUNTIME_SHA, decoded=str(runtime_ir),
            decoded_sha256=pins[str(runtime_ir)], actual_target=MERGED), requested_production_target=REQUESTED,
        original_six_trace_records=original['original_trace_records'], reader_receipt=str(oldargs.reader_receipt),
        reader_sha256=READER_RECEIPT, prior_failed_body=dict(path=str(FAILED / 'terminal.json'), sha256=BODY_SHA),
        compact_compile_reexecutions=0, compact_entry_decode_reexecutions=0, reader_qualification_reexecutions=0,
        actual_missing_evidence_commands=6, source_work_qualified=False, application_executions=0,
        fixture_reexecutions=0, full_performance_runs=0, diagnostic_executable_substituted=False,
        original_failures_preserved=True, final_LTO_ARM_review_required=True, minimum_OS_compatibility_proven=False)


def main():
    parser = argparse.ArgumentParser()
    for name in ('worker', 'review', 'plan', 'closed-review'):
        parser.add_argument('--' + name + '-sha256', required=True)
    args = parser.parse_args()
    assert sys.dont_write_bytecode and all(re.fullmatch('[a-f0-9]{64}', value) for value in vars(args).values())
    print('WAIT common mutex: six missing Pony evidence commands; compact successes reused', flush=True)
    with LOCK.open('r+') as common, ExitStack() as leases:
        fcntl.flock(common, fcntl.LOCK_EX)
        opened, named = os.fstat(common.fileno()), LOCK.stat()
        assert (opened.st_dev, opened.st_ino) == (named.st_dev, named.st_ino) and opened.st_ino == 79859134
        for owner in (OWNER, C / 'owned-jobs/pony-original-evidence-capture-v1',
                C / 'owned-jobs/pony-two-tree-reviewed-next-stage-v2', C / 'owned-jobs/pony-llvm23-reader-qualification-v1'):
            lease = leases.enter_context((owner / 'owner.lock').open('r+'))
            fcntl.flock(lease, fcntl.LOCK_EX | fcntl.LOCK_NB)
            opened, named = os.fstat(lease.fileno()), (owner / 'owner.lock').stat()
            assert (opened.st_dev, opened.st_ino) == (named.st_dev, named.st_ino)
        output = P / 'jobs/remaining-scored-evidence-v1'
        output.mkdir(parents=True, exist_ok=False)
        pins, failure = {}, None
        try:
            for path, digest in ((P / 'continue.py', args.worker_sha256), (P / 'REVIEW.md', args.review_sha256), (P / 'PLAN.json', args.plan_sha256)):
                assert not path.stat().st_mode & 0o222
                bind(path, digest, pins)
            assert all(not getattr(item, '__file__', None) or not Path(item.__file__).resolve().is_relative_to(H.parents[1]) or
                Path(item.__file__).resolve() == P / 'continue.py' for item in list(sys.modules.values()))
            base, oldargs, review = prior_failure(args, pins)
            result = continue_evidence(base, oldargs, review, output, common.fileno(), pins)
            new(output / 'result.json', result)
        except BaseException as error:
            failure = dict(error=str(error) or type(error).__name__, error_type=type(error).__name__)
            try:
                with (output / 'error.log').open('x') as stream:
                    stream.write(traceback.format_exc())
            except BaseException as journal_error:
                failure['error_log_write_failure'] = str(journal_error)
            raise
        finally:
            available, errors = {}, []
            candidates = dict(pins)
            candidates.update({str(path): None for path in output.rglob('*') if path.is_file() and str(path) not in candidates})
            for path, expected in candidates.items():
                try:
                    available[path] = sha(path)
                    assert expected is None or available[path] == expected
                except BaseException as error:
                    errors.append(dict(path=path, error=str(error) or type(error).__name__))
            status = 'failed-terminal-integrity' if errors else 'failed-continuation-evidence' if failure else 'complete-remaining-scored-evidence-awaiting-semantic-review'
            new(output / 'terminal.json', dict(status=status, body_error=failure, integrity_errors=errors,
                pins=pins, available_evidence_pins=available, common_mutex_held=True, source_work_qualified=False,
                application_executions=0, fixture_reexecutions=0, full_performance_runs=0,
                compact_compile_reexecutions=0, compact_entry_decode_reexecutions=0, original_failures_preserved=True))
            if errors:
                raise RuntimeError('Continuation integrity failed; all readable evidence retained')


if __name__ == '__main__':
    main()
