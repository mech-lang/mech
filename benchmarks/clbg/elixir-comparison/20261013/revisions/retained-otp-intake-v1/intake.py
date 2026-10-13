"""Exact publisher intake + retained OTP probes; no subject compile or workload."""
import argparse
import base64
from datetime import datetime, timezone
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import signal
import stat
import sys
import traceback
import unicodedata
import urllib.request
import zipfile

P = Path(__file__).resolve().parent
HERE = P.parents[1]
CLBG = HERE.parents[1]
DRAFT = HERE/'drafts/private-v1'
LOCK = Path('/tmp/mech-clbg-timing.lock')
BENCHES = ('binarytrees','fannkuchredux','fasta','knucleotide','mandelbrot','nbody','pidigits','regexredux','revcomp','spectralnorm')
INPUTS = [HERE/name for name in ('TOOLCHAIN_CANDIDATE.json','RESEARCH_PLAN.md')]+[
    DRAFT/name for name in ('profiles.json','GATES.json','REVIEW.md','ROOT_REVIEW.md','support/support.ex')]+[
    DRAFT/'sources'/name/'main.ex' for name in BENCHES]
OTP = Path('/opt/homebrew/Cellar/erlang/29.1.1_2')
OTP_LIB = OTP/'lib/erlang'
METADATA = CLBG/'continuation/publication-gleam-metadata-v1/qualified/actual-metadata-v1'
METADATA_JOB = CLBG/'continuation/owned-jobs/gleam-publication-metadata-v1'
ROOT_REVIEW = CLBG/'continuation/jobs/gleam-metadata-closed-review-v1/receipt.json'
RUNTIME_SHA = '7dc62848e329047e47f46b9df9278e90b4f59f6ac99212f85ba3ee8999371da1'
OUTCOME_SHA = '452e3437a24a5fb6b9f1252d32af5872cbdd879bd2c05f240dda3ed6bb69687b'
TERMINAL_SHA = '87098c03510b7830b4e7ba7dfddedf60b447e913dd51653a8465c7abd483511d'
SERVICE = CLBG/'c3-comparison/20261012/revisions/private-v1/guarded_prepare.py'
SERVICE_SHA = '78b9f2870e0627b2aa6cc87ddcbd9ba54c1581ef7f28392f1859b79509dff9d4'
FIXTURES = CLBG/'gleam-comparison/20261012/proposal-seals/private-v1/generic-ownership-inventory.json'
FIXTURES_SHA = '44178d322cf7e5b390d3150c6e4cfb73b1e8758ab9bb8953dc7f6aa769f7b1e3'
FIXTURE_COMMAND = FIXTURES.with_name('generic-ownership-fixtures.json')
FIXTURE_COMMAND_SHA = 'acba51f10f2d70b8e746dca591564fc06fc218af2af20367ea9914f38c91937b'
PAYLOAD_URL = 'https://github.com/elixir-lang/elixir/releases/download/v1.20.4/elixir-otp-29.zip'
PAYLOAD_SHA = '7863c546cda13fecc949e562e326042451dacf8fd8698a36783cb71eeb223b46'
CHECKSUM_URL = PAYLOAD_URL+'.sha256sum'
CHECKSUM_SHA = '37b92a97277301498685dc7b6a81445dca02817004ac9fef7cfada322430899b'
FLAGS = ['+S','8:8','+A','1','+SDcpu','1:1','+SDio','1']


def sha(path):
    with Path(path).open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()


def save(path,value):
    with path.open('x') as stream:json.dump(value,stream,indent=2);stream.write('\n')


def bind(path,expected,pins):
    path = Path(path).resolve(strict=True)
    assert re.fullmatch('[a-f0-9]{64}',expected) and sha(path) == expected, ('Changed exact intake input',path)
    assert str(path) not in pins or pins[str(path)] == expected, ('Conflicting pin',path)
    pins[str(path)] = expected;return path


def read(path,expected,pins):
    return json.loads(bind(path,expected,pins).read_text())


def verify(pins):
    for path,digest in pins.items():assert sha(path) == digest, ('Changed retained input or output',path)


