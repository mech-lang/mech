"""One static-I/O official safe-trim build; no workload or AOT admission."""
import argparse
from datetime import datetime, timezone
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import shutil
import signal
import sys
import tomllib
import traceback

P = Path(__file__).resolve().parent
HERE = P.parents[1]
CLBG = HERE.parents[1]
LOCK = Path('/tmp/mech-clbg-timing.lock')
PILOT = HERE/'jobs/nbody-library-pilot-v1/report.json'
BOOTSTRAP = HERE/'jobs/packagecompiler-bootstrap-v2/report.json'
BOOTSTRAP_SHA = '0756b1108e0cbba480285c7e2ac0a4edfa61a78056bbe4bec0b429909365386c'
CALIBRATION = CLBG/'continuation/jobs/julia-calibration-v2-closed-review-v1/review.json'
CALIBRATION_SHA = '5cdb703a95dd53c4fd7e5467287c993beb3b2d18b19e614d1e0a6889d1e43d73'
ORIGINAL = CLBG/'nbody/upstream/nbody-julia-5.jl'
ORIGINAL_SHA = '2def3e7e9910b46cd522b81041e04bbcede2481f69684b683bea605bef598a54'
OLD_BUILD = HERE/'.build/nbody-library-pilot-v1'
ADAPTATION = HERE/'jobs/nbody-library-pilot-v1/source-adaptation.json'
ADAPTATION_SHA = '7edf4e1baf8533c4cf3a1e88aa86b880f3691229a27cb2b9b4c29cf33a3df0f4'
RUNTIME = HERE/'.build/packagecompiler-bootstrap-v2/restored-runtime/julia-1.13.1'
TRIMMING = RUNTIME/'share/julia/test/trimming'
PREPARATION = CLBG/'c3-comparison/20261012/revisions/private-v3/guarded_prepare.py'
PREPARATION_SHA = 'f1290ab11e8a77ea1381f5cfab2033e2b709b2b2b9d419ddf2fa8a21410e04fd'
FAILED = HERE/'jobs/official-safe-trim-nbody-v1'
FAILED_JOB = CLBG/'continuation/owned-jobs/julia-official-safe-trim-nbody-v1'
FAILED_ROOT = CLBG/'continuation/jobs/julia-safe-trim-build-closed-review-v1/receipt.json'
FAILED_ROOT_SHA = '929d8dc37de11c1dd5887dad8399e10cde5a3b90cabd86b85ec8be17a39c9666'
FAILED_OUTCOME_SHA = '42c319719f806306c73a89192dc1638f4810c28de9367eb794c14f998b5d5634'
FAILED_TERMINAL_SHA = '63687cd23db5886735d4c2fb8b6238571e7c7544b87583773b5b1856c844fe7a'
FAILED_COMPILE_SHA = '4945703547e5f9fc35290f9bc9da1cb176e6e297e7e3bd3f64707a36290a7ef3'
SHIPPED = {
    'juliac-buildscript.jl':'392b91e0810f7d46b019afd7c481381b90156833cb1d27df0e05467d0fba64df',
    'juliac-trim-base.jl':'3dd7c0621df19510d7eab8b88b96f5ec32d9cd93d4aa4019ca908666478e4967',
    'juliac-trim-stdlib.jl':'3740d83feed070e9f00f5ffbecdbaea2e2dd6a1c71ce23fa33e9c47e15b0b440',
    'abi_export.jl':'4527857e6c7846eff54ffe8ac3406bb843c525c584c68323eeb9c92d7e038128',
    'Makefile':'26c1f1f1bec91e90c489f34ffb00cb59ce3e38d341c3c2b86b8eca8639eacb7a',
}


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream,'sha256').hexdigest()


def save(path,value):
    with path.open('x') as stream:
        json.dump(value,stream,indent=2);stream.write('\n')


def merge(pins,other):
    for path,digest in other.items():
        assert path not in pins or pins[path] == digest, ('Conflicting source/build pin',path)
        pins[path] = digest


def bind(path,digest,pins):
    path = Path(path).resolve(strict=True)
    assert re.fullmatch('[a-f0-9]{64}',digest) and sha(path) == digest, ('Exact reviewed input changed',path)
    merge(pins,{str(path):digest});return path


def verify(pins):
    for path,digest in pins.items():
        assert sha(path) == digest, ('Retained input/output changed',path)


