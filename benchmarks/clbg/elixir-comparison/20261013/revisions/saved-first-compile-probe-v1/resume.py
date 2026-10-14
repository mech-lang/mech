"""Reuse successful first compilation; run its original unscored probe once."""
import argparse
from contextlib import ExitStack
from datetime import datetime, timezone
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import signal
import sys
import traceback
import types

P = Path(__file__).resolve().parent
HERE = P.parents[1]
CLBG = HERE.parents[1]
LOCK = Path('/tmp/mech-clbg-timing.lock')
PYTHON = '/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3.12'
INTAKE = HERE/'revisions/retained-otp-intake-v1/intake.py'
INTAKE_SHA = '51512225c30d507b1f19eef119b1402ef6455a9682d4b6cb1b80b90b5cb44cd9'
PRIOR = HERE/'revisions/compiler-stdio-owner-v1'
PRIOR_SHA = '56f8ddb9a721c3095d6739bc21dc3e27df0009787bba5af9d23ea071a31414e2'
PROBE_SHA = '67327eb3b7b36fb08b8a530cee9c0280bc413ffdae3bf51a2e32af12109b573b'
NOTE_SHA = 'd538030115364fde6dfa7c45e967005aedca408e8064b9a7cce7257700b323a8'
OLD_OUT = HERE/'jobs/first-compiler-stdio-owner-v1'
OLD_BUILD = HERE/'.build/first-compiler-stdio-owner-v1'
OWNER = CLBG/'continuation/owned-jobs/elixir-first-compiler-stdio-owner-v1'
ROOT = CLBG/'continuation/jobs/elixir-first-compile-warning-closed-review-v1/receipt.json'
ROOT_SHA = '34de9e98daefecee561002031de45ffdc051cd54521588b5758347979c779abb'
OUTCOME_SHA = '7ef7f235d4e84908b5f1e66656cdd3d4758723b0dbd60eba3510e9336c5ccdb3'
TERMINAL_SHA = '7de451627c527ab882eb3444bbf2258aca1994312de327b01a98b0f1e59b7fbd'
REQUEST_SHA = 'ecf26bd43ddb0d08a7af3f26b5bcda5e68b49c3a80fe648230cbeb5f4580120d'
COMPILE_SHA = '16331957ce01d78be080559e2f8f24e3db8b541b4b4c5082b6be496cd88f0243'
STDERR_SHA = 'ed2caac4185706609c30ae5d8d30d95258c0fa923f5d21ee51625ca61cd7be33'
STDOUT_SHA = 'f69bfbce7486f8b12f7a790372b30082df98caad90cc99d3aa5f44f9e10878e3'
COMPILER_SOURCE = Path('/opt/homebrew/Cellar/erlang/29.1.1_2/lib/erlang/lib/compiler-10.0.6/src/compile.erl')
COMPILER_SOURCE_SHA = 'c386a2b42181d7e1ef8964893e415fc723b374d697400d678bc15d70c4722541'
BEAMS = [('Elixir.Clbg.Main.beam', '038e5bff2af67b5f0a5d0ba80e67e4719c0112f420c89e8871b1fb4f88d957f7', 9968),
    ('Elixir.Clbg.Support.beam', 'eb007b1656cd8a33a02b7f7c42a892e01db06566f07741ac5b4db71961999f8f', 8008)]


def load_base(pins):
    raw = INTAKE.read_bytes()
    assert not INTAKE.stat().st_mode & 0o222 and hashlib.sha256(raw).hexdigest() == INTAKE_SHA
    name = 'elixir_saved_compile_original_helpers'; assert name not in sys.modules
    base = types.ModuleType(name); base.__file__ = str(INTAKE); sys.modules[name] = base
    exec(compile(raw, str(INTAKE), 'exec'), base.__dict__)
    base.bind(INTAKE, INTAKE_SHA, pins)
    return base


