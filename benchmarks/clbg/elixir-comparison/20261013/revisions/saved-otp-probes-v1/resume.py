"""Reuse the actual publisher/source intake; run only two missing OTP probes."""
import argparse
import base64
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
OLD = HERE/'revisions/retained-otp-intake-v1'
OLD_WORKER_SHA = '51512225c30d507b1f19eef119b1402ef6455a9682d4b6cb1b80b90b5cb44cd9'
OLD_PROBE_SHA = '95ed4047ce05389cf86421acbc8a95b20dc865fe41d3bac99d2d72901fcb5aad'
OLD_RUNNER_SHA = 'bdad2389d437fd4c69122e5d25b3065412b4bca0c272631667283273392ba8ff'
OLD_NOTE_SHA = '36500ff5d22c1e2a48701a2d13b615e3bc51f2dd1ee3ff2f0f1b66d285af503c'
OLD_JOB = CLBG/'continuation/owned-jobs/elixir-retained-otp-toolchain-v1'
OLD_OUT = HERE/'jobs/retained-otp-toolchain-v1'
OLD_BUILD = HERE/'.build/retained-otp-toolchain-v1'
ROOT_REVIEW = CLBG/'continuation/jobs/elixir-retained-otp-closed-review-v1/receipt.json'
ROOT_REVIEW_SHA = 'fc8d2ab0b7dda8181fdc5770316efc032afcf0078149466ba8d16b584043c129'
OUTCOME_SHA = 'd76a2d019e4ec3a480df223392b76eff4cf2b6efcea74b7cf1b48c964622aed5'
TERMINAL_SHA = 'ed9bb4ac28092872d87915aeb5e35318ed912659946a3c578d56b7dd0dbe2c45'
PUBLISHER_SHA = '29857ceb0f0eef651490ea683d9ff3381d394e3e858eb1557d2f3f627c6f70f6'
INVENTORY_SHA = '80309d98f655c9d0894cf4c17c99d1909fcb3bd31d9408282b36bd0f5ecb6caf'
DRY_SHA = 'd9310eabb28d0acfd477e7a3e0d895dbe161854363d290d2b23a2b10a82e2942'
DRY_STDOUT_SHA = '4fee4530c68f17deaedc2eb6b73a5bd1544dd9fe7774100a3aebce34edf8b8e7'
ELIXIR_SCRIPT_SHA = '5cb23d89a78f75589b06fade3097e217b77e63416d61c1800f674579748c4307'
ELIXIRC_SCRIPT_SHA = '448cc9ffdc2f23604eccfe370076b5afde6069a97e47dcb8d42fe22d1e169ed1'
PYTHON = '/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3.12'
RUNTIME_REVIEW_SHA = '406ac95e0f93b99efc26d7bb62d0fe593568860c8d6cc3b8bfa39918ca1f9cdc'


def cold_original(pins):
    path = OLD/'intake.py'; assert not path.stat().st_mode & 0o222
    raw = path.read_bytes()
    assert hashlib.sha256(raw).hexdigest() == OLD_WORKER_SHA
    name = 'elixir_saved_actual_intake_helpers_v1'; assert name not in sys.modules
    module = types.ModuleType(name); module.__file__ = str(path); sys.modules[name] = module
    exec(compile(raw,str(path),'exec'),module.__dict__)
    module.bind(path,OLD_WORKER_SHA,pins)
    return module