def cold(path,digest,name,pins):
    bind(path,digest,pins);assert name not in sys.modules
    raw = path.read_bytes();assert hashlib.sha256(raw).hexdigest() == digest
    spec = importlib.util.spec_from_file_location(name,path)
    module = importlib.util.module_from_spec(spec);sys.modules[name] = module
    exec(compile(raw,str(path),'exec'),module.__dict__);return module


def download(url,path,maximum):
    assert not path.exists()
    request = urllib.request.Request(url,headers={'User-Agent':'mech-Elixir-publisher-intake'})
    with urllib.request.urlopen(request,timeout=45) as response,path.open('xb') as stream:
        total = 0
        while chunk := response.read(65536):
            total += len(chunk);assert total <= maximum,'Publisher response exceeds reviewed bound'
            stream.write(chunk)


def extract(path,destination):
    destination.mkdir(exist_ok=False)
    with zipfile.ZipFile(path) as archive:
        entries = archive.infolist();assert 0 < len(entries) <= 8192
        assert sum(entry.file_size for entry in entries) <= 256*1024*1024
        names,records = set(),[]
        for entry in entries:
            name = PurePosixPath(entry.filename)
            assert entry.filename and '\\' not in entry.filename and not name.is_absolute()
            assert all(part not in ('','.','..') for part in entry.filename.rstrip('/').split('/'))
            key = unicodedata.normalize('NFD',str(name)).casefold();assert key not in names,'Duplicate/case-colliding archive member'
            names.add(key);assert not entry.flag_bits & 1,'Encrypted archive member'
            mode = entry.external_attr >> 16;kind = stat.S_IFMT(mode)
            assert kind in (0,stat.S_IFDIR) if entry.is_dir() else kind in (0,stat.S_IFREG),'Archive links/devices forbidden'
            target = destination.joinpath(*name.parts);assert target.resolve().is_relative_to(destination.resolve())
            if entry.is_dir():target.mkdir(parents=True,exist_ok=True)
            else:
                target.parent.mkdir(parents=True,exist_ok=True)
                with archive.open(entry) as source,target.open('xb') as output:shutil.copyfileobj(source,output)
                assert target.stat().st_size == entry.file_size
                target.chmod(0o444 | (mode & 0o111))
            records.append(dict(name=entry.filename,bytes=entry.file_size,crc32=entry.CRC,original_mode=mode,directory=entry.is_dir()))
    return records