def cold(path,digest,name,pins):
    bind(path,digest,pins);assert name not in sys.modules
    source = path.read_bytes();assert hashlib.sha256(source).hexdigest() == digest
    spec = importlib.util.spec_from_file_location(name,path)
    module = importlib.util.module_from_spec(spec);sys.modules[name] = module
    exec(compile(source,str(path),'exec'),module.__dict__)
    return module


def entry_abi(path):
    abi = json.loads(path.read_text())
    exported = [row for row in abi['functions'] if row['symbol'] == 'clbg_entry']
    assert len(exported) == 1 and len(exported[0]['arguments']) == 2
    types = {row['id']:row for row in abi['types']};assert len(types) == len(abi['types'])
    row = exported[0]
    argc = types[row['arguments'][0]['type_id']];result = types[row['returns']['type_id']]
    assert argc == result and argc['kind'] == 'primitive' and argc['name'] == 'Int32'
    assert argc['signed'] is True and argc['bits'] == 32 and argc['size'] == 4
    argv = types[row['arguments'][1]['type_id']];assert argv['kind'] == 'pointer'
    charptr = types[argv['pointee_type_id']];assert charptr['kind'] == 'pointer'
    char = types[charptr['pointee_type_id']]
    assert char['kind'] == 'primitive' and char['name'] == 'Int8' and char['bits'] == 8 and char['signed'] is True
    return dict(function=row,argc=argc,argv=argv,char_pointer=charptr,char=char,
        all_exported_symbols=[item['symbol'] for item in abi['functions']],native_owner_qualified=False)



def retained_failure(pins):
    root = json.loads(bind(FAILED_ROOT,FAILED_ROOT_SHA,pins).read_text())
    assert root['status'] == 'closed-safe-trim-rejection-of-dynamic-output-error-wrapper' and root['verified_pins'] == 10038
    assert root['outcome_sha256'] == FAILED_OUTCOME_SHA and root['durable_terminal_sha256'] == FAILED_TERMINAL_SHA
    assert root['native_compile_receipt'] == dict(path=str(FAILED/'compile/official-safe-trim.json'),sha256=FAILED_COMPILE_SHA)
    assert root['native_compile_exit'] == 1 and root['trim_verifier_errors'] == 6 and root['trim_verifier_warnings'] == 0
    assert root['unresolved_calls'] == ['showerror(stderr::IO, exception::Any)','println(stderr::IO)','print(stdout::IO, String, newline)']
    assert root['actual_workload_executions'] == root['benchmark_runs'] == root['LLDB_attempts'] == 0 and root['runtime_aot_proven'] is False
    terminal = json.loads(bind(FAILED_JOB/'terminal.json',FAILED_TERMINAL_SHA,pins).read_text())
    request,started,worker = [json.loads(bind(FAILED_JOB/name,sha(FAILED_JOB/name),pins).read_text()) for name in ('request.json','command-started.json','worker-started.json')]
    command = ['/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3.12','-I','-S','-B','-u',
        str(HERE/'revisions/safe-trim-build-v1/build.py'),'--revision','official-safe-trim-nbody-v1',
        '--worker-sha256','489441d9f35a40c5016b3f58ee5c49a8eeff9980529bdca99f6b705bac0af3d5',
        '--options-sha256','44d47002ac19bafde6b6748d4e4cec01219a4ec66539fe450cb3772527ccbfc4',
        '--review-sha256','cfc3d0e142b534c4cfb3fcdee7a772ca6fa64ad550eeaf4f86cd68073482de93',
        '--pilot-sha256','39d2d809b45780918c6aa11e6e3d7f9f41218fd940b12d29297244f734259e95']
    observed = terminal['command_identity']['listing'].split(None,7)
    assert terminal['status'] == 'command-failed' and type(terminal['exit_code']) is int and terminal['exit_code'] == 1
    assert terminal['actual_command_started'] is True and terminal['command_identity_observation_missing'] is False and terminal['stop_signals'] == []
    assert terminal['qualification_granted'] is terminal['automatic_retry'] is False and not any(terminal.get(key) for key in ('error','exception_cleanup','error_log_write_failure'))
    assert request['command'] == started['command'] == worker['command'] == command and request['cwd'] == started['cwd'] == worker['cwd'] == str(CLBG.parent.parent)
    assert request['common_mutex_owned_by_target_command'] is True and request['qualification_granted'] is request['automatic_retry'] is False
    assert started['command_identity'] == terminal['command_identity'] and started['command_pid'] == terminal['command_pid'] == 49624
    assert started['command_identity_observation_missing'] is False and worker['worker_identity'] == terminal['worker_identity']
    assert terminal['worker_identity']['pid'] == 49618 and observed[:2] == ['49624','49618'] and observed[7].split() == command
    outcome = json.loads(bind(FAILED/'outcome.json',FAILED_OUTCOME_SHA,pins).read_text())
    assert outcome['status'] == 'failed-safe-trim-build' and outcome['runtime_aot_proven'] is outcome['chart_admission'] is False
    assert outcome['actual_workload_executions'] == outcome['actual_benchmark_runs'] == outcome['LLDB_attempts'] == 0
    assert outcome['exclusive_common_mutex_held'] is True and outcome['common_mutex_inode'] == 79859134 and not outcome.get('integrity_errors')
    for mapping in (outcome['prerequisite_pins'],outcome['available_evidence_pins']):
        for path,digest in mapping.items():bind(path,digest,pins)
    receipt = json.loads(bind(FAILED/'compile/official-safe-trim.json',FAILED_COMPILE_SHA,pins).read_text())
    request_body = json.loads((FAILED/'source-and-build-request.json').read_text())
    assert receipt['command'] == request_body['command'] and receipt['pid'] == receipt['identity']['pid'] == 49668
    assert receipt['exit_status'] == receipt['final_exit_status'] == 1 and receipt['inherited_common_mutex'] is True and receipt['timeout_seconds'] == 1800
    assert not receipt.get('cleanup_errors') and not (FAILED/'safe-build-result.json').exists()
    text = (FAILED/'compile/official-safe-trim.stdout').read_text()
    assert text.count('Verifier error #') == 6 and 'Trim verify finished with 6 errors, 0 warnings.' in text