def saved_intake(base,pins):
    review = base.read(ROOT_REVIEW,ROOT_REVIEW_SHA,pins)
    assert review['status'] == 'closed-successful-publisher-intake-stopped-at-dry-launch-spelling-check' and review['verified_pins'] == 7741
    assert review['outcome_sha256'] == OUTCOME_SHA and review['durable_terminal_sha256'] == TERMINAL_SHA
    assert review['publisher_intake_sha256'] == PUBLISHER_SHA and review['source_inventory_sha256'] == INVENTORY_SHA
    assert review['successful_dry_command'] == dict(path=str(OLD_OUT/'launch/resolved-version-launch.json'),sha256=DRY_SHA)
    assert review['materialized_forms'] == 20 and review['authored_inputs_per_form'] == 3 and review['publisher_downloads'] == 2
    assert all(review[key] == 0 for key in ('beam_version_commands','configuration_probes','subject_compiler_commands','fixture_reexecutions','benchmark_runs'))
    terminal = base.read(OLD_JOB/'terminal.json',TERMINAL_SHA,pins)
    request,started,worker = [base.read(OLD_JOB/name,base.sha(OLD_JOB/name),pins) for name in ('request.json','command-started.json','worker-started.json')]
    command = [PYTHON,'-I','-S','-B','-u',str(OLD/'intake.py'),'--revision','retained-otp-toolchain-v1',
        '--worker-sha256',OLD_WORKER_SHA,'--probe-sha256',OLD_PROBE_SHA,'--runner-sha256',OLD_RUNNER_SHA,
        '--review-sha256',OLD_NOTE_SHA,'--input-pins-file',str(HERE/'proposals/retained-otp-inputs-v1.json'),
        '--input-pins-sha256','ca683f61769bc6c7930ae6d1e7c29eb55982cb9119200846b33d0b8edb215456',
        '--runtime-review-sha256',RUNTIME_REVIEW_SHA]
    observed = terminal['command_identity']['listing'].split(None,7)
    assert terminal['status'] == 'command-failed' and type(terminal['exit_code']) is int and terminal['exit_code'] == 1
    assert terminal['actual_command_started'] is True and terminal['command_identity_observation_missing'] is False and terminal['stop_signals'] == []
    assert terminal['qualification_granted'] is terminal['automatic_retry'] is False and not any(terminal.get(key) for key in ('error','exception_cleanup','error_log_write_failure'))
    assert request['command'] == started['command'] == worker['command'] == command
    assert request['cwd'] == started['cwd'] == worker['cwd'] == str(CLBG.parent.parent)
    assert request['common_mutex_owned_by_target_command'] is True and request['qualification_granted'] is request['automatic_retry'] is False
    assert started['command_identity'] == terminal['command_identity'] and started['command_pid'] == terminal['command_pid'] == 46962
    assert started['command_identity_observation_missing'] is False and worker['worker_identity'] == terminal['worker_identity']
    assert terminal['worker_identity']['pid'] == 46958 and observed[:2] == ['46962','46958'] and observed[7].split() == command
    outcome = base.read(OLD_OUT/'outcome.json',OUTCOME_SHA,pins)
    assert outcome['status'] == 'failed-Elixir-intake' and outcome['error_type'] == 'AssertionError' and outcome['error'] == 'Actual Elixir script launch differs'
    assert outcome['chart_admission'] is False and outcome['subject_compiler_commands'] == outcome['actual_benchmark_runs'] == outcome['fixture_reexecutions'] == 0
    assert outcome['exclusive_common_mutex_held'] is True and outcome['common_mutex_inode'] == 79859134 and outcome['original_sources_and_OTP_preserved'] is True
    assert not any(outcome.get(key) for key in ('integrity_errors','available_bytes_not_fully_pinned'))
    for group in (outcome['prerequisite_pins'],outcome['available_evidence_pins']):
        for path,digest in group.items():base.bind(path,digest,pins)
    for name,digest in [('probe.exs',OLD_PROBE_SHA),('runner.exs',OLD_RUNNER_SHA),('REVIEW.md',OLD_NOTE_SHA)]:
        assert not (OLD/name).stat().st_mode & 0o222;base.bind(OLD/name,digest,pins)
    packet = base.read(HERE/'proposals/retained-otp-inputs-v1.json',command[19],pins)
    assert set(packet) == {'schema','pins'} and packet['schema'] == 'frozen-elixir-intake-inputs-v1' and set(packet['pins']) == {str(path) for path in base.INPUTS}
    for path,digest in packet['pins'].items():base.bind(path,digest,pins)
    publisher = base.read(OLD_OUT/'publisher-intake.json',PUBLISHER_SHA,pins)
    compiler = OLD_BUILD/'elixir'
    assert publisher['payload_url'] == base.PAYLOAD_URL and publisher['payload_sha256'] == base.PAYLOAD_SHA
    assert publisher['checksum_url'] == base.CHECKSUM_URL and publisher['checksum_sha256'] == base.CHECKSUM_SHA and publisher['extracted_root'] == str(compiler)
    base.bind(OLD_BUILD/'elixir-otp-29.zip',base.PAYLOAD_SHA,pins);base.bind(OLD_BUILD/'elixir-otp-29.zip.sha256sum',base.CHECKSUM_SHA,pins)
    assert 0 < len(publisher['archive_members']) <= 8192
    inventory = base.read(OLD_OUT/'source-input-inventory.json',INVENTORY_SHA,pins)
    assert inventory['status'] == 'exact20-inputs-materialized-not-compiled' and inventory['runner_counted'] is True and inventory['distinct_optimizer_claim'] is False
    assert inventory['original_input_packet'] == dict(path=str(HERE/'proposals/retained-otp-inputs-v1.json'),sha256=command[19])
    rows = inventory['forms']; assert len(rows) == len({row['id'] for row in rows}) == 20
    assert {(row['benchmark'],row['variant']) for row in rows} == {(name,variant) for name in base.BENCHES for variant in ('compact','optimized')}
    for row in rows:
        folder = OLD_BUILD/'forms'/row['id']
        assert row['id'] == 'elixir-'+row['benchmark']+'-'+row['variant'] and row['alias_of'] == 'elixir-'+row['benchmark']+'-baseline-v1'
        expected = [(base.DRAFT/'sources'/row['benchmark']/'main.ex','main.ex','application'),
            (base.DRAFT/'support/support.ex','support.ex','application-helper'),(OLD/'runner.exs','runner.exs','application-launcher')]
        assert row['physical_inputs'] == [dict(path=str(folder/name),sha256=pins[str(origin)],origin=str(origin),role=role,counted=True) for origin,name,role in expected]
        assert row['compile_command_proposed'] == [str(compiler/'bin/elixirc'),'-o',str(folder/'ebin'),str(folder/'support.ex'),str(folder/'main.ex')]
        assert row['runtime_command_proposed'] == [str(compiler/'bin/elixir'),'-pa',str(folder/'ebin'),str(folder/'runner.exs'),'{n}']
        assert row['source_input_materialized'] is True and row['subject_compiler_executed'] is row['work_qualified'] is row['chart_admission'] is False
        assert row['canonical_loc'] is row['formatted_loc'] is None and (folder/'ebin').is_dir() and not any((folder/'ebin').iterdir())
        for source in row['physical_inputs']:
            base.bind(source['path'],source['sha256'],pins);assert not Path(source['path']).stat().st_mode & 0o222
    dry = base.read(OLD_OUT/'launch/resolved-version-launch.json',DRY_SHA,pins)
    base.bind(OLD_OUT/'launch/resolved-version-launch.stdout',DRY_STDOUT_SHA,pins)
    assert dry['command'] == [str(compiler/'bin/elixir'),'--version'] and dry['exit_status'] == dry['final_exit_status'] == 0
    assert dry['pid'] == dry['identity']['pid'] == 47106 and dry['inherited_common_mutex'] is True and dry['timeout_seconds'] == 30
    assert dry['preparation_only'] is dry['never_chart_eligible'] is True and not dry.get('cleanup_errors')
    assert (OLD_OUT/'launch/resolved-version-launch.stderr').read_bytes() == b''
    literal_launch = ['erl','-noshell','-elixir_root',str(compiler/'bin/../lib'),'-pa',str(compiler/'bin/../lib/elixir/ebin'),
        *base.FLAGS,'-s','elixir','start_cli','-extra','--version']
    assert (OLD_OUT/'launch/resolved-version-launch.stdout').read_text().split() == literal_launch
    elixir = base.bind(compiler/'bin/elixir',ELIXIR_SCRIPT_SHA,pins)
    base.bind(compiler/'bin/elixirc',ELIXIRC_SCRIPT_SHA,pins)
    script = elixir.read_text()
    assert 'ERL_EXEC="erl"' in script and 'ERTS_BIN=\nERTS_BIN="$ERTS_BIN"' in script
    assert 'set -- "$ERTS_BIN$ERL_EXEC" -noshell -elixir_root "$SCRIPT_PATH"/../lib -pa "$SCRIPT_PATH"/../lib/elixir/ebin $ELIXIR_ERL_OPTIONS $ERL "$@"' in script
    assert 'exec "$@"' in script and 'ELIXIR_VERSION=1.20.4' in script
    assert not (OLD_OUT/'version').exists() and not (OLD_OUT/'probe').exists() and not (OLD_OUT/'intake-result.json').exists()
    return compiler,literal_launch


