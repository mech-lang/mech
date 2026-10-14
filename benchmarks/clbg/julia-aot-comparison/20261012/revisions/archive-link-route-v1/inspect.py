"""Read/extract already compiled archives; propose, never run, the next link."""
import argparse
from contextlib import ExitStack
from datetime import datetime, timezone
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import sys
import traceback
import types

P = Path(__file__).resolve().parent
HERE = P.parents[1]
CLBG = HERE.parents[1]
LOCK = Path('/tmp/mech-clbg-timing.lock')
PYTHON = '/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3.12'
SEAL = HERE/'revisions/saved-safe-build-seal-v1'
SEAL_SHA = '4eafdd9b6de63ff8d480e74431f247ab6e6adc5ba15654546c62474fb652fc82'
SEAL_NOTE_SHA = 'b0c9b76174573b1d6389d1c092d3fd221eeff84a87b4aa6c25feac3e8b41ecbf'
SAVED = HERE/'jobs/official-static-io-safe-build-seal-v1'
OWNER = CLBG/'continuation/owned-jobs/julia-saved-safe-build-seal-v1'
ROOT = CLBG/'continuation/jobs/julia-saved-safe-build-seal-closed-review-v1/receipt.json'
ROOT_SHA = '1ebe64084ffe2996ad4be7e0f5814d200bb61721a4a2bcae0cd3ab9ca9c19d84'
RESULT_SHA = '90b03c6657e45b824406e948fd6721d0cbb8fb1035037d09c372e4f868a84f27'
OPTIONS_SHA = '7ce0e355d1ac17c9d5f43cff91c58aa26529cbf52e778e345bc3f50451f7593c'
OUTCOME_SHA = '08484dd6e7ce8834a98c299b08f58003d689606ae740707c0cb4814d95ae139f'
TERMINAL_SHA = '7d910d43046271197d8620cb85d77b3a19a0866c284eea92ad9e82e9d5fdba17'
REQUEST_SHA = 'fd111d09cbaefe13d1afcccde75b0ca0271d41764b2c205e607f70e31f12def3'
TOOLS = {name: Path('/opt/homebrew/opt/llvm@22/bin')/executable for name, executable in
    (('ar', 'llvm-ar'), ('nm', 'llvm-nm'), ('readobj', 'llvm-readobj'))}


def cold(path, digest, name, pins):
    raw = path.read_bytes()
    assert not path.stat().st_mode & 0o222 and hashlib.sha256(raw).hexdigest() == digest
    assert name not in sys.modules
    module = types.ModuleType(name); module.__file__ = str(path); sys.modules[name] = module
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    pins[str(path.resolve())] = digest
    return module


