"""Recover only missing scored Pony bitcode/IR/ARM; no application execution."""
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
ORIGINAL_READER_PACKET = P.with_name('native-evidence-recovery-v1')
ORIGINAL_READER_WORKER_SHA = '991aab6eaa9694c3b1e0a85d3b3541625fc0a577538dd600316a124588d14cf2'
ORIGINAL_READER_REVIEW_SHA = '62aad6530a688a47861f68e872c8f9212d5c3dd82f6d57c2da33d756b345a803'
H = P.parents[1]
C = H.parents[1] / 'continuation'
LOCK = Path('/tmp/mech-clbg-timing.lock')
CAPTURE = H / 'revisions/scored-allocation-recovery-v1/jobs/original-six-traces-v1'
CAPTURE_OWNER = C / 'owned-jobs/pony-original-evidence-capture-v1'
FAILED_OWNER = C / 'owned-jobs/pony-two-tree-reviewed-next-stage-v2'
Q = H / 'qualification/private-v8/report.json'
Q_SHA = '4da05f6dd351609923f5f1ca227d3db2b33a354925614b74f9c5dae39839d159'
PROPOSAL_SHA = 'e4fb58118389cc53c196a6c743a2a49e1a8e5d96e96be647364e94e75af2955d'
PREPARATION = H / 'guarded_prepare_v6.py'
SERVICE = H / 'revisions/work-full-v5/owned_service_work_v2.py'
READER = Path('/opt/homebrew/Cellar/llvm/23.1.3_1/bin/llvm-dis')
READER_SHA = 'a5f9b5c9bdd04dd73e3778a0dc46b89579dfc7686b292e9ff0db8881b492d693'
IDS = ('pony-binarytrees-compact-precedence-v4', 'pony-binarytrees-optimized-precedence-v4')


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def new(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')


def bind(path, digest, pins):
    path = Path(path).absolute()
    assert sha(path) == digest, ('Changed pinned input', str(path))
    assert str(path) not in pins or pins[str(path)] == digest
    pins[str(path)] = digest
    resolved = path.resolve(strict=True)
    if resolved != path:
        assert sha(resolved) == digest
        assert str(resolved) not in pins or pins[str(resolved)] == digest
        pins[str(resolved)] = digest
    return path


def verify(pins):
    for path, digest in pins.items():
        assert sha(path) == digest, ('Changed pinned input', path)


def read(path, digest, pins):
    return json.loads(bind(path, digest, pins).read_bytes())


def cold(path, name, pins):
    path = bind(path, pins[str(path)], pins)
    assert name not in sys.modules
    raw = path.read_bytes()
    assert hashlib.sha256(raw).hexdigest() == pins[str(path)]
    spec = importlib.util.spec_from_file_location(name, path)
    loaded = importlib.util.module_from_spec(spec)
    sys.modules[name] = loaded
    exec(compile(raw, str(path), 'exec'), loaded.__dict__)
    assert Path(loaded.__file__).resolve() == path and sys.modules[name] is loaded
    verify(pins)
    return loaded


def closed(owner, terminal_sha, pins, command=None):
    done = read(owner / 'terminal.json', terminal_sha, pins)
    assert done['status'] == 'command-exited-zero-awaiting-report-review'
    assert type(done['exit_code']) is int and done['exit_code'] == 0
    assert done['actual_command_started'] is True and done['stop_signals'] == []
    assert not done.get('error') and done['automatic_retry'] is done['qualification_granted'] is False
    request, launch, worker, started = [read(owner / name, sha(owner / name), pins) for name in
        ('request.json', 'launch.json', 'worker-started.json', 'command-started.json')]
    if command is None:
        command = request['command']
    assert all(item['command'] == command and item['cwd'] == str(H.parents[3])
               for item in (request, launch, worker, started))
    assert started['command_identity'] == done['command_identity']
    assert worker['worker_identity'] == done['worker_identity']
    assert worker['inherited_private_lease'] is worker['common_mutex_owned_by_target_command'] is True
    assert started['command_pid'] == done['command_pid'] == done['command_identity']['pid']
    assert started['command_identity_observation_missing'] is done['command_identity_observation_missing'] is False
    assert launch['worker_identity']['pid'] == worker['worker_identity']['pid']
    if owner == C / 'owned-jobs/pony-llvm23-reader-qualification-v1':
        initial, final = (item['worker_identity']['listing'].split(None, 8) for item in (launch, worker))
        assert len(initial) == len(final) == 9
        assert initial[0] == final[0] == '31638' and initial[2:7] == final[2:7]
        assert initial[7] == '/opt/homebrew/opt/python@3.14/bin/python3.14'
        assert final[7] == '/opt/homebrew/Cellar/python@3.14/3.14.8/Frameworks/Python.framework/Versions/3.14/Resources/Python.app/Contents/MacOS/Python'
        rest = '-B -u ' + str(C / 'launch_durable_v1.py') + ' --worker ' + str(owner) + ' --lease-fd 3 --go-fd 4'
        assert initial[8] == final[8] == rest and initial[1].isdigit() and final[1] == '1'
    else:
        assert launch['worker_identity']['listing'].split()[2:] == worker['worker_identity']['listing'].split()[2:]
    return request


def originals(args, pins):
    command = ['/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3.12',
        '-I', '-S', '-B', '-u', str(H / 'revisions/scored-allocation-recovery-v1/capture_existing_v1.py'),
        '--revision', 'original-six-traces-v1', '--source-sha256',
        'e299c68566068a96fffb0ca366efe411ba09f98dcf7d2b9316cb77946664cfe7',
        '--review-sha256', '64de34bc1b12238d0d00bb1b06202698fe3294b1a7f9f5bfc79dc14493e2866f',
        '--failed-report-sha256', 'b1336e55e14b5db6cf1b7bd6241c550da99d2e086ad07e2cb027c9a89bb1fd83',
        '--manifest-sha256', 'febd942e8c5de659b3dce114d0e7d5224598386059b207c72f82c97603aa37cb']
    closed(CAPTURE_OWNER, args.capture_owner_terminal_sha256, pins, command)
    outcome = read(CAPTURE / 'outcome.json', args.capture_outcome_sha256, pins)
    assert outcome['status'] == 'captured-original-evidence-proposals-awaiting-root-owner-review'
    assert outcome['common_mutex_held'] is True and not outcome.get('integrity_errors')
    assert all(type(outcome[name]) is int and outcome[name] == 0 for name in
               ('actual_native_commands', 'actual_benchmark_runs', 'actual_fixture_reexecutions'))
    for group in (outcome['pins'], outcome['available_evidence_pins']):
        for path, digest in group.items():
            bind(path, digest, pins)
    original = read(CAPTURE / 'ownership-proposals.json', PROPOSAL_SHA, pins)
    assert original['status'] == 'original-six-traces-and-partial-IR-retained-awaiting-independent-owner-review'
    assert len(original['original_trace_records']) == 6 and len(original['original_tree_observations']) == 2
    assert all(item['qualified'] is item['complete_output_match'] is True for item in original['original_trace_records'])
    assert original['surviving_main_header_is_builtin'] is True
    assert original['automatic_IR_selection_performed'] is False
    for path, digest in original['pins'].items():
        bind(path, digest, pins)
    q = read(Q, Q_SHA, pins)
    assert q['status'] == 'bounded-output-primitives-passed-awaiting-work-worker-and-LOC-review'
    for path, digest in q['pins'].items():
        bind(path, digest, pins)
    prep = cold(PREPARATION, 'pony_recovery_original_preparation', pins)
    service = cold(SERVICE, 'pony_recovery_owned_service', pins)
    guard = cold(prep.GUARD, 'pony_recovery_original_guard', pins)
    harness = guard.load_harness()
    for loaded in (harness, harness.compact, harness.compact.common):
        path = Path(loaded.__file__).resolve(strict=True)
        bind(path, pins[str(path)], pins)
    harness.install_interrupt_handlers()
    harness.compact.wait_for_environment(harness.REQUIRED)
    return original, q, prep, service, guard, harness


def environment(row=None):
    value = dict(os.environ, PATH='/usr/bin:/bin')
    if row is not None:
        value.update(row['runtime_environment'])
    for key in ('PONY_PATH', 'PONYPATH', 'PONYC_OPTIONS', 'CFLAGS', 'CPPFLAGS', 'LDFLAGS', 'LD_PRELOAD'):
        value.pop(key, None)
    for key in list(value):
        if key.startswith('DYLD_'):
            value.pop(key)
    if row is not None and 'DYLD_LIBRARY_PATH' in row['runtime_environment']:
        value['DYLD_LIBRARY_PATH'] = row['runtime_environment']['DYLD_LIBRARY_PATH']
    return value


def owned(prep, service, guard, harness, cmd, env, out, fd, timeout, name):
    result = service.owned_command(prep, guard, harness, cmd, env, out, fd, timeout, name)
    item = result['detail']
    assert item['command'] == cmd and item['inherited_common_mutex'] is True
    assert type(item['exit_status']) is type(item['final_exit_status']) is int
    assert item['exit_status'] == item['final_exit_status'] == 0
    assert item['leader_reaped'] is item['cleanup_complete'] is True
    assert item['cleanup_errors'] == item['remaining_group_members'] == [] and not item.get('error')
    return result


def reader(prep, service, guard, harness, out, fd, pins):
    bind(READER, READER_SHA, pins)
    env = environment()
    version = owned(prep, service, guard, harness, [str(READER), '--version'], env, out / 'version', fd, 30, 'version')
    found = re.findall(r'\bLLVM version (23\.\d+\.\d+)\b', version['stdout'].decode())
    assert len(found) == 1, 'Actual LLVM23 reader version required'
    pending, visited, contexts = [READER], set(), []
    while pending:
        image = pending.pop().resolve(strict=True)
        if image in visited:
            continue
        visited.add(image)
        directory = out / 'images' / f'{len(visited):03d}'
        image_pins = service.native_images(prep, guard, harness, image, env, directory, fd,
                                          [Path('/opt/homebrew/Cellar')])
        for path, digest in image_pins.items():
            bind(path, digest, pins)
        review = json.loads((directory / 'dependency-review.json').read_bytes())
        assert 'arm64' in review['architectures']
        for name in ('architecture', 'linked-images'):
            item = json.loads((directory / (name + '.json')).read_bytes())
            assert type(item['exit_status']) is type(item['final_exit_status']) is int
            assert item['exit_status'] == item['final_exit_status'] == 0
            assert item['cleanup_complete'] is item['leader_reaped'] is True
            assert item['cleanup_errors'] == item['remaining_group_members'] == []
        contexts.append(dict(image=str(image), image_sha256=sha(image), review=str(directory / 'dependency-review.json')))
        pending.extend(Path(item['path']) for item in review['dependencies'] if item.get('scope') != 'OS-image-not-hashed')
    for path, digest in prep.pin_tree(out).items():
        bind(path, digest, pins)
    receipt = dict(status='qualified-current-LLVM23-reader-and-transitive-link-images', reader=str(READER),
        reader_sha256=READER_SHA, version=found[0], version_receipt=str(out / 'version/version.json'),
        image_contexts=contexts, pins=dict(pins), original_compiler_backend_version='23.1.2',
        reader_is_original_compiler_image=False, application_executions=0, source_work_qualified=False)
    new(out / 'reader.json', receipt)
    return receipt


def recover(args, original, q, prep, service, guard, harness, output, fd, pins):
    receipt_path = args.reader_receipt.resolve(strict=True)
    assert receipt_path.name == 'reader.json' and receipt_path.parent.parent == ORIGINAL_READER_PACKET / 'jobs'
    assert args.reader_owner.resolve(strict=True) == C / 'owned-jobs/pony-llvm23-reader-qualification-v1'
    actual = read(receipt_path, args.reader_sha256, pins)
    assert actual['status'] == 'qualified-current-LLVM23-reader-and-transitive-link-images'
    assert actual['reader'] == str(READER) and actual['reader_sha256'] == READER_SHA and actual['version'].startswith('23.')
    end = read(receipt_path.parent / 'terminal.json', args.reader_terminal_sha256, pins)
    assert end['status'] == 'complete-reader-evidence-awaiting-root-review' and end['body_error'] is None and end['integrity_errors'] == []
    assert end['available_evidence_pins'][str(receipt_path)] == args.reader_sha256
    request = closed(args.reader_owner.resolve(strict=True), args.reader_owner_terminal_sha256, pins)
    cmd = request['command']
    assert cmd[:6] == ['/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3.12',
                     '-I', '-S', '-B', '-u', str(ORIGINAL_READER_PACKET / 'recover.py')]
    assert '--qualify-reader' in cmd and '--recover' not in cmd
    bind(ORIGINAL_READER_PACKET / 'recover.py', ORIGINAL_READER_WORKER_SHA, pins)
    bind(ORIGINAL_READER_PACKET / 'REVIEW.md', ORIGINAL_READER_REVIEW_SHA, pins)
    for option, expected in (('--revision', receipt_path.parent.name), ('--worker-sha256', ORIGINAL_READER_WORKER_SHA),
            ('--review-sha256', ORIGINAL_READER_REVIEW_SHA), ('--capture-outcome-sha256', args.capture_outcome_sha256),
            ('--capture-owner-terminal-sha256', args.capture_owner_terminal_sha256)):
        assert cmd.count(option) == 1 and cmd[cmd.index(option) + 1] == expected
    for group in (actual['pins'], end['pins'], end['available_evidence_pins']):
        for path, digest in group.items():
            bind(path, digest, pins)
    rows = {row['id']: row for row in q['profiles'] if row['benchmark'] == 'binarytrees'}
    assert set(rows) == set(IDS)
    profiles = {}
    for identifier in IDS:
        row = rows[identifier]
        source, binary = Path(row['source']), Path(row['command'][0])
        bind(source, row['source_sha256'], pins)
        bind(binary, row['binary_sha256'], pins)
        build = read(row['production_build_receipt'], q['pins'][row['production_build_receipt']], pins)
        command = list(build['command'])
        assert type(build['exit_status']) is type(build['final_exit_status']) is int
        assert build['exit_status'] == build['final_exit_status'] == 0
        assert not build.get('cleanup_error') and not build.get('error')
        assert command[0] == q['toolchain']['compiler'] and '--pass' not in command
        assert command[-1] == str(source.parent) and command[command.index('--bin-name') + 1] == identifier
        assert command[command.index('--output') + 1] == str(binary.parent)
        bind(command[0], q['toolchain']['compiler_sha256'], pins)
        assert set(source.parent.glob('*.pony')) == {source}, 'Unknown authored compiler input'
        library_roots = [Path(command[0]).parent.parent / 'packages'] + [Path(command[i + 1])
                         for i, part in enumerate(command) if part == '--path']
        for root in library_roots:
            root = root.resolve(strict=True)
            current = {str(path) for path in root.rglob('*.pony') if path.is_file()}
            expected = {path for path in q['pins']
                        if Path(path).is_relative_to(root) and Path(path).suffix == '.pony'}
            assert current == expected, ('Added or missing library/compiler input', str(root))
        directory = output / identifier
        bitcode = directory / 'bitcode'
        bitcode.mkdir(parents=True)
        command[command.index('--output') + 1] = str(bitcode)
        command[-1:-1] = ['--pass', 'bitcode']
        env = environment(row)
        env['SDKROOT'] = command[command.index('--sysroot') + 1]
        owned(prep, service, guard, harness, command, env, directory / 'compile', fd, 600, 'scored-bitcode')
        emitted = sorted(bitcode.iterdir())
        stderr = (directory / 'compile/scored-bitcode.stderr').read_text()
        assert stderr.splitlines().count('Building ' + str(source.parent) + ' -> ' + str(source.parent)) == 1
        writes = [line[8:] for line in stderr.splitlines() if line.startswith('Writing ')]
        assert emitted and len(writes) == len(set(writes)) == len(emitted)
        assert set(writes) == {str(path) for path in emitted}
        assert all(path.is_file() and not path.is_symlink() and path.suffix == '.bc' for path in emitted)
        main = bitcode / (identifier + '.main.bc')
        builtin = bitcode / (identifier + '.builtin.bc')
        assert main in emitted and builtin in emitted and main != builtin
        llvm = directory / 'authored-entry.ll'
        owned(prep, service, guard, harness, [str(READER), str(main), '-o', str(llvm)],
              environment(), directory / 'decode', fd, 120, 'entry-ir')
        raw = llvm.read_text()
        headers = raw.splitlines()[:5]
        assert 'target triple = "aarch64-apple-macosx14.4.0"' in headers
        assert 'source_filename = "' + str(main) + '"' in headers
        assert re.search(r'^define .*@Node_', raw, re.M)
        assert re.search(r'^define .*@Worker_', raw, re.M) and re.search(r'^define .*@Main_', raw, re.M)
        builtin_ir = directory / 'builtin.ll'
        owned(prep, service, guard, harness, [str(READER), str(builtin), '-o', str(builtin_ir)],
              environment(), directory / 'decode-builtin', fd, 120, 'builtin-ir')
        builtin_raw = builtin_ir.read_text()
        builtin_headers = builtin_raw.splitlines()[:5]
        assert 'source_filename = "' + str(builtin) + '"' in builtin_headers
        assert not re.search(r'^define .*@(Node|Worker|Main)_', builtin_raw, re.M)
        if identifier == IDS[0]:
            arm = Path(original['original_successful_scored_ARM_command']['command'][2])
            assert arm == binary
            arm = H / 'work-qualification/private-v8-work-v5/scored-tree-allocation' / identifier / 'arm/scored-arm.stdout'
            bind(arm, original['pins'][str(arm)], pins)
        else:
            owned(prep, service, guard, harness, ['/usr/bin/otool', '-tvV', str(binary)],
                  env, directory / 'arm', fd, 120, 'scored-arm')
            arm = directory / 'arm/scored-arm.stdout'
        assert arm.stat().st_size > 0
        profiles[identifier] = dict(source=str(source), source_sha256=row['source_sha256'],
            original_binary=str(binary), original_binary_sha256=row['binary_sha256'],
            production_build_receipt=row['production_build_receipt'], production_command=build['command'],
            diagnostic_command=command, emitted_bitcode_files=[str(path) for path in emitted],
            authored_entry_bitcode=str(main), separate_builtin_bitcode=str(builtin),
            authored_IR=str(llvm), module_headers=headers, separate_builtin_IR=str(builtin_ir),
            builtin_module_headers=builtin_headers, original_scored_ARM=str(arm),
            leaf_path_allocates_fresh_node=None, interior_path_allocates_fresh_node=None,
            no_static_or_shared_tree_graph=None, long_root_storage_retained_through_worker_joins=None)
        for path, digest in prep.pin_tree(directory).items():
            bind(path, digest, pins)
        verify(pins)
    return dict(status='missing-scored-IR-and-ARM-captured-awaiting-semantic-review', profiles=profiles,
        original_six_trace_records=original['original_trace_records'], reader_receipt=str(receipt_path),
        reader_sha256=args.reader_sha256, source_work_qualified=False, application_executions=0,
        fixture_reexecutions=0, full_performance_runs=0, diagnostic_executable_substituted=False,
        original_failures_preserved=True, final_LTO_ARM_review_required=True)


def main():
    parser = argparse.ArgumentParser()
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--qualify-reader', action='store_true')
    mode.add_argument('--recover', action='store_true')
    for name in ('revision', 'worker-sha256', 'review-sha256', 'capture-outcome-sha256', 'capture-owner-terminal-sha256'):
        parser.add_argument('--' + name, required=True)
    parser.add_argument('--reader-receipt', type=Path)
    parser.add_argument('--reader-sha256')
    parser.add_argument('--reader-terminal-sha256')
    parser.add_argument('--reader-owner', type=Path)
    parser.add_argument('--reader-owner-terminal-sha256')
    args = parser.parse_args()
    assert sys.dont_write_bytecode and re.fullmatch(r'[a-z0-9][a-z0-9-]{0,63}', args.revision)
    assert args.recover and not args.qualify_reader, 'This revision reuses the already qualified original reader'
    if args.recover:
        assert all((args.reader_receipt, args.reader_sha256, args.reader_terminal_sha256,
                    args.reader_owner, args.reader_owner_terminal_sha256))
    print('WAIT common mutex: Pony diagnostic evidence only', flush=True)
    with LOCK.open('r+') as common, ExitStack() as leases:
        fcntl.flock(common, fcntl.LOCK_EX)
        held, named = os.fstat(common.fileno()), LOCK.stat()
        assert (held.st_dev, held.st_ino) == (named.st_dev, named.st_ino) and held.st_ino == 79859134
        owners = [CAPTURE_OWNER, FAILED_OWNER] + ([args.reader_owner.resolve(strict=True)] if args.recover else [])
        for owner in owners:
            path = owner / 'owner.lock'
            lease = leases.enter_context(path.open('r+'))
            fcntl.flock(lease, fcntl.LOCK_EX | fcntl.LOCK_NB)
            held, named = os.fstat(lease.fileno()), path.stat()
            assert (held.st_dev, held.st_ino) == (named.st_dev, named.st_ino)
        output = P / 'jobs' / args.revision
        output.mkdir(parents=True, exist_ok=False)
        pins, result, failure = {}, {}, None
        try:
            for path, digest in ((P / 'recover.py', args.worker_sha256), (P / 'REVIEW.md', args.review_sha256)):
                assert not path.stat().st_mode & 0o222
                bind(path, digest, pins)
            assert all(not getattr(item, '__file__', None) or
                       not Path(item.__file__).resolve().is_relative_to(H.parents[1]) or
                       Path(item.__file__).resolve() == P / 'recover.py' for item in list(sys.modules.values()))
            original, q, prep, service, guard, harness = originals(args, pins)
            result = reader(prep, service, guard, harness, output, common.fileno(), pins) if args.qualify_reader else recover(
                args, original, q, prep, service, guard, harness, output, common.fileno(), pins)
            new(output / 'result.json', result)
        except BaseException as error:
            failure = dict(error=str(error) or type(error).__name__, error_type=type(error).__name__)
            (output / 'error.log').write_text(traceback.format_exc())
            raise
        finally:
            available, errors = {}, []
            candidates = dict(pins)
            candidates.update({str(path): None for path in output.rglob('*') if path.is_file() and str(path) not in candidates})
            for path, expected in candidates.items():
                try:
                    actual = sha(path)
                    available[path] = actual
                    assert expected is None or actual == expected
                except BaseException as error:
                    errors.append(dict(path=path, error=str(error) or type(error).__name__))
            status = 'failed-terminal-integrity' if errors else 'failed-diagnostic-evidence' if failure else (
                'complete-reader-evidence-awaiting-root-review' if args.qualify_reader else 'complete-scored-evidence-awaiting-semantic-review')
            new(output / 'terminal.json', dict(status=status, body_error=failure, integrity_errors=errors,
                pins=pins, available_evidence_pins=available, common_mutex_held=True, source_work_qualified=False,
                application_executions=0, fixture_reexecutions=0, full_performance_runs=0))
            if errors:
                raise RuntimeError('Diagnostic integrity failed; available raw bytes retained')


if __name__ == '__main__':
    main()
