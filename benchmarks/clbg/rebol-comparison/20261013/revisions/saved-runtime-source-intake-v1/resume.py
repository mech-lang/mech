"""Parse the five saved native successes; intake only the missing source archive."""
import argparse
from contextlib import ExitStack
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
PRIOR = P.with_name('retained-runtime-intake-v1')
PRIOR_SHA = 'd439a5e39d05741e0e5da71de8621e686acf75a9704c638e49c9ebb37de8c351'
PRIOR_NOTE_SHA = '3063b35c8e8e26fc1ca40e7f3d81fa931794d762c49e7288cd07111d8a7fa0c3'
OLD = H / 'runtime-qualification/retained-intake-v1'
OWNER = C / 'owned-jobs/rebol-retained-runtime-intake-v1'
REVIEW = C / 'jobs/rebol-retained-runtime-closed-review-v1/receipt.json'
REPORT_SHA = '6fd68b183d0469cb13dcfcbbbb7611bbd77eaaf8cbdbb07dca30db91b133709f'
BODY_SHA = '94b18e8f91650ff36763a7fcf317df01d378385f7cc750294e216ee5a5dc2e93'
DURABLE_SHA = '783652242552ea7691bd6d4f6c6e7efc3a48e3f9f02905400b87c951c9d84aec'
PYTHON = '/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3.12'
SEAL_BODY_SHA = '1bcf9599ff6b3c200f2fb03ec5fb6fe0d79b656ac605a2098efc35535462d9ec'
SEAL_OWNER_SHA = '1fe91e1fcc54710e4ae9002e57456f6b52adaa1f5e762ca0583ae4474ab9163f'


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def bind(path, expected, pins):
    path = Path(path).absolute()
    assert re.fullmatch('[a-f0-9]{64}', expected) and sha(path) == expected, str(path)
    assert str(path) not in pins or pins[str(path)] == expected
    pins[str(path)] = expected
    resolved = path.resolve(strict=True)
    if resolved != path:
        assert sha(resolved) == expected and (str(resolved) not in pins or pins[str(resolved)] == expected)
        pins[str(resolved)] = expected
    return path


def read(path, expected, pins):
    return json.loads(bind(path, expected, pins).read_bytes())


def cold(path, name, pins):
    assert name not in sys.modules
    raw = path.read_bytes()
    assert hashlib.sha256(raw).hexdigest() == pins[str(path)]
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    assert Path(module.__file__).resolve() == path.resolve()
    return module


