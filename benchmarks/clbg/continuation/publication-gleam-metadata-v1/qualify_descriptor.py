"""Metadata only: exact existing Gleam/BEAM receipts; no native or fixture runs."""
import argparse
from copy import deepcopy
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import sys
import traceback

HERE = Path(__file__).resolve().parent
CLBG = HERE.parents[1]
COHORT = CLBG / 'gleam-comparison/20261012'
LOCK = Path('/tmp/mech-clbg-timing.lock')
IMPORTER = CLBG / 'overview/explorer/native_languages_data_v1.py'
IMPORTER_SHA = '27779d09a6cfea98d0377c58d4cdd459fc7f59aac46efdb8f61144070a871dc7'
SEAL = COHORT / 'source-counts/semantic-exact20-v2'
COUNTS = SEAL / 'qualified-loc-overlay.json'
COUNTS_SHA = 'dd51c9a33d0eade6b5f4f94266ad1561b0f36770ac03a9c1691d2f19cbc04fce'
RESOLUTION_SHA = 'c4b65664696b26539d4ce1f32e7494c80c2e706786ffff482df18116aca72505'
SEAL_OUTCOME = COHORT / 'source-counts/semantic-seal-outcomes/semantic-exact20-v2/outcome.json'
SEAL_OUTCOME_SHA = '37e762c2cd3072c9aa9b8caca6796e96afe4708cd24464567592fcc23bf06e02'
SEAL_JOB = CLBG / 'continuation/owned-jobs/gleam-semantic-exact20-loc-v2'
SEAL_TERMINAL_SHA = '58c082f42422a6a9aaca306cdfb6eb8fb49f57e9b222d391c856938980ecc82d'
LAUNCHER = '/opt/homebrew/opt/python@3.14/bin/python3.14'
FRAMEWORK = '/opt/homebrew/Cellar/python@3.14/3.14.8/Frameworks/Python.framework/Versions/3.14/Resources/Python.app/Contents/MacOS/Python'
HOST = CLBG / 'continuation/jobs/python-framework-host-linkage-v1/host-linkage.json'
HOST_SHA = '9a1c0fa4fc330cf0c7c176349c220640ad4714c5e8e081eaead84c00b89729af'
ROOT_REVIEW = CLBG / 'continuation/jobs/gleam-semantic-exact20-v2-closed-review/receipt.json'
SUITE = COHORT / 'reconciliation/full-v4-suite-v2.json'
SUITE_SHA = 'e2b4a772153c9997d4ab5f749dfa950cbcf4ebaad825201b3c6338454f4e2594'
PREPARATION = COHORT / 'qualification/private-v5/report.json'
PREPARATION_SHA = 'a19f2d079b917f1172b54d09e494e72c5a7aea9d6a9301ef0974633b4d4f30dc'
INDEX = COHORT / 'studies/full-v4/index.json'
INDEX_SHA = 'd5eff42c8f88733af92a8e597b3e02c2b3e8212b24b827b17a494cfabd54497b'
COMPILER = COHORT / '.build/private-v1/compiler/gleam'
OTP = Path('/opt/homebrew/Cellar/erlang/29.1.1_2')
ERL = OTP / 'bin/erl'
ENGINE = OTP / 'lib/erlang/erts-17.1/bin/beam.smp'
ERLEXEC = ENGINE.with_name('erlexec')
FLAGS = ['+S','8:8','+A','1','+SDcpu','1:1','+SDio','1']
BENCHMARKS = {'binarytrees','fannkuchredux','nbody','spectralnorm','mandelbrot','pidigits','fasta','revcomp','knucleotide','regexredux'}
IDS = {'gleam-'+name+'-'+variant for name in BENCHMARKS for variant in ('compact','optimized')}


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream,'sha256').hexdigest()


def save(path,value):
    path.parent.mkdir(parents=True,exist_ok=True)
    with path.open('x') as stream:
        json.dump(value,stream,indent=2); stream.write('\n')


def merge(pins,values):
    for path,digest in values.items():
        assert type(path) is str and Path(path).is_absolute() and re.fullmatch('[0-9a-f]{64}',digest)
        assert path not in pins or pins[path] == digest, ('Conflicting actual evidence',path)
        pins[path] = digest


def verify(pins):
    for path,digest in pins.items():
        assert sha(path) == digest, ('Actual evidence changed',path)