def saved(pins):
    seal = cold(SEAL/'seal.py', SEAL_SHA, 'julia_archive_saved_seal_helpers', pins)
    base = seal.load_original(pins)
    def read(path, digest): return json.loads(base.bind(path, digest, pins).read_bytes())
    base.bind(SEAL/'REVIEW.md', SEAL_NOTE_SHA, pins)
    root = read(ROOT, ROOT_SHA)
    assert root['status'] == 'closed-qualified-Julia-static-IO-safe-build-options-and-ABI' and root['verified_pins'] == 9839
    assert (root['result_sha256'], root['parsed_options_sha256'], root['outcome_sha256'], root['durable_terminal_sha256'],
        root['request_sha256']) == (RESULT_SHA, OPTIONS_SHA, OUTCOME_SHA, TERMINAL_SHA, REQUEST_SHA)
    assert (root['worker_pid'], root['command_pid']) == (55243, 55249)
    assert root['original_successful_compile_reused'] is root['common_mutex_held'] is True
    assert root['native_compile_exit'] == 0 and root['actual_build_options_verified'] == 15
    assert all(type(root[key]) is int and root[key] == 0 for key in ('actual_native_processes', 'compile_reexecutions', 'fixture_reexecutions', 'benchmark_runs'))
    assert all(root[key] is False for key in ('runtime_aot_proven', 'native_owner_qualified', 'actual_ir_removal_qualified', 'fallback_closure_qualified', 'chart_admission'))
    argv = [PYTHON, '-I', '-S', '-B', '-u', str(SEAL/'seal.py'), '--revision', 'official-static-io-safe-build-seal-v1',
        '--worker-sha256', SEAL_SHA, '--review-sha256', SEAL_NOTE_SHA]
    terminal = read(OWNER/'terminal.json', TERMINAL_SHA)
    request = read(OWNER/'request.json', REQUEST_SHA)
    started, worker = [read(OWNER/name, base.sha(OWNER/name)) for name in ('command-started.json', 'worker-started.json')]
    assert all(record['command'] == argv and record['cwd'] == str(CLBG.parent.parent) for record in (request, started, worker))
    assert terminal['status'] == 'command-exited-zero-awaiting-report-review' and type(terminal['exit_code']) is int and terminal['exit_code'] == 0
    assert terminal['actual_command_started'] is True and terminal['stop_signals'] == []
    assert terminal['qualification_granted'] is terminal['automatic_retry'] is request['qualification_granted'] is request['automatic_retry'] is False
    assert request['common_mutex_owned_by_target_command'] is worker['common_mutex_owned_by_target_command'] is True
    assert worker['inherited_private_lease'] is worker['detached_session'] is True
    assert started['command_identity_observation_missing'] is terminal['command_identity_observation_missing'] is False
    assert started['command_identity'] == terminal['command_identity'] and started['command_pid'] == terminal['command_pid'] == 55249
    assert worker['worker_identity'] == terminal['worker_identity'] and terminal['worker_identity']['pid'] == 55243
    listing = terminal['command_identity']['listing'].split(None, 7)
    assert listing[:2] == ['55249', '55243'] and listing[7].split() == argv
    assert not any(terminal.get(key) for key in ('error', 'error_type', 'exception_cleanup', 'error_log_write_failure'))
    outcome = read(SAVED/'outcome.json', OUTCOME_SHA)
    result = read(SAVED/'safe-build-result.json', RESULT_SHA)
    parsed = read(SAVED/'actual-build-options.json', OPTIONS_SHA)
    assert outcome['status'] == result['status'] == 'safe-trim-build-complete-awaiting-native-image-and-fallback-review'
    assert outcome['exclusive_common_mutex_held'] is True and outcome['common_mutex_inode'] == 79859134
    assert not any(outcome.get(key) for key in ('error', 'error_type', 'integrity_errors', 'available_bytes_not_fully_pinned'))
    for mapping in (outcome['prerequisite_pins'], outcome['available_evidence_pins'], result['pins']):
        for path, digest in mapping.items(): base.bind(path, digest, pins)
    assert result['successful_native_compile_reexecutions'] == result['actual_native_processes'] == result['actual_workload_executions'] == result['actual_benchmark_runs'] == result['linker_executions'] == result['LLDB_attempts'] == result['fixture_reexecutions'] == 0
    assert result['safe_build_exit_status'] == 0 and result['verifier_stdout_empty'] is result['verifier_stderr_empty'] is True
    assert all(result[key] is False for key in ('native_owner_qualified', 'actual_ir_removal_qualified', 'fallback_closure_qualified', 'runtime_aot_proven', 'chart_admission'))
    assert parsed['format'] == 'parsed-exact-saved-Julia-Dict-display-v1' and parsed['evaluation_used'] is False
    assert parsed['options'] == result['actual_options'] and len(parsed['options']) == 15
    assert type(parsed['options']['trim']) is int and parsed['options']['trim'] == 1
    assert type(parsed['options']['strip_ir']) is int and parsed['options']['strip_ir'] == 1
    assert type(parsed['options']['incremental']) is int and parsed['options']['incremental'] == 0
    assert result['exported_entry']['all_exported_symbols'] == ['clbg_entry'] and result['exported_entry']['native_owner_qualified'] is False
    assert result['artifact_container_types'] == dict(object='ar-archive', bitcode='ar-archive-not-yet-member-qualified')
    assert result['artifacts'] == [dict(path=str(seal.PRIOR_BUILD/name), sha256=digest, bytes=size, header_hex=header) for name, digest, size, header in seal.ARTIFACTS]
    for item in result['artifacts']:
        path = base.bind(item['path'], item['sha256'], pins)
        with path.open('rb') as stream: assert stream.read(16).hex() == item['header_hex']
        assert path.stat().st_size == item['bytes']
    return base, seal, result