def retained(args, pins):
    root = read(REVIEW, args.closed_review_sha256, pins)
    assert root['status'] == 'closed-native-successful-intake-stopped-at-boolean-text-parser' and root['verified_pins'] == 152
    assert root['report_sha256'] == REPORT_SHA and root['body_terminal_sha256'] == BODY_SHA and root['durable_terminal_sha256'] == DURABLE_SHA
    assert root['runtime_archive_downloads'] == 1 and root['source_archive_downloads'] == 0 and root['source_tree_present'] is False
    assert root['fixture_reexecutions'] == root['benchmark_runs'] == 0 and root['runtime_configuration_admitted'] is False
    assert root['actual_boolean_serialization'] == {'true':'#(true)', 'false':'#(false)'}
    assert root['regular_file_all256_bytes_verified'] is root['features_stderr_empty'] is True
    report = read(OLD / 'report.json', REPORT_SHA, pins)
    ended = read(OLD / 'terminal-pin.json', BODY_SHA, pins)
    assert report['status'] == ended['status'] == 'stopped-unqualified' and report['error_type'] == 'AssertionError'
    assert report['terminal_evidence_complete'] is True and not report.get('terminal_artifact_errors')
    assert ended['report'] == str(OLD / 'report.json') and ended['report_sha256'] == REPORT_SHA and ended['partial_evidence_pins'] == report['pins']
    for path, digest in report['pins'].items():
        bind(path, digest, pins)
    done = read(OWNER / 'terminal.json', DURABLE_SHA, pins)
    request, worker, started = [read(OWNER / name, sha(OWNER / name), pins) for name in
        ('request.json', 'worker-started.json', 'command-started.json')]
    command = [PYTHON, '-I', '-S', '-B', '-u', str(PRIOR / 'intake.py'), '--worker-sha256', PRIOR_SHA,
        '--review-sha256', PRIOR_NOTE_SHA, '--seal-terminal-sha256', SEAL_BODY_SHA,
        '--owner-terminal-sha256', SEAL_OWNER_SHA]
    assert all(item['command'] == command and item['cwd'] == str(H.parents[3]) for item in (request, worker, started))
    assert done['status'] == 'command-failed' and type(done['exit_code']) is int and done['exit_code'] == 1
    assert done['actual_command_started'] is True and done['stop_signals'] == [] and not done.get('error')
    assert done['automatic_retry'] is done['qualification_granted'] is request['automatic_retry'] is request['qualification_granted'] is False
    assert request['common_mutex_owned_by_target_command'] is worker['common_mutex_owned_by_target_command'] is True
    assert worker['inherited_private_lease'] is worker['detached_session'] is True
    assert done['worker_identity'] == worker['worker_identity'] and done['command_identity'] == started['command_identity']
    assert done['command_pid'] == started['command_pid'] == 43676 and worker['worker_identity']['pid'] == 43670
    assert done['command_identity_observation_missing'] is started['command_identity_observation_missing'] is False
    listing = started['command_identity']['listing'].split(None, 7)
    assert listing[:2] == ['43676', '43670'] and listing[7].split() == command
    bind(PRIOR / 'intake.py', PRIOR_SHA, pins); bind(PRIOR / 'REVIEW.md', PRIOR_NOTE_SHA, pins)
    prior = cold(PRIOR / 'intake.py', 'rebol_saved_original_intake', pins)
    base = prior.retain_seal(argparse.Namespace(seal_terminal_sha256=SEAL_BODY_SHA,
                                               owner_terminal_sha256=SEAL_OWNER_SHA), pins)
    contracts = sys.modules['rebol_retained_fixture_contracts']
    assert Path(contracts.__file__).resolve() == (base.PACKET / 'contracts.py').resolve()
    build = H / '.build/runtime-v1'
    assert report['private_root'] == str(build)
    runtime = build / 'runtime/rebol3'
    assert pins[str(runtime)] == pins[str(build / 'originals/rebol3')] == '27e255625d4d9928f5d726d3f2546fa64ff82a09a2b971cbc3a236a0bbed08a2'
    assert pins[str(build / 'archives/rebol3-bulk-macos-arm64.gz')] == contracts.RUNTIME_SHA
    download = read(OLD / 'publisher-runtime-download.json', pins[str(OLD / 'publisher-runtime-download.json')], pins)
    assert download['url'] == contracts.RUNTIME_URL and download['sha256'] == contracts.RUNTIME_SHA and download['actual_bytes'] == 1168762
    feature_command = [str(runtime), '-q', '-s', str(base.PACKET / 'runtime_probe.reb'), str(OLD / 'regular-file-all-bytes.bin')]
    commands = {
        OLD / 'images/architecture.json': ['/usr/bin/lipo', '-archs', str(runtime)],
        OLD / 'images/linked-images.json': ['/usr/bin/otool', '-L', str(runtime)],
        OLD / 'images/mach-header.json': ['/usr/bin/otool', '-hv', str(runtime)],
        OLD / 'queries/version.json': [str(runtime), '-v'],
        OLD / 'queries/features.json': feature_command,
    }
    actual_receipts = root['successful_native_commands']
    assert len(actual_receipts) == 5 and {item['path'] for item in actual_receipts} == {str(path) for path in commands}
    for item in actual_receipts:
        path = Path(item['path']); assert item['command'] == commands[path] and item['sha256'] == pins[str(path)]
        receipt = read(path, item['sha256'], pins)
        assert receipt['command'] == commands[path] and receipt['inherited_common_mutex'] is True
        assert type(receipt['exit_status']) is int and receipt['exit_status'] == receipt['final_exit_status'] == 0
        assert not any(receipt.get(key) for key in ('error', 'error_type', 'cleanup_error', 'cleanup_errors'))
        for suffix in ('.stdout', '.stderr'):
            raw = path.with_suffix(suffix); bind(raw, pins[str(raw)], pins)
        assert path.with_suffix('.stderr').read_bytes() == b''
    assert 'Rebol/Bulk 3.22.1' in (OLD / 'queries/version.stdout').read_text()
    images = read(OLD / 'os-image-review.json', pins[str(OLD / 'os-image-review.json')], pins)
    assert images['binary'] == str(runtime) and images['architectures'] == ['arm64']
    assert images == read(OLD / 'images/dependency-review.json', pins[str(OLD / 'images/dependency-review.json')], pins)
    for dependency in images['dependencies']:
        contracts.os_image(dependency['install_name'])
    assert not (build / 'source-original').exists() and not (build / 'archives'/('Rebol3-' + contracts.COMMIT + '.tar.gz')).exists()
    base.verify(pins)
    return base, contracts, runtime, feature_command, root