def pinned_json(path,pins):
    path = Path(path)
    assert str(path) in pins and sha(path) == pins[str(path)], ('Unbound actual receipt',str(path))
    return json.loads(path.read_text())


def pinned_text(path,pins):
    path = Path(path)
    assert str(path) in pins and sha(path) == pins[str(path)], ('Unbound actual output',str(path))
    return path.read_text()


def owned_receipt(path,command,pins):
    value = pinned_json(path,pins)
    assert value['command'] == command
    assert value['inherited_common_mutex'] is value['preparation_only'] is value['never_chart_eligible'] is True
    assert type(value['exit_status']) is type(value['final_exit_status']) is int
    assert value['exit_status'] == value['final_exit_status'] == 0
    assert not any(value.get(key) for key in ('error','cleanup_error','cleanup_errors',
        'remaining_group_members','output_drain_error'))
    return value



def bind(path, digest, pins):
    path = Path(path).resolve(strict=True)
    merge(pins,{str(path):digest}); verify({str(path):digest})
    return path


def closed_seal(args,pins):
    bind(ROOT_REVIEW,args.root_review_sha256,pins); review = pinned_json(ROOT_REVIEW,pins)
    assert review['status'] == 'passed-closed-Gleam-exact20-source-overlay-review' and review['verified_pin_count'] == 6896
    assert review['qualified_overlay'] == dict(path=str(COUNTS),sha256=COUNTS_SHA)
    assert review['resolution_sha256'] == RESOLUTION_SHA and review['outcome_sha256'] == SEAL_OUTCOME_SHA
    assert review['durable_terminal_sha256'] == SEAL_TERMINAL_SHA and set(review['profiles']) == IDS
    assert all(review[key] == 0 for key in ('benchmark_runs','native_processes','fixture_reexecutions','source_parser_executions')) and review['chart_admission'] is False
    records = review['exact_process_records']; assert set(records) == {'request','launch','worker-started','command-started','terminal'}
    for name,item in records.items():
        assert item['path'] == str(SEAL_JOB/(name+'.json')); bind(item['path'],item['sha256'],pins)
    request,launch,worker,started,terminal = (pinned_json(SEAL_JOB/(name+'.json'),pins) for name in ('request','launch','worker-started','command-started','terminal'))
    assert records['terminal']['sha256'] == SEAL_TERMINAL_SHA
    assert request['command'] == launch['command'] == worker['command'] == started['command']
    command = ['/opt/homebrew/opt/python@3.14/bin/python3.14','-B','-u',str(COHORT/'source-counts/seal_semantic_overlay_v3.py'),
        '--base-sealer-sha256','0e10d449388ab5fee11909f7098faddd34ee5ef89f4097dc4e4e167bf86d9051',
        '--facts-sha256','437f7883efd55044bb4a03d012354f4981033e4954787f265242f220ecdf63cf',
        '--peer-review',str(COHORT/'source-counts/peer-source-review-v1.json'),'--peer-sha256','c088f69a0eeef793445cc7efc83cad003c5a3d45acb45a327ecc9964ba49e1b3',
        '--closed-job-pins',str(COHORT/'source-counts/closed-original-jobs-v1.json'),'--closed-job-pins-sha256','a5ca760b6a4cc131492aa5f9ed437ba54319b32b3e64766c3040bced5f2d6b36',
        '--revision','semantic-exact20-v2']
    assert request['command'] == command and request['cwd'] == launch['cwd'] == worker['cwd'] == started['cwd'] == str(CLBG.parents[1])
    assert request['automatic_retry'] is request['qualification_granted'] is terminal['automatic_retry'] is terminal['qualification_granted'] is False
    assert request['common_mutex_owned_by_target_command'] is worker['common_mutex_owned_by_target_command'] is True
    assert worker['inherited_private_lease'] is worker['detached_session'] is True
    assert terminal['status'] == 'command-exited-zero-awaiting-report-review' and type(terminal['exit_code']) is int and terminal['exit_code'] == 0
    assert terminal['actual_command_started'] is True and terminal['stop_signals'] == []
    assert terminal['command_identity_observation_missing'] is started['command_identity_observation_missing'] is False
    assert not any(terminal.get(key) for key in ('error','error_type','exception_cleanup','error_log_write_failure'))
    assert terminal['worker_identity'] == worker['worker_identity'] and terminal['command_identity'] == started['command_identity']
    assert terminal['command_pid'] == started['command_pid'] == terminal['command_identity']['pid']
    initial,final = (item['worker_identity']['listing'].split(None,8) for item in (launch,terminal))
    assert len(initial) == len(final) == 9 and initial[0] == final[0] == '30357' and initial[2:7] == final[2:7]
    assert initial[7] == LAUNCHER and final[7] == FRAMEWORK
    rest = '-B -u '+str(CLBG/'continuation/launch_durable_v1.py')+' --worker '+str(SEAL_JOB)+' --lease-fd 3 --go-fd 4'
    assert initial[8] == final[8] == rest and initial[1].isdigit() and final[1] == '1'
    image = terminal['command_identity']['listing'].split(None,8)
    assert len(image) == 9 and image[0] == str(terminal['command_pid']) == '30363' and image[1] == '30357' and image[2:7] == final[2:7] and image[7] == command[0] and image[8] == ' '.join(command[1:])
    bind(SEAL_OUTCOME,SEAL_OUTCOME_SHA,pins); outcome = pinned_json(SEAL_OUTCOME,pins)
    assert outcome['status'] == 'completed-overlay-stage' and outcome['body_error'] is None and outcome['integrity_errors'] == []
    assert outcome['actual_common_mutex_held'] is True and outcome['original_sources_or_timings_mutated'] is outcome['chart_admission'] is False
    assert all(outcome[key] == 0 for key in ('actual_native_subjects','actual_signals_sent','fixture_reexecutions'))
    merge(pins,outcome['pins']); verify(pins)
    assert pins[str(HOST)] == HOST_SHA
    host = pinned_json(HOST,pins)
    assert host['status'] == 'root-reviewed-exact-python-framework-linkage' and host['framework']['path'] == FRAMEWORK
    assert host['same_image_bytes_claimed'] is host['benchmark_runtime_image_changed'] is False
    assert pins[host['launcher']['resolved_path']] == host['launcher']['sha256'] and pins[FRAMEWORK] == host['framework']['sha256']
    return review