def run(args,output,build,pins,lock_fd,base):
    for name,digest in [('resume.py',args.worker_sha256),('REVIEW.md',args.review_sha256)]:
        assert not (P/name).stat().st_mode & 0o222;base.bind(P/name,digest,pins)
    compiler,literal_launch = saved_intake(base,pins)
    args.runtime_review_sha256 = RUNTIME_REVIEW_SHA
    actual_runtime = base.retained_runtime(args,pins)
    fixture = base.read(base.FIXTURES,base.FIXTURES_SHA,pins)
    command = base.read(base.FIXTURE_COMMAND,base.FIXTURE_COMMAND_SHA,pins)
    assert fixture['status'] == 'passed' and fixture['successful'] is True and fixture['inventory'] == 'ownership'
    assert fixture['tests_run'] == fixture['discovered_tests'] == 11 and len(set(fixture['expected_test_ids'])) == len(set(fixture['actual_test_ids'])) == 11
    assert set(fixture['expected_test_ids']) == set(fixture['actual_test_ids']) and all(fixture[key] == [] for key in ('skipped','failures','errors','expected_failures','unexpected_successes'))
    assert command['exit_status'] == command['final_exit_status'] == 0 and command['inherited_common_mutex'] is True and not command.get('cleanup_errors')
    service = base.cold(base.SERVICE,base.SERVICE_SHA,'elixir_saved_original_owned_service',pins)
    guard = service.load_guard();harness = guard.load_harness()
    task_home,task_config,temporary = build/'home',build/'config',build/'tmp'
    for path in (task_home,task_config,temporary):path.mkdir(exist_ok=False)
    assert not any(task_home.iterdir()) and not any(task_config.iterdir())
    assert not list(service.HERE.glob('*.beam')) and not (service.HERE/'start.boot').exists() and not (service.HERE/'.erlang').exists()
    environment = dict(PATH=str(base.OTP/'bin')+':'+str(compiler/'bin')+':/usr/bin:/bin:/usr/sbin:/sbin',HOME=str(task_home),
        XDG_CONFIG_HOME=str(task_config),LC_ALL='C',LANG='C',TMPDIR=str(temporary)+os.sep,
        ERL_FLAGS='',ERL_AFLAGS='',ERL_ZFLAGS='',ERL_LIBS='',ELIXIR_ERL_OPTIONS=' '.join(base.FLAGS),
        ERL_COMPILER_OPTIONS='',ERL_CRASH_DUMP=str(output/'erl-crash.dump'),OMP_NUM_THREADS='1',OPENBLAS_NUM_THREADS='1',VECLIB_MAXIMUM_THREADS='1')
    resolved_erl = shutil.which('erl',path=environment['PATH'])
    assert resolved_erl == str(base.OTP/'bin/erl') and Path(resolved_erl).resolve(strict=True) == (base.OTP/'bin/erl').resolve(strict=True)
    base.bind(resolved_erl,'ef68f389ca08a7984d2cfbf9beedc44a111cf42215f77d020c3868a9a6ef88f1',pins)
    assert literal_launch[0] == 'erl' and Path(literal_launch[3]).resolve() == (compiler/'lib').resolve()
    assert Path(literal_launch[5]).resolve() == (compiler/'lib/elixir/ebin').resolve()
    base.save(output/'startup-contract.json',dict(status='source-bound-normal-Elixir-CLI-startup-before-first-BEAM-command',
        saved_literal_dry_argv=literal_launch,resolved_erl=resolved_erl,actual_dry_command_reused=dict(path=str(OLD_OUT/'launch/resolved-version-launch.json'),sha256=DRY_SHA),
        path_resolution_verified=True,vendor_script_modified=False,inherited_ERTS_BIN_not_used=True,
        runtime_environment=environment,private_home_and_config_initially_empty=True,user_home_files_read=False,
        default_module_loading='interactive',module_loading_basis='unchanged official CLI argv and OTP29.1.1 documented default',
        shell='noshell',default_boot_lookup_current_directory_empty=True,subject_compiler_commands=0,chart_admission=False))
    base.verify(pins)
    version = service.owned_command(guard,harness,[str(compiler/'bin/elixir'),'--version'],environment,output/'version',lock_fd,90,'compiler-runtime-version')
    assert 'Elixir 1.20.4' in version['stdout'].decode() and 'Erlang/OTP 29' in version['stdout'].decode()
    assert shutil.which('erl',path=environment['PATH']) == resolved_erl;base.verify(pins)
    probe = service.owned_command(guard,harness,[str(compiler/'bin/elixir'),str(OLD/'probe.exs')],environment,output/'probe',lock_fd,90,'runtime-configuration')
    values,paths = {},[]
    for line in probe['stdout'].decode().splitlines():
        key,value = line.split('=',1)
        if key == 'CODE_PATH64':paths.append(base64.b64decode(value,validate=True).decode())
        else:assert key not in values;values[key] = value
    required = dict(ELIXIR='1.20.4',OTP='29',ERTS='17.1',ARCH=actual_runtime['actual_toolchain']['arch'],EMU_FLAVOR='jit',
        SCHEDULERS='8',SCHEDULERS_ONLINE='8',DIRTY_CPU='1',DIRTY_CPU_ONLINE='1',DIRTY_IO='1',ASYNC_THREADS='1',CLBG_MAIN_LOADED='false',CLBG_SUPPORT_LOADED='false')
    assert all(values[key] == value for key,value in required.items()) and int(values['BOOTSTRAP_PROCESS_COUNT']) > 0
    assert set(values) == set(required) | {'BOOTSTRAP_PROCESS_COUNT','GC_DEFAULTS_TEXT64','ELIXIR_BEAM64','SYSTEM_BEAM64'} and paths
    for key,name in [('ELIXIR_BEAM64','elixir.beam'),('SYSTEM_BEAM64','Elixir.System.beam')]:
        assert Path(base64.b64decode(values[key],validate=True).decode()).resolve() == (compiler/'lib/elixir/ebin'/name).resolve()
    for path in paths:
        resolved = Path(path).resolve() if Path(path).is_absolute() else (service.HERE/path).resolve()
        assert resolved.is_relative_to(base.OTP_LIB.resolve()) or resolved.is_relative_to((compiler/'lib').resolve()) or (
            resolved == service.HERE.resolve() and not list(resolved.glob('*.beam'))), ('Unreviewed actual Elixir code path',path)
    gc_defaults_text = base64.b64decode(values['GC_DEFAULTS_TEXT64'],validate=True).decode()
    base.verify(pins)
    base.save(output/'intake-result.json',dict(status='saved-publisher-toolchain-and-exact20-inputs-ready-awaiting-compiler-stdio-owner-review',
        compiler=str(compiler/'bin/elixirc'),runtime_erl=resolved_erl,saved_literal_dry_argv=literal_launch,
        actual_version_stdout=version['stdout'].decode(),runtime_environment=environment,configuration_probe=values,
        actual_code_paths=paths,actual_gc_defaults_text=gc_defaults_text,retained_intake=dict(path=str(OLD_OUT/'outcome.json'),sha256=OUTCOME_SHA),
        retained_source_inventory=dict(path=str(OLD_OUT/'source-input-inventory.json'),sha256=INVENTORY_SHA),
        publisher_downloads=0,archive_extractions=0,source_rematerializations=0,dry_command_reexecutions=0,
        beam_version_commands=1,configuration_probes=1,original_ownership_fixtures_reused=11,fixture_reexecutions=0,
        materialized_forms_reused=20,subject_compiler_commands=0,raw_stdio_qualified=False,subject_execution_mode_qualified=False,
        collector_policy_qualified=False,source_work_qualified=False,canonical_loc=None,formatted_loc=None,actual_benchmark_runs=0,
        chart_admission=False,pins=dict(pins)))