def parse_saved(base, raw):
    """Only the actual mold spelling is adapted; original key/type gates remain."""
    boolean_keys = {'function_is_interpreted_value', 'function_is_native_value', 'dynamic_rebinding', 'fresh_block_identity'}
    normalized = []
    for line in raw.decode('utf-8').splitlines():
        key, value = line.split('=', 1)
        if key in boolean_keys:
            assert value in {'#(true)', '#(false)'}, 'Unexpected actual Rebol boolean serialization'
            value = {'#(true)':'true', '#(false)':'false'}[value]
        normalized.append(key + '=' + value)
    return base.parse_probe(('\n'.join(normalized) + '\n').encode('utf-8'))


def main():
    parser = argparse.ArgumentParser()
    for name in ('worker', 'review', 'closed-review'):
        parser.add_argument('--' + name + '-sha256', required=True)
    args = parser.parse_args()
    assert sys.dont_write_bytecode and all(re.fullmatch('[a-f0-9]{64}', value) for value in vars(args).values())
    assert Path(sys.executable).resolve() == Path(PYTHON).resolve()
    def interrupted(signum, frame):
        raise KeyboardInterrupt('Source intake interrupted by signal ' + str(signum))
    for signum in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP, signal.SIGALRM):
        signal.signal(signum, interrupted)
    print('WAIT common mutex: saved Rebol features + missing source intake; zero native commands', flush=True)
    with LOCK.open('r+') as common, ExitStack() as leases:
        fcntl.flock(common, fcntl.LOCK_EX)
        opened, named = os.fstat(common.fileno()), LOCK.stat()
        assert (opened.st_dev, opened.st_ino) == (named.st_dev, named.st_ino) and opened.st_ino == 79859134
        for owner in (OWNER, C / 'owned-jobs/rebol-runtime6-fixture-seal-v1'):
            lease = leases.enter_context((owner / 'owner.lock').open('rb'))
            fcntl.flock(lease, fcntl.LOCK_EX | fcntl.LOCK_NB)
            opened, named = os.fstat(lease.fileno()), (owner / 'owner.lock').stat()
            assert (opened.st_dev, opened.st_ino) == (named.st_dev, named.st_ino)
        output = H / 'runtime-qualification/saved-runtime-source-intake-v1'
        build = H / '.build/saved-runtime-source-intake-v1'
        assert not output.exists() and not build.exists(), 'Fresh continuation only; preserve failed intake'
        for target in (output, build):
            assert target.resolve().is_relative_to(H.resolve())
            ancestor = target
            while ancestor != H:
                assert not ancestor.is_symlink()
                ancestor = ancestor.parent
        output.mkdir(parents=True); build.mkdir(parents=True)
        pins, base, failure = {}, None, None
        report = dict(status='saved-runtime-source-intake-started', inherited_common_mutex=True,
            private_root=str(build), native_commands_started=0, actual_fixture_methods_reexecuted=0,
            benchmark_processes_authorized=0, full_processes_authorized=0, chart_admission=False,
            runtime_configuration_admitted=False, source_work_qualified=False, header_generation_executed=False,
            extension_build_executed=False, runtime_downloads_started=0, source_downloads_started=0)
        signal.alarm(600)
        try:
            for path, digest in ((P / 'resume.py', args.worker_sha256), (P / 'REVIEW.md', args.review_sha256)):
                assert not path.stat().st_mode & 0o222
                bind(path, digest, pins)
            base, contracts, runtime, command, root = retained(args, pins)
            fields = parse_saved(base, (OLD / 'queries/features.stdout').read_bytes())
            observed = contracts.observations(fields, (OLD / 'regular-file-all-bytes.bin').read_bytes())
            assert [fields['automatic_gc_before'], fields['automatic_gc_after']] == root['automatic_gc_counts']
            before = read(OLD / 'startup-before.json', pins[str(OLD / 'startup-before.json')], pins)
            after = contracts.startup_inventory(runtime.parent, H / '.build/runtime-v1/private-data')
            assert before == after
            base.save(output / 'startup-after.json', after)
            base.save(output / 'observed-runtime-features.json', dict(fields=fields, observations=observed,
                command=command, original_stdout=dict(path=str(OLD / 'queries/features.stdout'), sha256=pins[str(OLD / 'queries/features.stdout')]),
                boolean_serialization='exact #(true)/#(false) mold values', runtime_binary=str(runtime),
                runtime_binary_sha256=pins[str(runtime)], native_commands_reexecuted=0,
                regular_byte_output=str(OLD / 'regular-file-all-bytes.bin'), regular_byte_output_sha256=pins[str(OLD / 'regular-file-all-bytes.bin')]))
            base.verify(pins)
            archive = build / ('Rebol3-' + contracts.COMMIT + '.tar.gz')
            report['source_downloads_started'] = 1
            record = base.download(contracts.SOURCE_URL, archive, 64 * 1024 * 1024)
            base.save(output / 'source-download.json', dict(record, source_commit=contracts.COMMIT,
                actual_sha_newly_captured=True, prior_publisher_sha_available=False))
            bind(archive, record['sha256'], pins)
            source = build / 'source-original'
            inventory = base.unpack_source(contracts, archive, source)
            for path in sorted(source.rglob('*')):
                if path.is_file(): bind(path, sha(path), pins)
            base.save(output / 'source-header-closure.json', dict(inventory, required_paths=contracts.SOURCE_REQUIRED,
                source_archive=str(archive), source_archive_sha256=record['sha256'],
                generated_headers_present=[str(path) for path in source.rglob('reb-lib.h')],
                source_archive_root_digest_review_required_before_generation=True))
            report.update(status='runtime-discovered-awaiting-source-header-and-primitive-review',
                runtime_binary=str(runtime), runtime_binary_sha256=pins[str(runtime)], observed_runtime_features=observed,
                successful_native_commands_reused=5, native_command_count=0, source_archive_sha256=record['sha256'],
                source_archive_newly_captured=True, source_tree_commit_independently_verified=False,
                source_header_generation_executed=False, selected_architecture='arm64', emulation_used=False,
                original_failed_report_sha256=REPORT_SHA, original_failed_body_terminal_sha256=BODY_SHA,
                original_failed_durable_terminal_sha256=DURABLE_SHA, prior_fixture_bytes_preserved=True)
        except BaseException as error:
            failure = error
            with (output / 'error.log').open('x') as stream: stream.write(traceback.format_exc())
            raise
        finally:
            signal.alarm(0)
            errors = []
            for path in sorted(build.rglob('*')):
                if path.is_file():
                    try: bind(path, pins.get(str(path), sha(path)), pins)
                    except BaseException as error: errors.append(dict(path=str(path), error=str(error)))
            if errors: report['private_root_pin_errors'] = errors
            if base is None:
                # The original terminal helper is loaded only from the literal
                # reviewed source after the same physical pin gate.
                path = P.with_name('runtime-v1') / 'bootstrap_runtime.py'
                bind(path, '7e2d3cf696273b8d96f9a31b752e7170da69ea0240b577de6c9dbe118f8ec117', pins)
                base = cold(path, 'rebol_saved_terminal_only', pins)
            base.terminal(output, report, pins, failure)


if __name__ == '__main__':
    main()