def edit_spans(original,replacements):
    edits = []
    for old,new,count in replacements:
        assert original.count(old) == count, ('Exact declared I/O span changed',old)
        offset = 0
        for _ in range(count):
            start = original.index(old,offset);end = start+len(old)
            edits.append(dict(start=start,end=end,original=old,replacement=new));offset=end
    edits.sort(key=lambda item:item['start']);cursor=0;parts=[];untouched=[]
    for item in edits:
        assert cursor <= item['start'] and original[item['start']:item['end']] == item['original']
        text = original[cursor:item['start']];parts.extend((text,item['replacement']))
        untouched.append(dict(start=cursor,end=item['start'],text=text));cursor=item['end']
    parts.append(original[cursor:]);untouched.append(dict(start=cursor,end=len(original),text=original[cursor:]))
    return ''.join(parts),edits,untouched


def build(args,output,buildroot,pins,lock_fd):
    bind(Path(__file__).resolve(),args.worker_sha256,pins)
    bind(P/'build_options.jl',args.options_sha256,pins);bind(P/'REVIEW.md',args.review_sha256,pins)
    bind(P/'entry_static.jl',args.entry_sha256,pins);retained_failure(pins)
    pilot = json.loads(bind(PILOT,args.pilot_sha256,pins).read_text())
    assert pilot['status'] == 'nbody-library-built-awaiting-state-native-coverage-and-runtime-qualification'
    assert pilot['pins_unchanged_at_end'] is True and pilot['actual_benchmark_runs'] == pilot['actual_workload_executions'] == 0
    assert pilot['runtime_aot_proven'] is pilot['chart_admission'] is False
    merge(pins,pilot['pins']);verify(pins)
    assert pilot['bootstrap'] == dict(path=str(BOOTSTRAP),sha256=BOOTSTRAP_SHA)
    bootstrap = json.loads(bind(BOOTSTRAP,BOOTSTRAP_SHA,pins).read_text())
    assert bootstrap['status'] == 'exact-packagecompiler-bootstrap-complete-awaiting-library-pilot'
    assert bootstrap['julia_version'] == pilot['julia_version'] == 'julia version 1.13.1'
    assert bootstrap['runtime_restoration']['runtime'] == str(RUNTIME) and bootstrap['pins_unchanged_at_end'] is True
    merge(pins,bootstrap['pins']);verify(pins)
    calibration = json.loads(bind(CALIBRATION,CALIBRATION_SHA,pins).read_text())
    assert calibration['status'] == 'closed-two-attach-timeouts-reviewed-no-native-retry'
    assert calibration['private_lease_closed'] is True and calibration['durable_exit_code'] == 0
    assert calibration['native_helper_executions'] == calibration['new_native_commands'] == 0
    assert calibration['common_mutex_inode'] == 79859134 and not calibration['stop_signals']
    assert len(calibration['cells']) == 2 and all(cell['native_target_executions'] == 0 and
        cell['original_cleanup']['no_remaining_parent_owned_native'] is True for cell in calibration['cells'])
    for name,digest in SHIPPED.items():
        assert pilot['pins'][str(TRIMMING/name)] == bootstrap['pins'][str(TRIMMING/name)] == digest
        bind(TRIMMING/name,digest,pins)
    assert '#define JL_TRIM_SAFE 1' in bind(RUNTIME/'include/julia/julia.h',pilot['pins'][str(RUNTIME/'include/julia/julia.h')],pins).read_text()
    original = bind(ORIGINAL,ORIGINAL_SHA,pins).read_text()
    adaptation = json.loads(bind(ADAPTATION,ADAPTATION_SHA,pins).read_text())
    assert adaptation['original_source'] == str(ORIGINAL) and adaptation['original_sha256'] == ORIGINAL_SHA
    derived = adaptation['adapter'];assert len(derived['edits']) == 2 and derived['matched_derived_jit_required'] is True
    assert derived['edits'][0]['replacement'] == 'global bodies::Vector{Body}\n'
    for edit in derived['edits']:assert original[edit['start']:edit['end']] == edit['original']
    for span in adaptation['untouched_original_spans']:assert original[span['start']:span['end']] == span['text']
    assert not derived['appended_declarations'] and len(derived['runtime_initializers']) == 1
    assert derived['runtime_aot_proven'] is derived['chart_admission'] is False
    src = OLD_BUILD/'ClbgNbodyPilot/src'
    assert (src/'workload.jl').read_text() == derived['source']
    workload,workload_edits,workload_untouched = edit_spans(derived['source'],[
        ('println(Base.Ryu.writefixed(energy(bodies), 9))','println(Core.stdout, Base.Ryu.writefixed(energy(bodies), 9))',2)])
    old_entry = bind(src/'entry.jl',pilot['pins'][str(src/'entry.jl')],pins).read_text()
    entry,entry_edits,entry_untouched = edit_spans(old_entry,[
        ('    catch exception\n        showerror(stderr, exception)\n        println(stderr)\n',
         '    catch\n        println(Core.stderr, "nbody entry failed")\n',1)])
    assert (P/'entry_static.jl').read_text() == entry
    for path,expected,text in [
        (RUNTIME/'share/julia/base/boot.jl','1b52bf01e59e0a2b3d740a1db38012d65d804bf3686eee052165320abeff244b','const stdout = CoreSTDOUT()'),
        (RUNTIME/'share/julia/base/coreio.jl','575b5b8fbe9079200afd6893965e8c9a8a6d2e1a917dd963b0e3c2ce65beda48','stdout::IO = Core.stdout'),
        (TRIMMING/'hello.jl','5eb370d9ea0024d7fb17e27cda89653bb02e612b224f9466278b44f47bf50063','println(Core.stdout, "Hello, world!")')]:
        assert pins[str(path)] == expected and text in bind(path,expected,pins).read_text()
    snapshot = buildroot/'ClbgNbodyPilot';(snapshot/'src').mkdir(parents=True)
    copied = {}
    for source,target in [(src/'ClbgNbodyPilot.jl',snapshot/'src/ClbgNbodyPilot.jl'),(P/'entry_static.jl',snapshot/'src/entry.jl'),
        (OLD_BUILD/'ClbgNbodyPilot/Project.toml',snapshot/'Project.toml'),
        (OLD_BUILD/'direct_driver.c',buildroot/'direct_driver.c'),
        (OLD_BUILD/'matched_jit_driver.jl',buildroot/'matched_jit_driver.jl'),
        (P/'build_options.jl',buildroot/'build_options.jl')]:
        expected = pins[str(source)];bind(source,expected,pins);shutil.copyfile(source,target)
        assert sha(target) == expected;target.chmod(0o444);merge(pins,{str(target):expected})
        copied[str(source)] = dict(path=str(target),sha256=expected)
    workload_target = snapshot/'src/workload.jl'
    with workload_target.open('x') as stream:stream.write(workload)
    workload_target.chmod(0o444);merge(pins,{str(workload_target):sha(workload_target)})
    copied[str(src/'workload.jl')] = dict(path=str(workload_target),sha256=pins[str(workload_target)],declared_io_edits=workload_edits)
    save(output/'static-io-source-adaptation.json',dict(status='exact-declared-static-IO-only-source-derivation',
        previous_safe_trim_failure=dict(path=str(FAILED/'outcome.json'),sha256=FAILED_OUTCOME_SHA),
        workload=dict(original_path=str(src/'workload.jl'),original_sha256=pilot['pins'][str(src/'workload.jl')],
            path=str(workload_target),sha256=pins[str(workload_target)],edits=workload_edits,untouched_original_spans=workload_untouched),
        entry=dict(original_path=str(src/'entry.jl'),original_sha256=pilot['pins'][str(src/'entry.jl')],
            path=str(snapshot/'src/entry.jl'),sha256=args.entry_sha256,edits=entry_edits,untouched_original_spans=entry_untouched),
        shared_fresh_module_used_by_matched_derived_JIT=True,energy_formatting_and_kernel_unchanged=True,
        positive_output_before_and_after_all_steps=True,error_message_changed_to_static_literal=True,
        actual_workload_executions=0,source_LOC_admitted=False,chart_admission=False))
    julia = bind(RUNTIME/'bin/julia','81e48f60e1d484c6fb2ea64e7cd3986190ff0bb5d5125b66f51e8d793325e3a9',pins)
    sysimage = RUNTIME/'lib/julia/sys.dylib';bind(sysimage,pilot['pins'][str(sysimage)],pins)
    depot = buildroot/'depot';depot.mkdir();temporary = buildroot/'tmp';temporary.mkdir()
    environment = dict(bootstrap['runtime_environment'],JULIA_DEPOT_PATH=str(depot),TMPDIR=str(temporary)+os.sep,
        JULIA_IMAGE_THREADS='8',OMP_NUM_THREADS='1',OPENBLAS_NUM_THREADS='1',VECLIB_MAXIMUM_THREADS='1')
    object_file,bitcode,abi,options = [buildroot/name for name in ('nbody-safe-o.a','nbody-safe.bc','entry-abi.json','actual-build-options.toml')]
    command = [str(julia),'--startup-file=no','--history-file=no','--threads=8,0','--project='+str(snapshot),
        '-J',str(sysimage),'-O3','-g2','--output-incremental=no','--experimental','--trim=safe','--strip-ir',
        '--output-o='+str(object_file),'--output-bc='+str(bitcode),str(buildroot/'build_options.jl'),
        str(TRIMMING/'juliac-buildscript.jl'),str(snapshot/'src/ClbgNbodyPilot.jl'),'--output-lib','true',str(abi),str(options)]
    assert '--trim=unsafe' not in command and '--trim=unsafe-warn' not in command and '--strip-metadata' not in command
    save(output/'source-and-build-request.json',dict(command=command,cwd=str(HERE),environment=environment,
        original_source=str(ORIGINAL),original_sha256=ORIGINAL_SHA,original_adaptation=dict(path=str(ADAPTATION),sha256=ADAPTATION_SHA),
        compiled_source_inputs=copied,static_io_adaptation=str(output/'static-io-source-adaptation.json'),shipped_builder_sources={str(TRIMMING/name):digest for name,digest in SHIPPED.items()},
        workload_executions=0,linker_executions=0,LLDB_attempts=0,chart_admission=False))
    preparation = cold(PREPARATION,PREPARATION_SHA,'julia_static_io_safe_trim_existing_owned_service',pins)
    guard = preparation.load_guard();harness = guard.load_harness();preparation.HERE = HERE
    verify(pins)
    preparation.owned_command(guard,harness,command,environment,output/'compile',lock_fd,1800,'official-safe-trim')
    actual = tomllib.loads(options.read_text())
    expected = dict(trim=1,strip_ir=1,strip_metadata=0,incremental=0,opt_level=3,debug_level=2,default_threads=8,
        output_o=str(object_file),output_bc=str(bitcode),input_image=str(sysimage),app_source=str(snapshot/'src/ClbgNbodyPilot.jl'),
        output_kind='--output-lib',include_ccallable_roots=True,abi_output=str(abi),workload_state_initialized=False)
    assert actual == expected, ('Actual options/root protocol differs',actual)
    artifacts = []
    for path in (object_file,bitcode,abi,options):
        assert path.is_file() and path.stat().st_size > 0
        digest = sha(path)
        with path.open('rb') as stream:header = stream.read(16)
        artifacts.append(dict(path=str(path),sha256=digest,bytes=path.stat().st_size,header_hex=header.hex()))
        merge(pins,{str(path):digest})
    abi_detail = entry_abi(abi)
    verify(pins)
    save(output/'safe-build-result.json',dict(status='safe-trim-build-complete-awaiting-native-image-and-fallback-review',
        actual_options=actual,artifacts=artifacts,exported_entry=abi_detail,safe_build_exit_status=0,
        verifier_diagnostics=str(output/'compile/official-safe-trim.stderr'),verifier_diagnostics_require_root_review=True,
        native_owner_qualified=False,actual_ir_removal_qualified=False,fallback_closure_qualified=False,
        runtime_aot_proven=False,chart_admission=False,actual_workload_executions=0,actual_benchmark_runs=0,
        linker_executions=0,LLDB_attempts=0,pins=dict(pins)))