def retained_runtime(args,pins):
    review = read(ROOT_REVIEW,args.runtime_review_sha256,pins)
    assert review['status'] == 'closed-qualified-Gleam-exact20-publication-metadata' and review['verified_pins'] == 6907
    assert review['runtime_sha256'] == RUNTIME_SHA and review['outcome_sha256'] == OUTCOME_SHA and review['durable_terminal_sha256'] == TERMINAL_SHA
    assert review['native_processes'] == review['fixture_reexecutions'] == review['benchmark_runs'] == 0 and review['chart_admission'] is False
    terminal = read(METADATA_JOB/'terminal.json',TERMINAL_SHA,pins)
    assert terminal['status'] == 'command-exited-zero-awaiting-report-review' and terminal['exit_code'] == 0 and terminal['stop_signals'] == []
    assert terminal['actual_command_started'] is True and terminal['command_identity_observation_missing'] is False
    assert not any(terminal.get(key) for key in ('error','error_type','exception_cleanup','error_log_write_failure'))
    outcome = read(METADATA/'outcome.json',OUTCOME_SHA,pins)
    assert outcome['status'] == 'metadata-qualified-awaiting-root-current-candidate-review' and outcome['native_subjects'] == outcome['fixture_reexecutions'] == 0
    assert outcome['chart_admission'] is False and not any(outcome.get(key) for key in ('error','error_type','integrity_errors','available_bytes_not_fully_pinned'))
    for group in (outcome['prerequisite_pins'],outcome['available_evidence_pins']):
        for path,digest in group.items():bind(path,digest,pins)
    process_records = [read(METADATA_JOB/name,sha(METADATA_JOB/name),pins) for name in ('request.json','command-started.json','worker-started.json')]
    request,started,worker = process_records
    assert request['command'] == started['command'] == worker['command'] and request['cwd'] == started['cwd'] == worker['cwd'] == str(CLBG.parent.parent)
    assert started['command_identity'] == terminal['command_identity'] and started['command_pid'] == terminal['command_pid'] == 34603
    assert worker['worker_identity'] == terminal['worker_identity'] and terminal['worker_identity']['pid'] == 34599
    assert request['common_mutex_owned_by_target_command'] is True and request['qualification_granted'] is request['automatic_retry'] is False
    command = request['command'];assert command[:5] == ['/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3.12','-I','-S','-B','-u']
    assert command[5:] == [str(METADATA.parents[1]/'qualify_descriptor.py'),'--revision','actual-metadata-v1',
        '--qualifier-sha256','1503e6a1e6bb87732eae780a83403fb145f0dc59b5a614f8b9d18c3e3ab73f3c',
        '--descriptor-sha256','ac19c1d896e556a683ece26c9ff8384dfe48c43c501ecd315b09365484ecb112',
        '--review-sha256','e58445350df8dc342520105372dbf2710c6e9fd5d91c6ddb439a196b5ef30031',
        '--root-review-sha256','db2e7999b951932089070f223d77649b0cdc9c1a579fdb5562102a206c303dec']
    actual = read(METADATA/'toolchain-runtime-receipt.json',RUNTIME_SHA,pins)
    assert actual['status'] == 'verified-actual-Gleam-BEAM-JIT-compiler-runtime-and-default-process-GC'
    assert actual['actual_toolchain'] == dict(re='10.47 2025-10-21',otp='29',schedulers=8,erts='17.1',jit='jit',arch='aarch64-apple-darwin25.6.0')
    assert actual['native_architecture'] == 'arm64'
    for path,digest in actual['pins'].items():bind(path,digest,pins)
    for name,path,digest in [('beam',OTP_LIB/'erts-17.1/bin/beam.smp','317a5aa96d7f488ee1f5377f76cb87f47279e235b3eff835ec3c98265074116b'),
        ('erlexec',OTP_LIB/'erts-17.1/bin/erlexec','60877b262a4aa18ad0d6aaf528e4ded12ad33b89cba9f245678411668c6997e6')]:
        assert actual['native_images'][name]['binary'] == str(path) and actual['native_images'][name]['sha256'] == digest
        bind(path,digest,pins)
    assert bind(OTP_LIB/'releases/29/OTP_VERSION','744d21462fddce81ca82770789d5eccb8c94cde90d412399a1514afff0815a0e',pins).read_text().strip() == '29.1.1'
    bind(OTP/'bin/erl','ef68f389ca08a7984d2cfbf9beedc44a111cf42215f77d020c3868a9a6ef88f1',pins)
    return actual