def image_proof(image,folder,pins):
    owned_receipt(folder/'architecture.json',['/usr/bin/lipo','-archs',str(image)],pins)
    assert pinned_text(folder/'architecture.stdout',pins).split() == ['arm64']
    owned_receipt(folder/'linked-images.json',['/usr/bin/otool','-L',str(image)],pins)
    review = pinned_json(folder/'dependency-review.json',pins)
    assert review['binary'] == str(image) and review['architectures'] == ['arm64']
    assert [item['install_name'] for item in review['dependencies']] == [line.strip().split(' (',1)[0] for line in pinned_text(folder/'linked-images.stdout',pins).splitlines()[1:]]
    for item in review['dependencies']:
        if 'path' in item: assert item['sha256'] == pins[item['path']] == sha(item['path'])
        else: assert item['scope'] == 'OS-image-not-hashed' and item['install_name'].startswith(('/usr/lib/','/System/Library/'))
    return dict(binary=str(image),sha256=pins[str(image)],architecture_receipt=str(folder/'architecture.json'),dependency_review=str(folder/'dependency-review.json'))


def qualify(args,output,pins):
    for path,digest in ((Path(__file__).resolve(),args.qualifier_sha256),(HERE/'descriptor.json',args.descriptor_sha256),(HERE/'REVIEW.md',args.review_sha256),(IMPORTER,IMPORTER_SHA),
            (COUNTS,COUNTS_SHA),(SUITE,SUITE_SHA),(PREPARATION,PREPARATION_SHA),(INDEX,INDEX_SHA),(SEAL/'source-meaning-resolution.json',RESOLUTION_SHA)):
        bind(path,digest,pins)
    descriptor = pinned_json(HERE/'descriptor.json',pins)
    assert descriptor['status'] == 'source-reviewed-descriptor-awaiting-guarded-actual-evidence-qualification' and descriptor['language'] == 'gleam' and descriptor['execution'] == 'JIT'
    assert descriptor['actual_compiler_version_verified'] is descriptor['actual_native_architecture_verified'] is descriptor['actual_memory_policy_verified'] is False
    root_review = closed_seal(args,pins)
    suite,counts,preparation,index = (pinned_json(path,pins) for path in (SUITE,COUNTS,PREPARATION,INDEX))
    merge(pins,suite['provenance']); merge(pins,counts['pins']); merge(pins,preparation['pins']); verify(pins)
    assert suite['canonical_full_native_processes'] == 90 and suite['candidate_sources'] == 20 and suite['actual_samples_per_source'] == 3
    assert suite['source_timing_relabels'] == suite['source_or_report_mutations'] == 0 and suite['chart_admitted'] is False
    assert counts['status'] == 'qualified-exact-source-LOC' and counts['profile_count'] == 20 and counts['canonical_LOC_admitted'] is True and counts['chart_admission'] is False
    assert index['status'] == 'sealed-awaiting-root-review' and index['samples'] == 3 and set(index['benchmarks']) == BENCHMARKS
    assert suite['provenance'][str(INDEX)] == INDEX_SHA
    bootstrap = COHORT/'qualification/private-v1/bootstrap'
    owned_receipt(bootstrap/'gleam-version.json',[str(COMPILER),'--version'],pins)
    assert pinned_text(bootstrap/'gleam-version.stdout',pins) == 'gleam 1.19.0' + chr(10) and pinned_text(bootstrap/'gleam-version.stderr',pins) == ''
    expression = ('io:put_chars(json:encode(#{otp=>list_to_binary(erlang:system_info(otp_release)), '
        'erts=>list_to_binary(erlang:system_info(version)), jit=>erlang:system_info(emu_flavor), '
        'arch=>list_to_binary(erlang:system_info(system_architecture)), '
        'schedulers=>erlang:system_info(schedulers_online), re=>re:version()})),halt().')
    owned_receipt(bootstrap/'otp-version.json',[str(ERL),*FLAGS,'-noshell','-noinput','-eval',expression],pins)
    toolchain = json.loads(pinned_text(bootstrap/'otp-version.stdout',pins))
    assert toolchain == dict(re='10.47 2025-10-21',otp='29',schedulers=8,erts='17.1',jit='jit',arch='aarch64-apple-darwin25.6.0')
    assert pinned_text(bootstrap/'otp-version.stderr',pins) == '' and preparation['actual_toolchain'] == toolchain and preparation['compiler'] == str(COMPILER)
    images = {name:image_proof(image,bootstrap/name,pins) for name,image in (('gleam',COMPILER),('beam',ENGINE),('erlexec',ERLEXEC))}
    profiles = {}; original = {row['id']:row for row in preparation['profiles']}
    native = {row['id']:row for bench in suite['benchmarks'].values() for row in bench['profiles'].values() if row['language'] == 'gleam'}
    assert len(original) == len(native) == 20 and set(original) == set(native) == set(counts['profiles']) == IDS
    for identity,row in native.items():
        prior = original[identity]; invref = counts['compiler_bound_input_inventory'][identity]
        wrapper = pinned_json(invref['evidence_path'],pins)
        assert pins[invref['evidence_path']] == invref['evidence_sha256']
        assert wrapper['status'] == invref['status'] == 'qualified-exact-authored-compiler-inputs' and wrapper['profile'] == identity
        assert wrapper['source_sha256'] == invref['source_sha256'] == row['source_sha256'] and wrapper['binary_sha256'] == invref['binary_sha256'] == row['binary_sha256']
        bound = wrapper['original_compiler_inventory']; assert bound['path'] == row['compiled_input_inventory'] == prior['compiled_input_inventory']
        assert bound['sha256'] == row['compiled_input_inventory_sha256'] == prior['compiled_input_inventory_sha256'] == pins[bound['path']]
        inventory = pinned_json(bound['path'],pins); assert inventory == wrapper['exact_original_inventory']
        assert inventory['status'] == 'compiled-exact-authored-input-inventory' and inventory['profile'] == identity
        for key in ('source','source_sha256','source_bundle','compiled_artifacts','runtime_classpath'):
            assert inventory[key] == row[key] == prior[key]
        assert wrapper['files'] == invref['files'] == [dict(path=item['path'],sha256=item['sha256']) for item in row['source_bundle']]
        assert bound['source_bundle'] == inventory['source_bundle'] and bound['compiled_artifacts'] == inventory['compiled_artifacts'] and bound['runtime_classpath'] == inventory['runtime_classpath']
        package = Path(row['source']).parent.parent; application = Path(row['runtime_classpath'][0]).parent
        assert inventory['compiler'] == str(COMPILER) and inventory['compiler_sha256'] == pins[str(COMPILER)] == sha(COMPILER)
        build_command = [str(COMPILER),'compile-package','--target','erlang','--no-dev','--package',str(package),'--out',str(application),'--lib',str(COHORT/'.build/private-v1/libraries')]
        assert inventory['build_command'] == build_command
        build = Path(bound['path']).with_name('compile.json'); owned_receipt(build,build_command,pins)
        assert row['command'] == prior['command'] == [str(ERL),*FLAGS,'-noshell','-noinput','-pa',*inventory['runtime_classpath'],'-s','clbg_runner','main','-extra','{n}']
        assert row['binary'] == str(ERL) and row['binary_sha256'] == prior['binary_sha256'] == pins[str(ERL)] == sha(ERL)
        assert row['compiler_version'] == '1.19.0' and row['runtime_version'] == '29.1.1' and row['backend'] == 'BEAM load-time JIT' and row['measured_architecture'] == 'arm64'
        for path,digest in row['compiled_artifacts'].items(): assert pins[path] == digest
        qualified = prior['runtime_environment']; crash = str(COHORT/'studies/full-v4'/row['benchmark']/'runtime-output'/identity/'erl_crash.dump')
        assert row['qualified_runtime_environment'] == qualified and row['runtime_environment_overrides'] == {'ERL_CRASH_DUMP':{'from':qualified['ERL_CRASH_DUMP'],'to':crash}}
        assert row['runtime_environment'] == dict(qualified,ERL_CRASH_DUMP=crash)
        assert set(qualified) == {'PATH','LC_ALL','ERL_FLAGS','ERL_CRASH_DUMP','OMP_NUM_THREADS','OPENBLAS_NUM_THREADS','VECLIB_MAXIMUM_THREADS'}
        assert qualified['PATH'] == str(OTP/'bin')+':/usr/bin:/bin:/usr/sbin:/sbin' and qualified['LC_ALL'] == 'C' and qualified['ERL_FLAGS'] == ' '.join(FLAGS)
        assert all(qualified[key] == '1' for key in ('OMP_NUM_THREADS','OPENBLAS_NUM_THREADS','VECLIB_MAXIMUM_THREADS'))
        assert wrapper['inventory_derived_from_original_actual_compile_receipt'] is True
        for item in row['source_bundle']:
            assert pins[item['path']] == item['sha256'] and item['role'] in ('application','application-helper')
            text = pinned_text(item['path'],pins)
            assert not any(token in text for token in ('process_flag','system_flag','spawn_opt','disable_gc','gc_disable'))
        metric = counts['profiles'][identity]; expected = root_review['profiles'][identity]
        assert metric['canonical_loc'] == expected['canonical_loc'] and metric['physical_loc'] == expected['physical_loc'] and len(metric['source_bundle']) == expected['authored_inputs']
        profiles[identity] = dict(source_sha256=row['source_sha256'],binary_sha256=row['binary_sha256'],qualified_compiler_input_wrapper=invref,
            original_compiler_inventory=bound,original_build_receipt=str(build),runtime_command=row['command'],runtime_environment=row['runtime_environment'],
            original_qualified_environment=qualified,explicit_crash_output_relocation=row['runtime_environment_overrides'],compiled_artifacts=row['compiled_artifacts'],runtime_classpath=row['runtime_classpath'],
            memory_policy='Default BEAM process-heap garbage collection; no authored application GC override or heap-policy setting',memory_policy_basis='Actual pinned OTP/ERTS identity, exact saved flags/environment and source; version-matched official ERTS documentation',
            collector_probe_executed=False,original_full_samples_reused=3)
    verify(pins)
    runtime = output/'toolchain-runtime-receipt.json'
    save(runtime,dict(status='verified-actual-Gleam-BEAM-JIT-compiler-runtime-and-default-process-GC',backend='BEAM load-time JIT',
        actual_toolchain=toolchain,compiler_version='1.19.0',native_architecture='arm64',original_bootstrap=str(bootstrap),native_images=images,profiles=profiles,
        memory_strategies=['gc'],collector_probe_executed=False,new_native_processes=0,source_counts_unchanged=True,pins=deepcopy(pins)))
    merge(pins,{str(runtime):sha(runtime)})
    metadata = deepcopy(descriptor); metadata.update(status='reviewed-actual-native-language-metadata',actual_compiler_version_verified=True,
        actual_native_architecture_verified=True,actual_memory_policy_verified=True,memory_strategies=['gc'],pins=deepcopy(pins))
    metadata['measured_backend_template']['verified'] = True
    metadata['measured_backend_template']['runtime_receipt'].update(verified=True,report=str(runtime),report_sha256=pins[str(runtime)])
    name = 'gleam_publication_exact_cohort_validator'; assert name not in sys.modules
    sys.dont_write_bytecode = True
    spec = importlib.util.spec_from_file_location(name,IMPORTER); validator = importlib.util.module_from_spec(spec);sys.modules[name] = validator
    importer_source = IMPORTER.read_bytes(); assert hashlib.sha256(importer_source).hexdigest() == IMPORTER_SHA
    exec(compile(importer_source,str(IMPORTER),'exec'),validator.__dict__)
    assert Path(validator.__file__).resolve() == IMPORTER and sha(IMPORTER) == IMPORTER_SHA
    assert len(validator.validate_cohort('gleam',suite,counts,metadata)) == 20
    verify(pins); save(output/'metadata.json',metadata)