def saved_compile(base, pins):
    assert not (PRIOR/'prepare.py').stat().st_mode & 0o222
    previous = base.cold(PRIOR/'prepare.py', PRIOR_SHA, 'elixir_first_compile_pure_retained_helpers', pins)
    # Only this pure saved predecessor closure is reused, never prior run/main.
    compiler, saved = previous.retained(base, pins)
    base.bind(PRIOR/'probe.exs', PROBE_SHA, pins); base.bind(PRIOR/'REVIEW.md', NOTE_SHA, pins)
    assert not (PRIOR/'probe.exs').stat().st_mode & 0o222 and not (PRIOR/'REVIEW.md').stat().st_mode & 0o222
    root = base.read(ROOT, ROOT_SHA, pins)
    assert root['status'] == 'closed-successful-Elixir-compile-with-retained-warning-controller-failure' and root['verified_pins'] == 7776
    assert (root['worker_pid'], root['command_pid'], root['compiler_pid']) == (58755, 58761, 58763)
    assert (root['outcome_sha256'], root['durable_terminal_sha256'], root['request_sha256'], root['compiler_receipt_sha256'],
        root['compiler_stderr_sha256'], root['compiler_stdout_sha256']) == (OUTCOME_SHA, TERMINAL_SHA, REQUEST_SHA, COMPILE_SHA, STDERR_SHA, STDOUT_SHA)
    assert root['compiler_exit_status'] == 0 and root['controller_exit_status'] == 1
    assert root['compiler_warning'] == 'One unreachable defensive error arm after IO.binwrite in Clbg.Support.write/1 line33'
    assert root['compiler_stdout'] == 'Ignoring bad term in ERL_COMPILER_OPTIONS\n'
    assert root['source_owner_and_writer_probe_executed'] is root['chart_admission'] is False and root['common_mutex_held'] is True
    assert all(type(root[key]) is int and root[key] == 0 for key in ('subject_main_executions', 'benchmark_runs', 'fixture_reexecutions'))
    argv = [PYTHON, '-I', '-S', '-B', '-u', str(PRIOR/'prepare.py'), '--revision', 'first-compiler-stdio-owner-v1',
        '--worker-sha256', PRIOR_SHA, '--probe-sha256', PROBE_SHA, '--review-sha256', NOTE_SHA]
    ended = base.read(OWNER/'terminal.json', TERMINAL_SHA, pins)
    request = base.read(OWNER/'request.json', REQUEST_SHA, pins)
    started, worker = [base.read(OWNER/name, base.sha(OWNER/name), pins) for name in ('command-started.json', 'worker-started.json')]
    assert all(record['command'] == argv and record['cwd'] == str(CLBG.parent.parent) for record in (request, started, worker))
    assert ended['status'] == 'command-failed' and type(ended['exit_code']) is int and ended['exit_code'] == 1
    assert ended['actual_command_started'] is True and ended['stop_signals'] == []
    assert ended['qualification_granted'] is ended['automatic_retry'] is request['qualification_granted'] is request['automatic_retry'] is False
    assert request['common_mutex_owned_by_target_command'] is worker['common_mutex_owned_by_target_command'] is True
    assert worker['inherited_private_lease'] is worker['detached_session'] is True
    assert started['command_identity_observation_missing'] is ended['command_identity_observation_missing'] is False
    assert started['command_identity'] == ended['command_identity'] and started['command_pid'] == ended['command_pid'] == 58761
    assert worker['worker_identity'] == ended['worker_identity'] and ended['worker_identity']['pid'] == 58755
    listing = ended['command_identity']['listing'].split(None, 7)
    assert listing[:2] == ['58761', '58755'] and listing[7].split() == argv
    assert not any(ended.get(key) for key in ('error', 'error_type', 'exception_cleanup', 'error_log_write_failure'))
    outcome = base.read(OLD_OUT/'outcome.json', OUTCOME_SHA, pins)
    assert outcome['status'] == 'failed-Elixir-first-compiler-owner-writer' and outcome['error'] == outcome['error_type'] == 'AssertionError'
    assert outcome['exclusive_common_mutex_held'] is True and outcome['common_mutex_inode'] == 79859134
    assert outcome['original_toolchain_sources_and_ebins_preserved'] is True
    assert not any(outcome.get(key) for key in ('integrity_errors', 'available_bytes_not_fully_pinned'))
    for mapping in (outcome['prerequisite_pins'], outcome['available_evidence_pins']):
        for path, digest in mapping.items(): base.bind(path, digest, pins)
    assert not (OLD_OUT/'probe').exists() and not (OLD_OUT/'preparation-result.json').exists()
    assert (OLD_OUT/'owners').is_dir() and not any((OLD_OUT/'owners').iterdir())
    contract_path = OLD_OUT/'preparation-contract.json'
    assert str(contract_path) in outcome['available_evidence_pins']
    contract = base.read(contract_path, outcome['available_evidence_pins'][str(contract_path)], pins)
    assert contract['status'] == 'first-Elixir-subject-compile-and-unscored-owner-writer-observation'
    assert contract['selected_profile'] == 'elixir-nbody-compact' and contract['main_entry_executed'] is contract['chart_admission'] is False
    assert contract['whole20_inventory'] == saved['retained_source_inventory']
    folder = HERE/'.build/retained-otp-toolchain-v1/forms/elixir-nbody-compact'
    main, support, runner = folder/'main.ex', folder/'support.ex', folder/'runner.exs'
    expected_sources = [dict(path=str(path), sha256=digest, role=role, counted=True) for path, digest, role in
        ((main, previous.MAIN_SHA, 'application'), (support, previous.SUPPORT_SHA, 'application-helper'), (runner, previous.RUNNER_SHA, 'application-launcher'))]
    assert contract['complete_runtime_source_bundle'] == expected_sources
    assert contract['original_compiler_inputs'] == [expected_sources[1], expected_sources[0]]
    assert contract['unscored_probe'] == dict(path=str(PRIOR/'probe.exs'), sha256=PROBE_SHA, counted_in_scored_application=False)
    compile_argv = [str(compiler/'bin/elixirc'), '-o', str(OLD_BUILD/'ebin'), str(support), str(main)]
    compilation = base.read(OLD_OUT/'compile/first-nbody-compact-compile.json', COMPILE_SHA, pins)
    assert compilation['command'] == compile_argv and compilation['pid'] == compilation['identity']['pid'] == 58763
    assert compilation['exit_status'] == compilation['final_exit_status'] == 0 and compilation['timeout_seconds'] == 180
    assert compilation['inherited_common_mutex'] is compilation['preparation_only'] is compilation['never_chart_eligible'] is True
    assert not any(compilation.get(key) for key in ('error', 'error_type', 'cleanup_errors'))
    observed = compilation['identity']['process_listing'].split(None, 5)
    assert observed[5].split() == ['/bin/sh', *compile_argv]
    stdout = base.bind(OLD_OUT/'compile/first-nbody-compact-compile.stdout', STDOUT_SHA, pins).read_bytes()
    warning = base.bind(OLD_OUT/'compile/first-nbody-compact-compile.stderr', STDERR_SHA, pins).read_text()
    assert stdout == b'Ignoring bad term in ERL_COMPILER_OPTIONS\n'
    assert warning.count('warning:') == 1 and warning.startswith('    warning: the following clause will never match:\n')
    assert 'IO.binwrite(:stdio, data)' in warning and 'dynamic(not {:error, term()})' in warning
    assert warning.endswith(str(support)+':33: Clbg.Support.write/1\n\n')
    assert support.read_text().splitlines()[32] == '      {:error, reason} -> raise "Output error: #{inspect(reason)}"'
    expected_beams = [dict(path=str(OLD_BUILD/'ebin'/name), sha256=digest, bytes=size) for name, digest, size in BEAMS]
    assert root['beams'] == expected_beams and {path.name for path in (OLD_BUILD/'ebin').iterdir()} == {name for name, _, _ in BEAMS}
    for item in expected_beams:
        path = base.bind(item['path'], item['sha256'], pins); assert path.stat().st_size == item['bytes']
        with path.open('rb') as stream: header = stream.read(12)
        assert header[:4] == b'FOR1' and header[8:12] == b'BEAM' and int.from_bytes(header[4:8], 'big')+8 == item['bytes']
    assert outcome['prerequisite_pins'][str(COMPILER_SOURCE)] == COMPILER_SOURCE_SHA
    source = base.bind(COMPILER_SOURCE, COMPILER_SOURCE_SHA, pins).read_text()
    assert 'env_default_opts() ->\n    Key = "ERL_COMPILER_OPTIONS",\n    case os:getenv(Key) of\n\tfalse -> [];' in source
    assert 'io:format("Ignoring bad term in ~s\\n", [Key]),\n\t\t\t    []' in source
    return previous, compiler, saved, contract, expected_beams, main, support


