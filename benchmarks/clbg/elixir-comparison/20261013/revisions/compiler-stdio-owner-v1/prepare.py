"""One first subject compile and unscored owner/writer observation only."""
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

P = Path(__file__).resolve().parent
HERE = P.parents[1]
CLBG = HERE.parents[1]
LOCK = Path('/tmp/mech-clbg-timing.lock')
PYTHON = '/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3.12'
OLD = HERE/'revisions/retained-otp-intake-v1'
OLD_SHA = '51512225c30d507b1f19eef119b1402ef6455a9682d4b6cb1b80b90b5cb44cd9'
RESUME = HERE/'revisions/saved-otp-probes-v1'
RESUME_SHA = '7bc94b050805369e8fc74212476eaaaa588e6b404d4f55e9f996dc146522b275'
RESUME_NOTE_SHA = '83579f701a10dcd6b6b64d46057a9749159575ce89181a657129d7a17ff3a569'
SAVED = HERE/'jobs/retained-otp-probes-v1'
OWNER = CLBG/'continuation/owned-jobs/elixir-saved-toolchain-probes-v1'
ROOT = CLBG/'continuation/jobs/elixir-saved-toolchain-probes-closed-review-v1/receipt.json'
ROOT_SHA = 'b18eb9dda83694122a6bb320990123a56e913c927c9b7e404f068bf0611e1450'
RESULT_SHA = 'ad8aca223866b085ba3e823538298b948bd427748909bff1913d874d608519d0'
STARTUP_SHA = '6be60a0560bbc99a902d0a489e06d82fdbd520451cc2d114b20648067d0cf72e'
OUTCOME_SHA = '82e224a49fafb9b798d061341594ef61bc9e8a26908e16005d7c2cea62f27c8d'
TERMINAL_SHA = 'b5b06a47a8e7d2692771953e9d54c1d5a7c403cca342617e672011d8230c6562'
REQUEST_SHA = '30979cec7fca773da90249085e4714442dc14be852f5e669e7a548f2a158e9e3'
MAIN_SHA = '253d24ca9ce83817df37811d3101db561070d5f39b360b540fa8730d3b3a622c'
SUPPORT_SHA = 'c9fbb60888c46fb50195245761a411716f1256145af367e03a23f70aea270e89'
RUNNER_SHA = 'bdad2389d437fd4c69122e5d25b3065412b4bca0c272631667283273392ba8ff'
EXPECTED_SUMMARY = ('compiled_modules=2\nsource_debug_bindings=2\ninput_eof=true\n'
    'raw_output_bytes=131328\nmain_entry_executed=false\nnonempty_input_qualified=false\n')