def main():
    parser = argparse.ArgumentParser()
    for name in ('revision','worker-sha256','review-sha256'):parser.add_argument('--'+name,required=True)
    args = parser.parse_args()
    assert re.fullmatch('[a-z0-9][a-z0-9-]{0,63}',args.revision)
    assert all(re.fullmatch('[a-f0-9]{64}',value) for key,value in vars(args).items() if key.endswith('_sha256'))
    def interrupted(signum,frame):raise KeyboardInterrupt('Elixir saved probe interrupted by signal '+str(signum))
    for signum in (signal.SIGINT,signal.SIGTERM,signal.SIGHUP,signal.SIGALRM):signal.signal(signum,interrupted)
    with (OLD_JOB/'owner.lock').open('rb') as dependency:
        fcntl.flock(dependency,fcntl.LOCK_SH|fcntl.LOCK_NB)
        opened,named = os.fstat(dependency.fileno()),Path(dependency.name).stat();assert (opened.st_dev,opened.st_ino) == (named.st_dev,named.st_ino)
        with (CLBG/'continuation/owned-jobs/gleam-publication-metadata-v1/owner.lock').open('rb') as runtime:
            fcntl.flock(runtime,fcntl.LOCK_SH|fcntl.LOCK_NB)
            opened,named = os.fstat(runtime.fileno()),Path(runtime.name).stat();assert (opened.st_dev,opened.st_ino) == (named.st_dev,named.st_ino)
            with (HERE/'saved-otp-probes-v1-owner.lock').open('a+') as owner:
                fcntl.flock(owner,fcntl.LOCK_EX|fcntl.LOCK_NB)
                print('WAIT common mutex: two missing Elixir toolchain probes only',flush=True)
                with LOCK.open('a+') as common:
                    fcntl.flock(common,fcntl.LOCK_EX)
                    opened,named = os.fstat(common.fileno()),LOCK.stat();assert (opened.st_dev,opened.st_ino) == (named.st_dev,named.st_ino) and opened.st_ino == 79859134
                    output,build = HERE/'jobs'/args.revision,HERE/'.build'/args.revision
                    assert not output.exists() and not build.exists(),'Fresh additive output/private HOME only'
                    output.mkdir(parents=True);build.mkdir(parents=True);pins = {}
                    outcome = dict(status='failed-Elixir-saved-probes',chart_admission=False,subject_compiler_commands=0,actual_benchmark_runs=0,
                        fixture_reexecutions=0,publisher_downloads=0,archive_extractions=0,source_rematerializations=0,dry_command_reexecutions=0)
                    signal.alarm(300);base = None
                    try:
                        base = cold_original(pins)
                        base.bind(Path(sys.executable).resolve(),base.sha(Path(sys.executable).resolve()),pins)
                        run(args,output,build,pins,common.fileno(),base)
                        outcome['status'] = 'saved-publisher-toolchain-and-exact20-inputs-ready-awaiting-compiler-stdio-owner-review'
                    except BaseException as error:
                        outcome.update(error=str(error) or type(error).__name__,error_type=type(error).__name__)
                        with (output/'actual-error.log').open('x') as stream:stream.write(traceback.format_exc())
                        raise
                    finally:
                        signal.alarm(0);failures,available = [],{}
                        def digest(path):
                            with Path(path).open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()
                        for path,value in pins.items():
                            try:assert digest(path) == value
                            except BaseException as error:failures.append(dict(path=path,error=str(error),error_type=type(error).__name__))
                        # Private HOME is startup state, not an evidence artifact.
                        # In particular, generated runtime cookie bytes are never read or journalled.
                        for path in output.rglob('*'):
                            if path.is_file():
                                try:available[str(path)] = digest(path)
                                except BaseException as error:failures.append(dict(path=str(path),error=str(error),error_type=type(error).__name__))
                        if failures:outcome.update(status='failed-terminal-integrity',integrity_errors=failures)
                        outcome.update(finished_at=datetime.now(timezone.utc).isoformat(),prerequisite_pins=pins,available_evidence_pins=available,
                            exclusive_common_mutex_held=True,common_mutex_inode=79859134,original_publisher_source_dry_and_OTP_preserved=True,
                            private_startup_directories_not_evidence_artifacts=True)
                        with (output/'outcome.json').open('x') as stream:json.dump(outcome,stream,indent=2);stream.write('\n')
                        if failures:raise RuntimeError('Elixir saved probes terminal integrity failed; original and new receipts preserved')


if __name__ == '__main__':main()