def run(args,output,build,pins,lock_fd):
    for name,value in [('intake.py',args.worker_sha256),('probe.exs',args.probe_sha256),('runner.exs',args.runner_sha256),('REVIEW.md',args.review_sha256)]:
        assert not (P/name).stat().st_mode & 0o222;bind(P/name,value,pins)
    inputs_path = args.input_pins_file.resolve(strict=True)
    assert inputs_path.is_relative_to(HERE) and not inputs_path.stat().st_mode & 0o222
    packet = read(inputs_path,args.input_pins_sha256,pins)
    assert set(packet) == {'schema','pins'} and packet['schema'] == 'frozen-elixir-intake-inputs-v1' and set(packet['pins']) == {str(path) for path in INPUTS}
    for path,digest in packet['pins'].items():
        assert not Path(path).stat().st_mode & 0o222;bind(path,digest,pins)
    actual_runtime = retained_runtime(args,pins)
    candidate = json.loads((HERE/'TOOLCHAIN_CANDIDATE.json').read_text())
    assert candidate['compiler_version'] == '1.20.4' and candidate['archive_url'] == PAYLOAD_URL and candidate['publisher_archive_sha256_observed'] == PAYLOAD_SHA
    assert candidate['runtime_candidate'] == str(OTP) and candidate['qualified'] is candidate['chart_admission'] is False
    fixture = read(FIXTURES,FIXTURES_SHA,pins);fixture_command = read(FIXTURE_COMMAND,FIXTURE_COMMAND_SHA,pins)
    assert fixture['status'] == 'passed' and fixture['successful'] is True and fixture['inventory'] == 'ownership'
    assert fixture['tests_run'] == fixture['discovered_tests'] == 11 and len(set(fixture['expected_test_ids'])) == len(set(fixture['actual_test_ids'])) == 11
    assert set(fixture['expected_test_ids']) == set(fixture['actual_test_ids']) and all(fixture[key] == [] for key in ('skipped','failures','errors','expected_failures','unexpected_successes'))
    assert fixture_command['exit_status'] == fixture_command['final_exit_status'] == 0 and fixture_command['inherited_common_mutex'] is True and not fixture_command.get('cleanup_errors')
    service = cold(SERVICE,SERVICE_SHA,'elixir_retained_original_owned_service',pins)
    guard = service.load_guard();harness = guard.load_harness()
    profiles = json.loads((DRAFT/'profiles.json').read_text())
    assert profiles['language'] == 'elixir' and profiles['physical_baseline_main_count'] == 10 and profiles['distinct_optimizer_claim'] is False
    rows = profiles['profiles'];assert len(rows) == len({row['id'] for row in rows}) == 20
    assert {(row['benchmark'],row['variant']) for row in rows} == {(name,variant) for name in BENCHES for variant in ('compact','optimized')}
    snapshot = build/'source-snapshot';snapshot.mkdir()
    for path in INPUTS:
        target = snapshot/path.relative_to(HERE);target.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(path,target)
        assert sha(target) == pins[str(path)];target.chmod(0o444);pins[str(target)] = pins[str(path)]
    payload,checksum = build/'elixir-otp-29.zip',build/'elixir-otp-29.zip.sha256sum'
    verify(pins);download(CHECKSUM_URL,checksum,4096);bind(checksum,CHECKSUM_SHA,pins)
    match = re.fullmatch(r'([a-f0-9]{64})[ \t]+\*?elixir-otp-29\.zip\r?\n?',checksum.read_text());assert match and match[1] == PAYLOAD_SHA
    download(PAYLOAD_URL,payload,32*1024*1024);bind(payload,PAYLOAD_SHA,pins)
    compiler = build/'elixir';members = extract(payload,compiler)
    save(output/'publisher-intake.json',dict(payload_url=PAYLOAD_URL,payload_sha256=PAYLOAD_SHA,checksum_url=CHECKSUM_URL,
        checksum_sha256=CHECKSUM_SHA,archive_members=members,extracted_root=str(compiler),publisher_listing='https://github.com/elixir-lang/elixir/releases/expanded_assets/v1.20.4'))
    elixir,elixirc = compiler/'bin/elixir',compiler/'bin/elixirc'
    assert all(path.is_file() and not path.is_symlink() and os.access(path,os.X_OK) for path in (elixir,elixirc))
    assert 'ELIXIR_VERSION=1.20.4' in elixir.read_text() and 'exec "$SCRIPT_PATH"/elixir +elixirc "$@"' in elixirc.read_text()
    assert (compiler/'lib/elixir/ebin/elixir.beam').is_file() and (compiler/'lib/elixir/ebin/Elixir.System.beam').is_file()
    for path in compiler.rglob('*'):
        assert not path.is_symlink()
        if path.is_file():pins[str(path)] = sha(path)
    inventory = []
    for row in rows:
        assert row['id'] == 'elixir-'+row['benchmark']+'-'+row['variant'] and row['source'] == 'sources/'+row['benchmark']+'/main.ex'
        assert row['authored_helpers'] == ['support/support.ex'] and row['maximum_subject_workers'] == 1 and row['source_input_materialized'] is False
        folder = build/'forms'/row['id'];folder.mkdir(parents=True,exist_ok=False);ebin = folder/'ebin';ebin.mkdir()
        sources = []
        for origin,name,role in [(DRAFT/row['source'],'main.ex','application'),(DRAFT/'support/support.ex','support.ex','application-helper'),(P/'runner.exs','runner.exs','application-launcher')]:
            target = folder/name;shutil.copyfile(origin,target);assert sha(target) == pins[str(origin)];target.chmod(0o444)
            pins[str(target)] = pins[str(origin)];sources.append(dict(path=str(target),sha256=pins[str(target)],origin=str(origin),role=role,counted=True))
        inventory.append(dict(id=row['id'],benchmark=row['benchmark'],variant=row['variant'],alias_of=row['alias_of'],
            physical_inputs=sources,source_input_materialized=True,compile_command_proposed=[str(elixirc),'-o',str(ebin),str(folder/'support.ex'),str(folder/'main.ex')],
            runtime_command_proposed=[str(elixir),'-pa',str(ebin),str(folder/'runner.exs'),'{n}'],
            subject_compiler_executed=False,work_qualified=False,canonical_loc=None,formatted_loc=None,chart_admission=False))
    save(output/'source-input-inventory.json',dict(status='exact20-inputs-materialized-not-compiled',forms=inventory,
        original_input_packet=dict(path=str(inputs_path),sha256=args.input_pins_sha256),runner_counted=True,distinct_optimizer_claim=False))
    temporary = build/'tmp';temporary.mkdir()
    environment = dict(PATH=str(OTP/'bin')+':'+str(compiler/'bin')+':/usr/bin:/bin:/usr/sbin:/sbin',HOME='/Users/cim310',
        LC_ALL='C',LANG='C',TMPDIR=str(temporary)+os.sep,ERTS_BIN=str(OTP/'bin')+os.sep,
        ERL_FLAGS='',ERL_AFLAGS='',ERL_ZFLAGS='',ERL_LIBS='',ELIXIR_ERL_OPTIONS=' '.join(FLAGS),
        ERL_COMPILER_OPTIONS='',ERL_CRASH_DUMP=str(output/'erl-crash.dump'),OMP_NUM_THREADS='1',OPENBLAS_NUM_THREADS='1',VECLIB_MAXIMUM_THREADS='1')
    verify(pins)
    dry = service.owned_command(guard,harness,[str(elixir),'--version'],dict(environment,ELIXIR_CLI_DRY_RUN='1'),output/'launch',lock_fd,30,'resolved-version-launch')
    expected = [str(OTP/'bin/erl'),'-noshell','-elixir_root',str(compiler/'lib'),'-pa',str(compiler/'lib/elixir/ebin'),*FLAGS,'-s','elixir','start_cli','-extra','--version']
    assert dry['stdout'].decode().split() == expected,'Actual Elixir script launch differs'
    version = service.owned_command(guard,harness,[str(elixir),'--version'],environment,output/'version',lock_fd,90,'compiler-runtime-version')
    assert 'Elixir 1.20.4' in version['stdout'].decode() and 'Erlang/OTP 29' in version['stdout'].decode()
    probe = service.owned_command(guard,harness,[str(elixir),str(P/'probe.exs')],environment,output/'probe',lock_fd,90,'runtime-configuration')
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
        assert resolved.is_relative_to(OTP_LIB.resolve()) or resolved.is_relative_to((compiler/'lib').resolve()) or (
            resolved == service.HERE.resolve() and not list(resolved.glob('*.beam'))), ('Unreviewed Elixir code path',path)
    gc_defaults_text = base64.b64decode(values['GC_DEFAULTS_TEXT64'],validate=True).decode()
    verify(pins)
    save(output/'intake-result.json',dict(status='publisher-toolchain-and-exact20-inputs-ready-awaiting-compiler-stdio-owner-review',
        compiler=str(elixirc),runtime_erl=str(OTP/'bin/erl'),actual_version_stdout=version['stdout'].decode(),resolved_launch=expected,
        runtime_environment=environment,configuration_probe=values,actual_code_paths=paths,actual_gc_defaults_text=gc_defaults_text,
        original_runtime_receipt=dict(path=str(METADATA/'toolchain-runtime-receipt.json'),sha256=RUNTIME_SHA),
        original_ownership_fixtures_reused=11,fixture_reexecutions=0,materialized_forms=20,subject_compiler_commands=0,
        raw_stdio_qualified=False,subject_execution_mode_qualified=False,collector_policy_qualified=False,
        source_work_qualified=False,canonical_loc=None,formatted_loc=None,actual_benchmark_runs=0,chart_admission=False,pins=dict(pins)))