def retained(base, pins):
    """Only saved_intake/read/bind helpers are called; never old run/main."""
    resume = base.cold(RESUME/'resume.py', RESUME_SHA, 'elixir_actual_saved_intake_v1', pins)
    assert not (RESUME/'resume.py').stat().st_mode & 0o222
    assert not (RESUME/'REVIEW.md').stat().st_mode & 0o222
    base.bind(RESUME/'REVIEW.md', RESUME_NOTE_SHA, pins)
    compiler, literal = resume.saved_intake(base, pins)
    root = base.read(ROOT, ROOT_SHA, pins)
    assert root['status'] == 'closed-successful-Elixir-saved-intake-version-and-runtime-probes' and root['verified_pins'] == 7765
    assert (root['result_sha256'], root['startup_contract_sha256'], root['outcome_sha256'],
        root['durable_terminal_sha256'], root['request_sha256']) == (RESULT_SHA, STARTUP_SHA, OUTCOME_SHA, TERMINAL_SHA, REQUEST_SHA)
    assert (root['worker_pid'], root['command_pid']) == (52439, 52452)
    assert root['materialized_forms_reused'] == 20 and root['original_ownership_fixtures_reused'] == 11
    assert all(type(root[key]) is int and root[key] == 0 for key in
        ('publisher_downloads', 'dry_command_reexecutions', 'fixture_reexecutions', 'subject_compiler_commands', 'benchmark_runs'))
    assert root['raw_stdio_qualified'] is root['source_work_qualified'] is root['chart_admission'] is False
    assert root['canonical_loc'] is None and root['common_mutex_held'] is True
    command = [PYTHON, '-I', '-S', '-B', '-u', str(RESUME/'resume.py'), '--revision', 'retained-otp-probes-v1',
        '--worker-sha256', RESUME_SHA, '--review-sha256', RESUME_NOTE_SHA]
    ended = base.read(OWNER/'terminal.json', TERMINAL_SHA, pins)
    request = base.read(OWNER/'request.json', REQUEST_SHA, pins)
    started, worker = [base.read(OWNER/name, base.sha(OWNER/name), pins) for name in ('command-started.json', 'worker-started.json')]
    assert all(record['command'] == command and record['cwd'] == str(CLBG.parent.parent) for record in (request, started, worker))
    assert ended['status'] == 'command-exited-zero-awaiting-report-review' and type(ended['exit_code']) is int and ended['exit_code'] == 0
    assert ended['actual_command_started'] is True and ended['stop_signals'] == []
    assert ended['command_identity_observation_missing'] is started['command_identity_observation_missing'] is False
    assert ended['qualification_granted'] is ended['automatic_retry'] is request['qualification_granted'] is request['automatic_retry'] is False
    assert request['common_mutex_owned_by_target_command'] is worker['common_mutex_owned_by_target_command'] is True
    assert worker['inherited_private_lease'] is worker['detached_session'] is True
    assert worker['worker_identity'] == ended['worker_identity'] and ended['worker_identity']['pid'] == 52439
    assert started['command_identity'] == ended['command_identity'] and started['command_pid'] == ended['command_pid'] == 52452
    observed = ended['command_identity']['listing'].split(None, 7)
    assert observed[:2] == ['52452', '52439'] and observed[7].split() == command
    assert not any(ended.get(key) for key in ('error', 'error_type', 'exception_cleanup', 'error_log_write_failure'))
    outcome = base.read(SAVED/'outcome.json', OUTCOME_SHA, pins)
    result = base.read(SAVED/'intake-result.json', RESULT_SHA, pins)
    assert outcome['status'] == result['status'] == 'saved-publisher-toolchain-and-exact20-inputs-ready-awaiting-compiler-stdio-owner-review'
    assert outcome['exclusive_common_mutex_held'] is True and outcome['common_mutex_inode'] == 79859134
    assert not any(outcome.get(key) for key in ('error', 'error_type', 'integrity_errors', 'available_bytes_not_fully_pinned'))
    for mapping in (outcome['prerequisite_pins'], outcome['available_evidence_pins'], result['pins']):
        for path, digest in mapping.items(): base.bind(path, digest, pins)
    base.read(SAVED/'startup-contract.json', STARTUP_SHA, pins)
    assert result['compiler'] == str(compiler/'bin/elixirc') and result['runtime_erl'] == str(base.OTP/'bin/erl')
    assert result['saved_literal_dry_argv'] == literal
    assert result['retained_source_inventory'] == dict(path=str(resume.OLD_OUT/'source-input-inventory.json'), sha256=resume.INVENTORY_SHA)
    assert result['materialized_forms_reused'] == 20 and result['original_ownership_fixtures_reused'] == 11
    assert result['beam_version_commands'] == result['configuration_probes'] == 1
    assert all(type(result[key]) is int and result[key] == 0 for key in
        ('publisher_downloads', 'archive_extractions', 'source_rematerializations', 'dry_command_reexecutions',
         'fixture_reexecutions', 'subject_compiler_commands', 'actual_benchmark_runs'))
    assert all(result[key] is False for key in ('raw_stdio_qualified', 'subject_execution_mode_qualified',
        'collector_policy_qualified', 'source_work_qualified', 'chart_admission'))
    assert result['canonical_loc'] is result['formatted_loc'] is None
    required = dict(ELIXIR='1.20.4', OTP='29', ERTS='17.1', ARCH='aarch64-apple-darwin25.6.0', EMU_FLAVOR='jit',
        SCHEDULERS='8', SCHEDULERS_ONLINE='8', DIRTY_CPU='1', DIRTY_IO='1', ASYNC_THREADS='1')
    assert root['toolchain'] == required and all(result['configuration_probe'][key] == value for key, value in required.items())
    assert result['configuration_probe']['CLBG_MAIN_LOADED'] == result['configuration_probe']['CLBG_SUPPORT_LOADED'] == 'false'
    assert result['actual_gc_defaults_text'] == '[max_heap_size: 0, min_bin_vheap_size: 46422, min_heap_size: 233, fullsweep_after: 65535]'
    expected_receipts = [(SAVED/'version/compiler-runtime-version.json', '551f6b4d317a8834f9c2324edc2dd3f324ccf2cce3d2b925a176d66e6d266723', [str(compiler/'bin/elixir'), '--version']),
        (SAVED/'probe/runtime-configuration.json', '40cf1b73ea8a113f7cf07e3b6b792e1c2e3568837ddbfbd768b8dd417b8871dd', [str(compiler/'bin/elixir'), str(OLD/'probe.exs')])]
    assert root['native_commands'] == [dict(path=str(path), sha256=digest, command=argv) for path, digest, argv in expected_receipts]
    for path, digest, argv in expected_receipts:
        receipt = base.read(path, digest, pins)
        assert receipt['command'] == argv and receipt['exit_status'] == receipt['final_exit_status'] == 0
        assert receipt['inherited_common_mutex'] is receipt['preparation_only'] is receipt['never_chart_eligible'] is True
        assert not receipt.get('cleanup_errors') and not receipt.get('error')
    return compiler, result