def run(args, output, build, pins, common_fd, base):
    for name, digest in (('resume.py', args.worker_sha256), ('REVIEW.md', args.review_sha256)):
        assert not (P/name).stat().st_mode & 0o222; base.bind(P/name, digest, pins)
    previous, compiler, saved, contract, beams, main, support = saved_compile(base, pins)
    home, config, temporary, owners = build/'home', build/'config', build/'tmp', output/'owners'
    for path in (home, config, temporary, owners): path.mkdir(exist_ok=False)
    before = contract['runtime_environment']; environment = dict(before)
    assert environment['ERL_COMPILER_OPTIONS'] == ''; del environment['ERL_COMPILER_OPTIONS']
    environment.update(HOME=str(home), XDG_CONFIG_HOME=str(config), TMPDIR=str(temporary)+os.sep, ERL_CRASH_DUMP=str(output/'erl-crash.dump'))
    relocated = ('HOME', 'XDG_CONFIG_HOME', 'TMPDIR', 'ERL_CRASH_DUMP')
    assert {key: value for key, value in environment.items() if key not in relocated} == {key: value for key, value in before.items() if key not in (*relocated, 'ERL_COMPILER_OPTIONS')}
    assert environment['PATH'] == saved['runtime_environment']['PATH'] and environment['ELIXIR_ERL_OPTIONS'] == ' '.join(base.FLAGS)
    assert not any(home.iterdir()) and not any(config.iterdir())
    assert shutil.which('erl', path=environment['PATH']) == str(base.OTP/'bin/erl')
    service = base.cold(base.SERVICE, base.SERVICE_SHA, 'elixir_saved_compile_original_owned_service', pins)
    guard = service.load_guard(); harness = guard.load_harness()
    assert not list(service.HERE.glob('*.beam')) and not (service.HERE/'start.boot').exists() and not (service.HERE/'.erlang').exists()
    base.save(output/'resume-contract.json', dict(status='reuse-saved-compile-run-original-unscored-owner-writer-once',
        successful_compilation=dict(path=str(OLD_OUT/'compile/first-nbody-compact-compile.json'), sha256=COMPILE_SHA),
        retained_compiler_stdout=dict(path=str(OLD_OUT/'compile/first-nbody-compact-compile.stdout'), sha256=STDOUT_SHA),
        exact_reviewed_warning=dict(path=str(OLD_OUT/'compile/first-nbody-compact-compile.stderr'), sha256=STDERR_SHA),
        original_failed_controller=dict(path=str(OLD_OUT/'outcome.json'), sha256=OUTCOME_SHA),
        beams=beams, complete_runtime_source_bundle=contract['complete_runtime_source_bundle'],
        original_environment=before, probe_environment=environment,
        removed_empty_compiler_option=dict(key='ERL_COMPILER_OPTIONS', original_value='', replacement='absent',
            effective_default_option_list='[] in both cases', retained_source=dict(path=str(COMPILER_SOURCE), sha256=COMPILER_SOURCE_SHA)),
        original_probe=dict(path=str(PRIOR/'probe.exs'), sha256=PROBE_SHA), successful_compiler_reexecutions=0,
        subject_main_executions=0, chart_admission=False))
    base.verify(pins)
    probe = service.owned_command(guard, harness, [str(compiler/'bin/elixir'), '-pa', str(OLD_BUILD/'ebin'), str(PRIOR/'probe.exs'),
        str(owners), str(OLD_BUILD/'ebin'), str(main), str(support)], environment, output/'probe', common_fd, 90, 'compiled-source-owner-and-raw-writer')
    assert probe['stderr'] == b'' and probe['stdout'] == bytes(range(256))*513
    assert {path.name for path in owners.iterdir()} == {'Elixir.Clbg.Main.ownership.etf', 'Elixir.Clbg.Support.ownership.etf', 'summary.txt'}
    assert (owners/'summary.txt').read_text() == previous.EXPECTED_SUMMARY
    terms = []
    for path in sorted(owners.glob('*.etf')):
        assert path.is_file() and not path.is_symlink() and 0 < path.stat().st_size <= 16*1024*1024
        with path.open('rb') as stream: assert stream.read(1) == b'\x83'
        digest = base.sha(path); base.bind(path, digest, pins)
        terms.append(dict(path=str(path), sha256=digest, bytes=path.stat().st_size))
    base.bind(owners/'summary.txt', base.sha(owners/'summary.txt'), pins)
    for benchmark in base.BENCHES:
        for variant in ('compact', 'optimized'):
            old_ebin = HERE/'.build/retained-otp-toolchain-v1/forms'/('elixir-'+benchmark+'-'+variant)/'ebin'
            assert old_ebin.is_dir() and not any(old_ebin.iterdir())
    base.verify(pins)
    base.save(output/'preparation-result.json', dict(status='saved-Elixir-first-compile-owner-and-writer-observation-completed',
        retained_successful_compilation=dict(path=str(OLD_OUT/'compile/first-nbody-compact-compile.json'), sha256=COMPILE_SHA),
        whole20_inventory=saved['retained_source_inventory'], selected_profile='elixir-nbody-compact', beams=beams,
        original_owner_terms=terms, owner_writer_command=probe['detail'], compiled_source_debug_bindings=2,
        writer_exact_all256_bytes_multiple_chunks=True, writer_output_bytes=131328, input_EOF_observed=True,
        subject_compiler_commands=0, successful_compiler_reexecutions=0, owner_stdio_commands=1, subject_main_executions=0,
        nonempty_input_qualified=False, full_stdio_qualified=False, remaining19_profiles_compiled=False,
        regex_formatting_and_math_qualified=False, subject_execution_mode_qualified=False, source_work_qualified=False,
        original_ownership_fixtures_reused=11, fixture_reexecutions=0, actual_benchmark_runs=0, publisher_downloads=0,
        version_configuration_or_dry_reexecutions=0, original_sources_beams_and_old_ebins_preserved=True,
        runtime_environment=environment, canonical_loc=None, formatted_loc=None, chart_admission=False, pins=dict(pins)))