def names(raw, optimized):
    """Require the exact official producer's member naming family, not physical count."""
    values = raw.decode('ascii').splitlines()
    assert values and len(values) == len(set(values)) and 3 <= len(values) <= 66
    prefix, extension = ('_opt', '.bc') if optimized else ('', '.o')
    tail = ['metadata'+prefix+extension, 'sysimg'+prefix+extension]
    assert values[-2:] == tail
    expected = ['text'+prefix+'#'+str(index)+extension for index in range(len(values)-2)]
    assert values[:-2] == expected
    return values


def run(args, output, build, pins, common_fd):
    base, seal, result = saved(pins)
    for name, digest in (('inspect.py', args.worker_sha256), ('direct_driver.c', args.driver_sha256), ('REVIEW.md', args.review_sha256)):
        assert not (P/name).stat().st_mode & 0o222; base.bind(P/name, digest, pins)
    tools = {}
    for name, path in TOOLS.items():
        tools[name] = base.bind(path, getattr(args, name+'_sha256'), pins)
        assert tools[name].is_relative_to(Path('/opt/homebrew/Cellar/llvm@22')) and os.access(tools[name], os.X_OK)
    service = base.cold(base.PREPARATION, base.PREPARATION_SHA, 'julia_archive_existing_owned_service', pins)
    guard = service.load_guard(); harness = guard.load_harness()
    environment = dict(PATH='/usr/bin:/bin:/usr/sbin:/sbin', LC_ALL='C', LANG='C', OMP_NUM_THREADS='1',
        OPENBLAS_NUM_THREADS='1', VECLIB_MAXIMUM_THREADS='1')
    archives = [Path(item['path']) for item in result['artifacts'][:2]]
    lists, members = [], []
    for archive, optimized, label in zip(archives, (False, True), ('native-object', 'optimized-bitcode')):
        base.verify(pins)
        listed = service.owned_command(guard, harness, [str(tools['ar']), 't', str(archive)], environment,
            output/'archive-commands', common_fd, 60, label+'-members')
        assert listed['stderr'] == b''
        selected = names(listed['stdout'], optimized); lists.append(selected)
        destination = build/label; destination.mkdir(exist_ok=False)
        extracted = service.owned_command(guard, harness, [str(tools['ar']), '--output='+str(destination), 'x', str(archive)],
            environment, output/'archive-commands', common_fd, 60, label+'-extract')
        assert extracted['stderr'] == extracted['stdout'] == b''
        assert {path.name for path in destination.iterdir()} == set(selected)
        for name in selected:
            path = destination/name
            assert path.is_file() and not path.is_symlink() and 0 < path.stat().st_size <= archive.stat().st_size
            with path.open('rb') as stream: header = stream.read(32)
            if optimized:
                assert header.startswith(b'BC\xc0\xde') or header.startswith(b'\xde\xc0\x17\x0b'), 'Unknown actual bitcode member header'
                kind = 'LLVM-bitcode-container-not-decoded'
            else:
                assert header[:4] == b'\xcf\xfa\xed\xfe' and int.from_bytes(header[4:8], 'little') == 0x0100000c
                assert int.from_bytes(header[12:16], 'little') == 1, 'Expected arm64 Mach-O object, not executable/foreign image'
                kind = 'arm64-Mach-O-object'
            digest = base.sha(path); base.bind(path, digest, pins)
            members.append(dict(archive=str(archive), member=name, path=str(path), sha256=digest,
                bytes=path.stat().st_size, header32=header.hex(), container=kind))
    assert len(lists[0]) == len(lists[1]), 'Official object/optimized-bitcode partition families differ'
    base.verify(pins)
    symbols = service.owned_command(guard, harness, [str(tools['nm']), '--format=just-symbols', '--defined-only', '--extern-only',
        str(archives[0])], environment, output/'native-observation', common_fd, 60, 'native-defined-external-symbols')
    assert symbols['stderr'] == b''
    assert symbols['stdout'].decode().splitlines().count('_clbg_entry') == 1, 'Missing/ambiguous actual native entry'
    headers = service.owned_command(guard, harness, [str(tools['readobj']), '--file-header', '--symbols', str(archives[0])],
        environment, output/'native-observation', common_fd, 90, 'all-native-member-headers-and-symbols')
    assert headers['stderr'] == b'' and headers['stdout'], 'Complete raw native member observation required'
    # The installed test Makefile references an absent Make.inc. Produce the
    # concrete Darwin whole-archive/config equivalent; never execute it here.
    makefile = base.bind(base.TRIMMING/'Makefile', base.SHIPPED['Makefile'], pins)
    config = base.RUNTIME/'share/julia/julia-config.jl'
    assert pins[str(config.resolve())] == 'b167885d46710160fc758b912ebaad45a95c9d8ccb264629590127ba89f2dc09'
    base.bind(config, pins[str(config.resolve())], pins)
    cexample = base.TRIMMING/'capplication.c'
    assert pins[str(cexample.resolve())] == '75a32483188b4365bb7bf019ce85ae7b485f6a392fc540f030161791c6d553d2'
    base.bind(cexample, pins[str(cexample.resolve())], pins)
    assert 'LDFLAGS_ADD = -lm $(shell $(JULIA_CONFIG) --ldflags --ldlibs) -ljulia-internal' in makefile.read_text()
    assert not (base.RUNTIME/'share/julia/Make.inc').exists()
    clang = Path(service.CLANG).resolve(strict=True); assert str(clang) in pins
    lib, private = base.RUNTIME/'lib', base.RUNTIME/'lib/julia'
    binary = HERE/'.build/official-static-io-safe-link-v1/nbody-safe'
    link = [str(clang), '-std=gnu11', '-fPIC', '-O3', '-g', '-I'+str(base.RUNTIME/'include/julia'),
        str(P/'direct_driver.c'), '-o', str(binary), '-Xlinker', '-force_load', str(archives[0]), '-lm',
        '-L'+str(lib), '-L'+str(private), '-Wl,-rpath,'+str(private), '-Wl,-rpath,'+str(lib), '-ljulia', '-ljulia-internal']
    base.verify(pins)
    base.save(output/'archive-result.json', dict(status='actual-archive-members-and-native-entry-observed-awaiting-root-owner-and-link-review',
        saved_safe_build=dict(path=str(SAVED/'safe-build-result.json'), sha256=RESULT_SHA), members=members,
        object_members=lists[0], optimized_bitcode_members=lists[1], actual_defined_native_entry_count=1,
        native_member_symbols=str(output/'native-observation/all-native-member-headers-and-symbols.stdout'),
        object_and_bitcode_members_captured=True, bitcode_decoded=False, actual_ir_removal_qualified=False,
        native_owner_qualified=False, fallback_closure_qualified=False, runtime_aot_proven=False,
        planned_next_link=dict(command=link, actual_execution=False, source_basis=dict(test_makefile=str(makefile),
            config_source=str(config), direct_C_library_example=str(cexample)), counted_fresh_glue=dict(path=str(P/'direct_driver.c'), sha256=args.driver_sha256),
            PackageCompiler_initialization_api_used=False, root_actual_archive_review_required=True,
            next_image_observations=[['/usr/bin/lipo', '-archs', str(binary)], ['/usr/bin/otool', '-L', str(binary)],
                ['/usr/bin/otool', '-l', str(binary)], [str(tools['nm']), '--format=just-symbols', '--defined-only', '--extern-only', str(binary)]]),
        actual_native_inspection_commands=6, actual_julia_processes=0, successful_compile_reexecutions=0,
        linker_executions=0, actual_workload_executions=0, fixture_reexecutions=0, benchmark_runs=0,
        LLDB_attempts=0, chart_admission=False, pins=dict(pins)))