def run(args, output, build, pins, common_fd, base):
    for name, digest in (('prepare.py', args.worker_sha256), ('probe.exs', args.probe_sha256), ('REVIEW.md', args.review_sha256)):
        assert not (P/name).stat().st_mode & 0o222
        base.bind(P/name, digest, pins)
    compiler, saved = retained(base, pins)
    runtime = base.retained_runtime(argparse.Namespace(runtime_review_sha256='406ac95e0f93b99efc26d7bb62d0fe593568860c8d6cc3b8bfa39918ca1f9cdc'), pins)
    assert runtime['actual_toolchain']['arch'] == 'aarch64-apple-darwin25.6.0'
    folder = HERE/'.build/retained-otp-toolchain-v1/forms/elixir-nbody-compact'
    main = base.bind(folder/'main.ex', MAIN_SHA, pins)
    support = base.bind(folder/'support.ex', SUPPORT_SHA, pins)
    runner = base.bind(folder/'runner.exs', RUNNER_SHA, pins)
    assert not any((folder/'ebin').iterdir())
    assert all('@on_load' not in path.read_text() for path in (main, support))
    ebin, owners = build/'ebin', output/'owners'
    home, config, temporary = build/'home', build/'config', build/'tmp'
    for path in (ebin, owners, home, config, temporary): path.mkdir(exist_ok=False)
    environment = dict(saved['runtime_environment'])
    environment.update(HOME=str(home), XDG_CONFIG_HOME=str(config), TMPDIR=str(temporary)+os.sep, ERL_CRASH_DUMP=str(output/'erl-crash.dump'))
    assert not any(home.iterdir()) and not any(config.iterdir()) and not any(ebin.iterdir())
    assert environment['PATH'] == str(base.OTP/'bin')+':'+str(compiler/'bin')+':/usr/bin:/bin:/usr/sbin:/sbin'
    assert environment['ELIXIR_ERL_OPTIONS'] == ' '.join(base.FLAGS)
    assert all(environment[key] == '' for key in ('ERL_FLAGS', 'ERL_AFLAGS', 'ERL_ZFLAGS', 'ERL_LIBS', 'ERL_COMPILER_OPTIONS'))
    assert shutil.which('erl', path=environment['PATH']) == str(base.OTP/'bin/erl')
    service = base.cold(base.SERVICE, base.SERVICE_SHA, 'elixir_first_compiler_original_owned_service', pins)
    guard = service.load_guard(); harness = guard.load_harness()
    assert not list(service.HERE.glob('*.beam')) and not (service.HERE/'start.boot').exists() and not (service.HERE/'.erlang').exists()
    fixture = base.read(base.FIXTURES, base.FIXTURES_SHA, pins)
    assert fixture['status'] == 'passed' and fixture['successful'] is True and fixture['inventory'] == 'ownership'
    assert fixture['tests_run'] == fixture['discovered_tests'] == 11
    assert set(fixture['expected_test_ids']) == set(fixture['actual_test_ids']) and len(set(fixture['actual_test_ids'])) == 11
    assert all(fixture[key] == [] for key in ('skipped', 'failures', 'errors', 'expected_failures', 'unexpected_successes'))
    fixture_command = base.read(base.FIXTURE_COMMAND, base.FIXTURE_COMMAND_SHA, pins)
    assert fixture_command['exit_status'] == fixture_command['final_exit_status'] == 0
    assert fixture_command['inherited_common_mutex'] is True and not fixture_command.get('cleanup_errors')
    base.save(output/'preparation-contract.json', dict(status='first-Elixir-subject-compile-and-unscored-owner-writer-observation',
        retained_toolchain=dict(path=str(SAVED/'intake-result.json'), sha256=RESULT_SHA),
        whole20_inventory=saved['retained_source_inventory'], selected_profile='elixir-nbody-compact',
        original_compiler_inputs=[dict(path=str(support), sha256=SUPPORT_SHA, role='application-helper', counted=True),
            dict(path=str(main), sha256=MAIN_SHA, role='application', counted=True)],
        complete_runtime_source_bundle=[dict(path=str(main), sha256=MAIN_SHA, role='application', counted=True),
            dict(path=str(support), sha256=SUPPORT_SHA, role='application-helper', counted=True),
            dict(path=str(runner), sha256=RUNNER_SHA, role='application-launcher', counted=True)],
        unscored_probe=dict(path=str(P/'probe.exs'), sha256=args.probe_sha256, counted_in_scored_application=False),
        runtime_environment=environment, original_sources_preserved=True, old_ebin_unchanged=True,
        stdin='existing-owned-service-DEVNULL', main_entry_executed=False, chart_admission=False))
    base.verify(pins)
    compilation = service.owned_command(guard, harness, [str(compiler/'bin/elixirc'), '-o', str(ebin), str(support), str(main)],
        environment, output/'compile', common_fd, 180, 'first-nbody-compact-compile')
    assert compilation['stderr'] == b''
    expected = {'Elixir.Clbg.Main.beam', 'Elixir.Clbg.Support.beam'}
    assert {path.name for path in ebin.iterdir()} == expected
    beams = []
    for path in sorted(ebin.iterdir()):
        assert path.is_file() and not path.is_symlink() and 0 < path.stat().st_size <= 16*1024*1024
        raw = path.read_bytes()
        assert len(raw) >= 12 and raw[:4] == b'FOR1' and raw[8:12] == b'BEAM' and int.from_bytes(raw[4:8], 'big')+8 == len(raw)
        beams.append(dict(path=str(path), sha256=base.sha(path), bytes=len(raw), header16=raw[:16].hex()))
        base.bind(path, beams[-1]['sha256'], pins)
    assert shutil.which('erl', path=environment['PATH']) == str(base.OTP/'bin/erl'); base.verify(pins)
    probe = service.owned_command(guard, harness, [str(compiler/'bin/elixir'), '-pa', str(ebin), str(P/'probe.exs'),
        str(owners), str(ebin), str(main), str(support)], environment, output/'probe', common_fd, 90, 'compiled-source-owner-and-raw-writer')
    assert probe['stderr'] == b'' and probe['stdout'] == bytes(range(256))*513
    assert {path.name for path in owners.iterdir()} == {'Elixir.Clbg.Main.ownership.etf', 'Elixir.Clbg.Support.ownership.etf', 'summary.txt'}
    assert (owners/'summary.txt').read_text() == EXPECTED_SUMMARY
    owner_records = []
    for path in sorted(owners.glob('*.etf')):
        assert path.is_file() and not path.is_symlink() and 0 < path.stat().st_size <= 16*1024*1024
        # The exact native probe validated the term shape; retain raw ETF, no host evaluator or conversion/compiler.
        with path.open('rb') as stream: assert stream.read(1) == b'\x83'
        digest = base.sha(path); base.bind(path, digest, pins)
        owner_records.append(dict(path=str(path), sha256=digest, bytes=path.stat().st_size))
    base.bind(owners/'summary.txt', base.sha(owners/'summary.txt'), pins)
    for benchmark in base.BENCHES:
        for variant in ('compact', 'optimized'):
            old_ebin = HERE/'.build/retained-otp-toolchain-v1/forms'/('elixir-'+benchmark+'-'+variant)/'ebin'
            assert old_ebin.is_dir() and not any(old_ebin.iterdir())
    base.verify(pins)
    base.save(output/'preparation-result.json', dict(status='first-Elixir-nbody-compile-owner-and-writer-observation-completed',
        retained_toolchain=dict(path=str(SAVED/'intake-result.json'), sha256=RESULT_SHA), whole20_inventory=saved['retained_source_inventory'],
        selected_profile='elixir-nbody-compact', compiler_command=compilation['detail'], owner_writer_command=probe['detail'],
        beams=beams, original_owner_terms=owner_records, compiled_source_debug_bindings=2,
        writer_exact_all256_bytes_multiple_chunks=True, writer_output_bytes=131328, input_EOF_observed=True,
        subject_compiler_commands=1, owner_stdio_commands=1, subject_main_executions=0, nonempty_input_qualified=False,
        full_stdio_qualified=False, remaining19_profiles_compiled=False, regex_formatting_and_math_qualified=False,
        runtime_environment=environment, original_sources_and_old_ebins_preserved=True,
        publisher_downloads=0, version_configuration_or_dry_reexecutions=0, fixture_reexecutions=0, actual_benchmark_runs=0,
        original_ownership_fixtures_reused=11, subject_execution_mode_qualified=False, source_work_qualified=False,
        canonical_loc=None, formatted_loc=None, chart_admission=False, pins=dict(pins)))