def main():
    parser = argparse.ArgumentParser()
    for name in ('revision','qualifier-sha256','descriptor-sha256','review-sha256','root-review-sha256'):parser.add_argument('--'+name,required=True)
    args = parser.parse_args()
    assert re.fullmatch('[a-z0-9][a-z0-9-]{0,63}',args.revision) and all(re.fullmatch('[a-f0-9]{64}',value) for key,value in vars(args).items() if key.endswith('_sha256'))
    for path in (Path(__file__).resolve(),HERE/'descriptor.json',HERE/'REVIEW.md'):assert not path.stat().st_mode & 0o222
    with (SEAL_JOB/'owner.lock').open('rb') as lease:
        fcntl.flock(lease,fcntl.LOCK_SH|fcntl.LOCK_NB); opened,named = os.fstat(lease.fileno()),Path(lease.name).stat();assert (opened.st_dev,opened.st_ino) == (named.st_dev,named.st_ino)
        print('WAIT common mutex: Gleam actual publication metadata only',flush=True)
        with LOCK.open('a+') as common:
            fcntl.flock(common,fcntl.LOCK_EX);opened,named = os.fstat(common.fileno()),LOCK.stat();assert (opened.st_dev,opened.st_ino) == (named.st_dev,named.st_ino) and opened.st_ino == 79859134
            output = HERE/'qualified'/args.revision;output.mkdir(parents=True,exist_ok=False)
            pins = {}; outcome = dict(status='failed-metadata',chart_admission=False,native_subjects=0,fixture_reexecutions=0)
            try:
                bind(Path(sys.executable).resolve(),sha(Path(sys.executable).resolve()),pins)
                qualify(args,output,pins);outcome['status'] = 'metadata-qualified-awaiting-root-current-candidate-review'
            except BaseException as error:
                outcome.update(error=str(error) or type(error).__name__,error_type=type(error).__name__)
                with (output/'actual-error.log').open('x') as stream:stream.write(traceback.format_exc())
                raise
            finally:
                available,failures = {},[]
                for path,digest in list(pins.items()):
                    try:verify({path:digest})
                    except BaseException as error:failures.append(dict(path=path,error=str(error) or type(error).__name__,error_type=type(error).__name__))
                for path in output.rglob('*'):
                    if path.is_file():
                        try:available[str(path)] = sha(path)
                        except BaseException as error:failures.append(dict(path=str(path),error=str(error) or type(error).__name__,error_type=type(error).__name__))
                if failures:outcome.update(status='failed-terminal-integrity',integrity_errors=failures,available_bytes_not_fully_pinned=True)
                outcome.update(prerequisite_pins=pins,available_evidence_pins=available,actual_common_mutex_held=True)
                save(output/'outcome.json',outcome)
                if failures:raise RuntimeError('Gleam metadata terminal integrity failure')
            print(json.dumps(dict(status=outcome['status'],metadata=str(output/'metadata.json'),sha256=sha(output/'metadata.json'),profiles=20,native_processes=0,fixture_reexecutions=0)),flush=True)


if __name__ == '__main__':main()