def main():
    parser = argparse.ArgumentParser()
    for name in ('revision', 'worker-sha256', 'driver-sha256', 'review-sha256', 'ar-sha256', 'nm-sha256', 'readobj-sha256'):
        parser.add_argument('--'+name, required=True)
    args = parser.parse_args()
    assert args.revision == 'official-static-io-safe-archives-v1'
    assert sys.dont_write_bytecode and Path(sys.executable).resolve() == Path(PYTHON).resolve()
    assert all(re.fullmatch('[a-f0-9]{64}', value) for key, value in vars(args).items() if key.endswith('_sha256'))
    def interrupted(signum, frame): raise KeyboardInterrupt('Julia archive observation interrupted by signal '+str(signum))
    for signum in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP, signal.SIGALRM): signal.signal(signum, interrupted)
    with ExitStack() as leases:
        for path in (OWNER/'owner.lock', CLBG/'continuation/owned-jobs/julia-official-static-io-safe-trim-nbody-v1/owner.lock'):
            lease = leases.enter_context(path.open('rb')); fcntl.flock(lease, fcntl.LOCK_SH|fcntl.LOCK_NB)
            actual, named = os.fstat(lease.fileno()), path.stat(); assert (actual.st_dev, actual.st_ino) == (named.st_dev, named.st_ino)
        own = leases.enter_context((HERE/'official-safe-archive-observation-v1.lock').open('a+'))
        fcntl.flock(own, fcntl.LOCK_EX|fcntl.LOCK_NB)
        print('WAIT common mutex: existing Julia archives only, no Julia compile/link/runtime', flush=True)
        with LOCK.open('r+') as common:
            fcntl.flock(common, fcntl.LOCK_EX)
            actual, named = os.fstat(common.fileno()), LOCK.stat()
            assert (actual.st_dev, actual.st_ino) == (named.st_dev, named.st_ino) and actual.st_ino == 79859134
            output, build = HERE/'jobs'/args.revision, HERE/'.build'/args.revision
            assert not output.exists() and not build.exists() and not any(path.is_symlink() for path in (output, build, output.parent, build.parent, HERE))
            output.mkdir(parents=True, exist_ok=False); build.mkdir(parents=True, exist_ok=False)
            pins, failures = {}, []
            outcome = dict(status='failed-Julia-archive-observation', actual_julia_processes=0, successful_compile_reexecutions=0,
                linker_executions=0, actual_workload_executions=0, fixture_reexecutions=0, benchmark_runs=0, LLDB_attempts=0,
                runtime_aot_proven=False, chart_admission=False)
            signal.alarm(600)
            try:
                pins[str(Path(sys.executable).resolve())] = 'ac60cfe0268614638d0ffa35f3b0284fc7b3a11482723793455e17eeb278509e'
                run(args, output, build, pins, common.fileno())
                outcome['status'] = 'actual-archive-members-and-native-entry-observed-awaiting-root-owner-and-link-review'
            except BaseException as error:
                outcome.update(error=str(error) or type(error).__name__, error_type=type(error).__name__)
                with (output/'error.log').open('x') as stream: stream.write(traceback.format_exc())
                raise
            finally:
                signal.alarm(0); available = {}
                def digest(path):
                    with Path(path).open('rb') as stream: return hashlib.file_digest(stream, 'sha256').hexdigest()
                for path, expected in pins.items():
                    try: assert digest(path) == expected
                    except BaseException as error: failures.append(dict(path=path, error=str(error) or type(error).__name__))
                for tree in (output, build):
                    for path in tree.rglob('*'):
                        if path.is_file():
                            try: available[str(path)] = digest(path)
                            except BaseException as error: failures.append(dict(path=str(path), error=str(error) or type(error).__name__))
                if failures: outcome.update(status='failed-terminal-integrity', integrity_errors=failures)
                outcome.update(finished_at=datetime.now(timezone.utc).isoformat(), prerequisite_pins=pins, available_evidence_pins=available,
                    exclusive_common_mutex_held=True, common_mutex_inode=79859134, original_safe_compile_and_seal_preserved=True)
                with (output/'outcome.json').open('x') as stream: json.dump(outcome, stream, indent=2); stream.write('\n')
                if failures: raise RuntimeError('Julia archive terminal pin integrity failed; partial evidence retained')


if __name__ == '__main__': main()