def main():
    parser = argparse.ArgumentParser()
    for name in ('revision', 'worker-sha256', 'probe-sha256', 'review-sha256'): parser.add_argument('--'+name, required=True)
    args = parser.parse_args()
    assert args.revision == 'first-compiler-stdio-owner-v1'
    assert sys.dont_write_bytecode and Path(sys.executable).resolve() == Path(PYTHON).resolve()
    assert all(re.fullmatch('[a-f0-9]{64}', value) for key, value in vars(args).items() if key.endswith('_sha256'))
    def interrupted(signum, frame): raise KeyboardInterrupt('Elixir first compiler/owner observation interrupted by signal '+str(signum))
    for signum in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP, signal.SIGALRM): signal.signal(signum, interrupted)
    with ExitStack() as leases:
        for path in (OWNER/'owner.lock', CLBG/'continuation/owned-jobs/elixir-retained-otp-toolchain-v1/owner.lock',
                     CLBG/'continuation/owned-jobs/gleam-publication-metadata-v1/owner.lock'):
            lease = leases.enter_context(path.open('rb')); fcntl.flock(lease, fcntl.LOCK_SH|fcntl.LOCK_NB)
            actual, named = os.fstat(lease.fileno()), path.stat()
            assert (actual.st_dev, actual.st_ino) == (named.st_dev, named.st_ino)
        owner = leases.enter_context((HERE/'first-compiler-stdio-owner-v1.lock').open('a+'))
        fcntl.flock(owner, fcntl.LOCK_EX|fcntl.LOCK_NB)
        print('WAIT common mutex: one Elixir compile and one unscored owner/writer command', flush=True)
        with LOCK.open('r+') as common:
            fcntl.flock(common, fcntl.LOCK_EX)
            actual, named = os.fstat(common.fileno()), LOCK.stat()
            assert (actual.st_dev, actual.st_ino) == (named.st_dev, named.st_ino) and actual.st_ino == 79859134
            output, build = HERE/'jobs'/args.revision, HERE/'.build'/args.revision
            assert not output.exists() and not build.exists() and not any(path.is_symlink() for path in (output, build, output.parent, build.parent, HERE))
            output.mkdir(parents=True, exist_ok=False); build.mkdir(parents=True, exist_ok=False)
            pins, failures, base = {}, [], None
            outcome = dict(status='failed-Elixir-first-compiler-owner-writer', subject_main_executions=0,
                fixture_reexecutions=0, actual_benchmark_runs=0, publisher_downloads=0, chart_admission=False)
            signal.alarm(420)
            try:
                raw = (OLD/'intake.py').read_bytes()
                assert not (OLD/'intake.py').stat().st_mode & 0o222 and hashlib.sha256(raw).hexdigest() == OLD_SHA
                import types
                name = 'elixir_first_compiler_intake_helpers'; assert name not in sys.modules
                base = types.ModuleType(name); base.__file__ = str(OLD/'intake.py'); sys.modules[name] = base
                exec(compile(raw, str(OLD/'intake.py'), 'exec'), base.__dict__)
                base.bind(OLD/'intake.py', OLD_SHA, pins)
                base.bind(Path(sys.executable).resolve(), 'ac60cfe0268614638d0ffa35f3b0284fc7b3a11482723793455e17eeb278509e', pins)
                run(args, output, build, pins, common.fileno(), base)
                outcome['status'] = 'first-Elixir-nbody-compile-owner-and-writer-observation-completed'
            except BaseException as error:
                outcome.update(error=str(error) or type(error).__name__, error_type=type(error).__name__)
                with (output/'actual-error.log').open('x') as stream: stream.write(traceback.format_exc())
                raise
            finally:
                signal.alarm(0); available = {}
                def digest(path):
                    with Path(path).open('rb') as stream: return hashlib.file_digest(stream, 'sha256').hexdigest()
                for path, expected in pins.items():
                    try: assert digest(path) == expected
                    except BaseException as error: failures.append(dict(path=path, error=str(error) or type(error).__name__))
                # Only outputs and ebin are evidence; private HOME/config/tmp (including cookies) are excluded.
                for tree in (output, build/'ebin'):
                    if tree.exists():
                        for path in tree.rglob('*'):
                            if path.is_file():
                                try: available[str(path)] = digest(path)
                                except BaseException as error: failures.append(dict(path=str(path), error=str(error) or type(error).__name__))
                if failures: outcome.update(status='failed-terminal-integrity', integrity_errors=failures)
                outcome.update(finished_at=datetime.now(timezone.utc).isoformat(), prerequisite_pins=pins, available_evidence_pins=available,
                    exclusive_common_mutex_held=True, common_mutex_inode=79859134, original_toolchain_sources_and_ebins_preserved=True,
                    private_startup_directories_not_evidence_artifacts=True)
                with (output/'outcome.json').open('x') as stream: json.dump(outcome, stream, indent=2); stream.write('\n')
                if failures: raise RuntimeError('Elixir compile/owner terminal integrity failed; partial evidence retained')


if __name__ == '__main__': main()