def main():
    parser = argparse.ArgumentParser()
    for name in ('revision','worker-sha256','probe-sha256','runner-sha256','review-sha256','input-pins-sha256','runtime-review-sha256'):parser.add_argument('--'+name,required=True)
    parser.add_argument('--input-pins-file',type=Path,required=True);args = parser.parse_args()
    assert re.fullmatch('[a-z0-9][a-z0-9-]{0,63}',args.revision) and all(re.fullmatch('[a-f0-9]{64}',value) for key,value in vars(args).items() if key.endswith('_sha256'))
    def interrupted(signum,frame):raise KeyboardInterrupt('Elixir intake interrupted by signal '+str(signum))
    for signum in (signal.SIGINT,signal.SIGTERM,signal.SIGHUP,signal.SIGALRM):signal.signal(signum,interrupted)
    with (METADATA_JOB/'owner.lock').open('rb') as original:
        fcntl.flock(original,fcntl.LOCK_SH|fcntl.LOCK_NB)
        opened,named = os.fstat(original.fileno()),Path(original.name).stat();assert (opened.st_dev,opened.st_ino) == (named.st_dev,named.st_ino)
        with (HERE/'retained-otp-intake-v1-owner.lock').open('a+') as owner:
            fcntl.flock(owner,fcntl.LOCK_EX|fcntl.LOCK_NB)
            print('WAIT common mutex: Elixir exact publisher/OTP intake only',flush=True)
            with LOCK.open('a+') as common:
                fcntl.flock(common,fcntl.LOCK_EX)
                opened,named = os.fstat(common.fileno()),LOCK.stat();assert (opened.st_dev,opened.st_ino) == (named.st_dev,named.st_ino) and opened.st_ino == 79859134
                output,build = HERE/'jobs'/args.revision,HERE/'.build'/args.revision
                assert not output.exists() and not build.exists(),'No existing-output retry/resume'
                output.mkdir(parents=True);build.mkdir(parents=True);pins = {}
                outcome = dict(status='failed-Elixir-intake',chart_admission=False,subject_compiler_commands=0,actual_benchmark_runs=0,fixture_reexecutions=0)
                signal.alarm(600)
                try:
                    bind(Path(sys.executable).resolve(),sha(Path(sys.executable).resolve()),pins)
                    run(args,output,build,pins,common.fileno());outcome['status'] = 'publisher-toolchain-and-exact20-inputs-ready-awaiting-compiler-stdio-owner-review'
                except BaseException as error:
                    outcome.update(error=str(error) or type(error).__name__,error_type=type(error).__name__)
                    with (output/'actual-error.log').open('x') as stream:stream.write(traceback.format_exc())
                    raise
                finally:
                    signal.alarm(0);failures,available = [],{}
                    for path,digest in list(pins.items()):
                        try:verify({path:digest})
                        except BaseException as error:failures.append(dict(path=path,error=str(error),error_type=type(error).__name__))
                    for folder in (output,build):
                        for path in folder.rglob('*'):
                            if path.is_file():
                                try:available[str(path)] = sha(path)
                                except BaseException as error:failures.append(dict(path=str(path),error=str(error),error_type=type(error).__name__))
                    if failures:outcome.update(status='failed-terminal-integrity',integrity_errors=failures)
                    outcome.update(finished_at=datetime.now(timezone.utc).isoformat(),prerequisite_pins=pins,available_evidence_pins=available,
                        exclusive_common_mutex_held=True,common_mutex_inode=79859134,original_sources_and_OTP_preserved=True)
                    save(output/'outcome.json',outcome)
                    if failures:raise RuntimeError('Elixir intake terminal integrity failed; actual partial evidence preserved')


if __name__ == '__main__':main()