def main():
    parser = argparse.ArgumentParser()
    for name in ('revision','worker-sha256','options-sha256','review-sha256','pilot-sha256','entry-sha256'):parser.add_argument('--'+name,required=True)
    args = parser.parse_args()
    assert re.fullmatch('[a-z0-9][a-z0-9-]{0,63}',args.revision)
    assert all(re.fullmatch('[a-f0-9]{64}',value) for key,value in vars(args).items() if key.endswith('_sha256'))
    for path in (Path(__file__).resolve(),P/'build_options.jl',P/'REVIEW.md',P/'entry_static.jl'):assert not path.stat().st_mode & 0o222
    def interrupted(signum,frame):raise KeyboardInterrupt('Safe trim build interrupted by signal '+str(signum))
    for signum in (signal.SIGINT,signal.SIGTERM,signal.SIGHUP):signal.signal(signum,interrupted)
    with (HERE/'static-io-safe-trim-v1-owner.lock').open('a+') as lease, (FAILED_JOB/'owner.lock').open('rb') as previous:
        fcntl.flock(lease,fcntl.LOCK_EX|fcntl.LOCK_NB)
        fcntl.flock(previous,fcntl.LOCK_SH|fcntl.LOCK_NB)
        opened,named = os.fstat(previous.fileno()),Path(previous.name).stat();assert (opened.st_dev,opened.st_ino) == (named.st_dev,named.st_ino)
        print('WAIT common mutex: Julia static I/O official safe trim build only',flush=True)
        with LOCK.open('a+') as common:
            fcntl.flock(common,fcntl.LOCK_EX)
            opened,named = os.fstat(common.fileno()),LOCK.stat()
            assert (opened.st_dev,opened.st_ino) == (named.st_dev,named.st_ino) and opened.st_ino == 79859134
            output,buildroot = HERE/'jobs'/args.revision,HERE/'.build'/args.revision
            assert not output.exists() and not buildroot.exists(),'No retry, resume or output overwrite'
            output.mkdir(parents=True);buildroot.mkdir(parents=True)
            pins = {};outcome = dict(status='failed-safe-trim-build',started_at=datetime.now(timezone.utc).isoformat(),
                runtime_aot_proven=False,chart_admission=False,actual_workload_executions=0,actual_benchmark_runs=0,LLDB_attempts=0)
            try:
                bind(Path(sys.executable).resolve(),sha(Path(sys.executable).resolve()),pins)
                build(args,output,buildroot,pins,common.fileno())
                outcome['status'] = 'safe-trim-build-complete-awaiting-native-image-and-fallback-review'
            except BaseException as error:
                outcome.update(error=str(error) or type(error).__name__,error_type=type(error).__name__)
                with (output/'actual-error.log').open('x') as stream:stream.write(traceback.format_exc())
                raise
            finally:
                failures = []
                for path,digest in list(pins.items()):
                    try:verify({path:digest})
                    except BaseException as error:failures.append(dict(path=path,error=str(error),error_type=type(error).__name__))
                available = {}
                for root in (output,buildroot):
                    for path in root.rglob('*'):
                        if path.is_file():
                            try:available[str(path)] = sha(path)
                            except BaseException as error:failures.append(dict(path=str(path),error=str(error),error_type=type(error).__name__))
                if failures:outcome.update(status='failed-terminal-integrity',integrity_errors=failures)
                outcome.update(finished_at=datetime.now(timezone.utc).isoformat(),prerequisite_pins=pins,available_evidence_pins=available,
                    common_mutex_inode=79859134,exclusive_common_mutex_held=True,original_pilot_preserved=True)
                save(output/'outcome.json',outcome)
                if failures:raise RuntimeError('Safe trim terminal pin check failed; all actual evidence retained')


if __name__ == '__main__':main()