def main():
    parser = argparse.ArgumentParser()
    for name in ('revision', 'worker-sha256', 'review-sha256'): parser.add_argument('--'+name, required=True)
    args = parser.parse_args()
    assert args.revision == 'saved-first-compile-owner-writer-v1'
    assert sys.dont_write_bytecode and Path(sys.executable).resolve() == Path(PYTHON).resolve()
    assert all(re.fullmatch('[a-f0-9]{64}', value) for key, value in vars(args).items() if key.endswith('_sha256'))
    def interrupted(signum, frame): raise KeyboardInterrupt('Elixir saved compiler probe interrupted by signal '+str(signum))
    for signum in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP, signal.SIGALRM): signal.signal(signum, interrupted)
    with ExitStack() as leases:
        for path in (OWNER/'owner.lock', CLBG/'continuation/owned-jobs/elixir-saved-toolchain-probes-v1/owner.lock',
                     CLBG/'continuation/owned-jobs/elixir-retained-otp-toolchain-v1/owner.lock', CLBG/'continuation/owned-jobs/gleam-publication-metadata-v1/owner.lock'):
            lease = leases.enter_context(path.open('rb')); fcntl.flock(lease, fcntl.LOCK_SH|fcntl.LOCK_NB)
            actual, named = os.fstat(lease.fileno()), path.stat(); assert (actual.st_dev, actual.st_ino) == (named.st_dev, named.st_ino)
        own = leases.enter_context((HERE/'saved-first-compile-probe-v1.lock').open('a+')); fcntl.flock(own, fcntl.LOCK_EX|fcntl.LOCK_NB)
        print('WAIT common mutex: saved Elixir compile, one missing owner/writer probe', flush=True)
        with LOCK.open('r+') as common:
            fcntl.flock(common, fcntl.LOCK_EX)
            actual, named = os.fstat(common.fileno()), LOCK.stat()
            assert (actual.st_dev, actual.st_ino) == (named.st_dev, named.st_ino) and actual.st_ino == 79859134
            output, build = HERE/'jobs'/args.revision, HERE/'.build'/args.revision
            assert not output.exists() and not build.exists() and not any(path.is_symlink() for path in (output, build, output.parent, build.parent, HERE))
            output.mkdir(parents=True, exist_ok=False); build.mkdir(parents=True, exist_ok=False)
            pins, failures = {}, []
            outcome = dict(status='failed-Elixir-saved-first-compile-owner-writer', subject_compiler_commands=0,
                successful_compiler_reexecutions=0, subject_main_executions=0, fixture_reexecutions=0,
                actual_benchmark_runs=0, publisher_downloads=0, chart_admission=False)
            signal.alarm(300)
            try:
                base = load_base(pins)
                base.bind(Path(sys.executable).resolve(), 'ac60cfe0268614638d0ffa35f3b0284fc7b3a11482723793455e17eeb278509e', pins)
                run(args, output, build, pins, common.fileno(), base)
                outcome['status'] = 'saved-Elixir-first-compile-owner-and-writer-observation-completed'
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
                # Fresh private HOME/config/tmp, including cookies, are not evidence.
                for path in output.rglob('*'):
                    if path.is_file():
                        try: available[str(path)] = digest(path)
                        except BaseException as error: failures.append(dict(path=str(path), error=str(error) or type(error).__name__))
                if failures: outcome.update(status='failed-terminal-integrity', integrity_errors=failures)
                outcome.update(finished_at=datetime.now(timezone.utc).isoformat(), prerequisite_pins=pins, available_evidence_pins=available,
                    exclusive_common_mutex_held=True, common_mutex_inode=79859134, original_compile_warning_beams_and_sources_preserved=True,
                    private_startup_directories_not_evidence_artifacts=True)
                with (output/'outcome.json').open('x') as stream: json.dump(outcome, stream, indent=2); stream.write('\n')
                if failures: raise RuntimeError('Elixir saved compile/probe terminal integrity failed; partial evidence retained')


if __name__ == '__main__': main()
