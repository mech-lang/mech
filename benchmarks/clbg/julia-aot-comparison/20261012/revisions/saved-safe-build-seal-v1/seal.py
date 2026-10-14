"""Seal saved successful safe compilation; no Julia/native command is reachable."""
import argparse
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
PRIOR = HERE/'revisions/static-io-safe-trim-v1'
PRIOR_WORKER_SHA = 'ff9e3b0c8a0b803d53f9e4a893815db3daed7c6f922cd5e4bce8e3544ccb3bcd'
PRIOR_JOB = CLBG/'continuation/owned-jobs/julia-official-static-io-safe-trim-nbody-v1'
PRIOR_OUT = HERE/'jobs/official-static-io-safe-trim-nbody-v1'
PRIOR_BUILD = HERE/'.build/official-static-io-safe-trim-nbody-v1'
ROOT = CLBG/'continuation/jobs/julia-static-io-safe-compile-closed-review-v1/receipt.json'
ROOT_SHA = '1be37bffac06a836065ce98fe5f65eab43458c5db2e6a0f3d7a7d18942ef84fa'
OUTCOME_SHA = 'c8df203a9830f6eafef84c4a00b80e52203c444d898b258f33d74e6b2e0ef822'
TERMINAL_SHA = 'a91abacb6c303d6eb5434732308e16a426364fe2648de3acba3af69e33b3ff16'
REQUEST_SHA = 'a6430bf6c7cf741c05f5f36100c5dbfb900ab48c5ff1247d5ecfa545f2fc04a6'
NATIVE_SHA = 'af1f0626ad8e2e31b909f3a6b2da724892df39acc360bf9a1a800baa45d1cd61'
ADAPTATION_SHA = '4695accea9eee9b0932b2e7f2f24bce8097ee08cf5164390f7eca112b8e39e6f'
ARTIFACTS = [
    ('nbody-safe-o.a','e25e9acec36b78baacc0e910df86503fc7de8918e4043b73c5328ac04b0d4654',9444136,'213c617263683e0a23312f3132202020'),
    ('nbody-safe.bc','fbf05e04d88c25deda0d0f1ebc9b78f167090d0b02739a97f8e6974c741d29bd',9672560,'213c617263683e0a23312f3132202020'),
    ('entry-abi.json','894267e7b45236df8604d64c25961362e04b7dab6127247dc155ff4774c1befa',823,'7b0a20202266756e6374696f6e73223a'),
    ('actual-build-options.toml','1093912c3d98ec83b114f783931d70bd9297cf5295938138dcca0e1489917f6f',1184,'446963747b537472696e672c20416e79')]
PYTHON = '/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3.12'


def parse_options(raw):
    """Only the observed bounded Dict of unescaped strings, integers and booleans."""
    assert type(raw) is bytes and 0 < len(raw) <= 16384
    text = raw.decode('ascii');prefix = 'Dict{String, Any}('
    assert text.startswith(prefix) and text.endswith(')')
    body = text[len(prefix):-1];result={};offset=0
    token = re.compile(r'"([a-z_]+)" => ("[^"\\\r\n]*"|true|false|0|[1-9][0-9]*)(, |$)')
    while offset < len(body):
        match = token.match(body,offset);assert match is not None,'Unsupported saved options syntax'
        key,value,separator = match.groups();assert key not in result,'Duplicate saved options key'
        result[key] = value[1:-1] if value.startswith('"') else value == 'true' if value in ('true','false') else int(value)
        offset = match.end()
        assert separator or offset == len(body)
        assert separator != ', ' or offset < len(body),'Trailing saved options separator'
    assert offset == len(body) and result
    return result


def load_original(pins):
    path = PRIOR/'build.py';assert not path.stat().st_mode & 0o222
    raw = path.read_bytes();assert hashlib.sha256(raw).hexdigest() == PRIOR_WORKER_SHA
    name = 'julia_saved_safe_build_pure_helpers_v1';assert name not in sys.modules
    module = types.ModuleType(name);module.__file__ = str(path);sys.modules[name] = module
    exec(compile(raw,str(path),'exec'),module.__dict__)
    module.bind(path,PRIOR_WORKER_SHA,pins);return module


def seal(args,output,pins,base):
    for name,digest in [('seal.py',args.worker_sha256),('REVIEW.md',args.review_sha256)]:
        assert not (P/name).stat().st_mode & 0o222;base.bind(P/name,digest,pins)
    def read(path,digest):return json.loads(base.bind(path,digest,pins).read_text())
    root = read(ROOT,ROOT_SHA)
    assert root['status'] == 'closed-successful-static-IO-safe-compile-stopped-at-options-text-parser' and root['verified_pins'] == 10072
    assert root['outcome_sha256'] == OUTCOME_SHA and root['durable_terminal_sha256'] == TERMINAL_SHA and root['request_sha256'] == REQUEST_SHA
    assert root['native_compile_receipt'] == dict(path=str(PRIOR_OUT/'compile/official-safe-trim.json'),sha256=NATIVE_SHA)
    assert root['native_compile_exit'] == 0 and root['native_compile_pid'] == 53013 and root['worker_pid'] == 52972 and root['command_pid'] == 52976
    assert root['verifier_stdout_empty'] is root['verifier_stderr_empty'] is root['safe_trim_and_strip_ir_requested'] is True
    assert root['source_adaptation_sha256'] == ADAPTATION_SHA and root['options_observed_format'] == 'Julia Dict{String, Any} display, not TOML'
    assert root['controller_error'] == 'TOMLDecodeError at line1 column5 after native compile success'
    assert root['original_and_failure_bytes_preserved'] is root['common_mutex_held'] is True
    assert root['actual_workload_executions'] == root['benchmark_runs'] == root['LLDB_attempts'] == 0 and root['runtime_aot_proven'] is root['chart_admission'] is False
    expected_artifacts = [dict(path=str(PRIOR_BUILD/name),sha256=digest,bytes=size,header_hex=header) for name,digest,size,header in ARTIFACTS]
    assert root['artifacts'] == expected_artifacts
    terminal = read(PRIOR_JOB/'terminal.json',TERMINAL_SHA)
    request = read(PRIOR_JOB/'request.json',REQUEST_SHA)
    started,worker = [read(PRIOR_JOB/name,base.sha(PRIOR_JOB/name)) for name in ('command-started.json','worker-started.json')]
    command = [PYTHON,'-I','-S','-B','-u',str(PRIOR/'build.py'),'--revision','official-static-io-safe-trim-nbody-v1',
        '--worker-sha256',PRIOR_WORKER_SHA,'--options-sha256','14a7473570cb9b8e6d1b92a4b71d2138d906bffcc166d82982d909979b857ad2',
        '--review-sha256','f58389bf2f5378402f4c6ee4e77537c6f7f1cecfc64e01f9a032fe6085d18a2e',
        '--pilot-sha256','39d2d809b45780918c6aa11e6e3d7f9f41218fd940b12d29297244f734259e95',
        '--entry-sha256','0fa7af0ecc58d6fd3ea3cdf810f70fc4ca67274f1f703852fa573d15de9f143d']
    assert request['command'] == started['command'] == worker['command'] == command
    assert request['cwd'] == started['cwd'] == worker['cwd'] == str(CLBG.parent.parent)
    assert request['common_mutex_owned_by_target_command'] is True and request['qualification_granted'] is request['automatic_retry'] is False
    assert terminal['status'] == 'command-failed' and type(terminal['exit_code']) is int and terminal['exit_code'] == 1
    assert terminal['actual_command_started'] is True and terminal['command_identity_observation_missing'] is False and terminal['stop_signals'] == []
    assert terminal['qualification_granted'] is terminal['automatic_retry'] is False and not any(terminal.get(key) for key in ('error','exception_cleanup','error_log_write_failure'))
    assert started['command_identity'] == terminal['command_identity'] and started['command_pid'] == terminal['command_pid'] == 52976
    assert started['command_identity_observation_missing'] is False and worker['worker_identity'] == terminal['worker_identity'] and terminal['worker_identity']['pid'] == 52972
    listing = terminal['command_identity']['listing'].split(None,7)
    assert listing[:2] == ['52976','52972'] and listing[7].split() == command
    outcome = read(PRIOR_OUT/'outcome.json',OUTCOME_SHA)
    assert outcome['status'] == 'failed-safe-trim-build' and outcome['error_type'] == 'TOMLDecodeError'
    assert outcome['runtime_aot_proven'] is outcome['chart_admission'] is False and outcome['exclusive_common_mutex_held'] is True and outcome['common_mutex_inode'] == 79859134
    assert outcome['actual_workload_executions'] == outcome['actual_benchmark_runs'] == outcome['LLDB_attempts'] == 0 and not outcome.get('integrity_errors')
    for mapping in (outcome['prerequisite_pins'],outcome['available_evidence_pins']):
        for path,digest in mapping.items():base.bind(path,digest,pins)
    native = read(PRIOR_OUT/'compile/official-safe-trim.json',NATIVE_SHA)
    source = json.loads((PRIOR_OUT/'source-and-build-request.json').read_text())
    assert native['command'] == source['command'] and native['pid'] == native['identity']['pid'] == 53013
    assert type(native['exit_status']) is type(native['final_exit_status']) is int and native['exit_status'] == native['final_exit_status'] == 0
    assert native['inherited_common_mutex'] is native['preparation_only'] is native['never_chart_eligible'] is True and native['timeout_seconds'] == 1800
    assert not any(native.get(key) for key in ('error','error_type','cleanup_errors'))
    assert (PRIOR_OUT/'compile/official-safe-trim.stdout').read_bytes() == (PRIOR_OUT/'compile/official-safe-trim.stderr').read_bytes() == b''
    build_command = [str(base.RUNTIME/'bin/julia'),'--startup-file=no','--history-file=no','--threads=8,0','--project='+str(PRIOR_BUILD/'ClbgNbodyPilot'),
        '-J',str(base.RUNTIME/'lib/julia/sys.dylib'),'-O3','-g2','--output-incremental=no','--experimental','--trim=safe','--strip-ir',
        '--output-o='+str(PRIOR_BUILD/'nbody-safe-o.a'),'--output-bc='+str(PRIOR_BUILD/'nbody-safe.bc'),str(PRIOR_BUILD/'build_options.jl'),
        str(base.TRIMMING/'juliac-buildscript.jl'),str(PRIOR_BUILD/'ClbgNbodyPilot/src/ClbgNbodyPilot.jl'),'--output-lib','true',
        str(PRIOR_BUILD/'entry-abi.json'),str(PRIOR_BUILD/'actual-build-options.toml')]
    assert native['command'] == build_command
    identity = native['identity']['process_listing'].split(None,5)
    assert identity[5].split() == build_command
    adaptation = read(PRIOR_OUT/'static-io-source-adaptation.json',ADAPTATION_SHA)
    assert adaptation['status'] == 'exact-declared-static-IO-only-source-derivation'
    assert adaptation['energy_formatting_and_kernel_unchanged'] is adaptation['positive_output_before_and_after_all_steps'] is True
    assert adaptation['shared_fresh_module_used_by_matched_derived_JIT'] is adaptation['error_message_changed_to_static_literal'] is True
    assert adaptation['actual_workload_executions'] == 0 and adaptation['source_LOC_admitted'] is adaptation['chart_admission'] is False
    assert len(adaptation['workload']['edits']) == 2 and len(adaptation['entry']['edits']) == 1
    for part in ('workload','entry'):
        info = adaptation[part]
        base.bind(info['original_path'],info['original_sha256'],pins);base.bind(info['path'],info['sha256'],pins)
        original = Path(info['original_path']).read_text()
        replacements = [(edit['original'],edit['replacement'],len(info['edits'])) for edit in info['edits'][:1]]
        derived,edits,untouched = base.edit_spans(original,replacements)
        assert edits == info['edits'] and untouched == info['untouched_original_spans'] and derived == Path(info['path']).read_text()
    artifacts = []
    for expected in expected_artifacts:
        path = base.bind(expected['path'],expected['sha256'],pins)
        with path.open('rb') as stream:header=stream.read(16)
        assert path.stat().st_size == expected['bytes'] and header.hex() == expected['header_hex']
        artifacts.append(dict(expected))
    assert bytes.fromhex(artifacts[0]['header_hex']).startswith(b'!<arch>\n') and bytes.fromhex(artifacts[1]['header_hex']).startswith(b'!<arch>\n')
    options = parse_options((PRIOR_BUILD/'actual-build-options.toml').read_bytes())
    expected_options = dict(trim=1,strip_ir=1,strip_metadata=0,incremental=0,opt_level=3,debug_level=2,default_threads=8,
        output_o=str(PRIOR_BUILD/'nbody-safe-o.a'),output_bc=str(PRIOR_BUILD/'nbody-safe.bc'),input_image=str(base.RUNTIME/'lib/julia/sys.dylib'),
        app_source=str(PRIOR_BUILD/'ClbgNbodyPilot/src/ClbgNbodyPilot.jl'),output_kind='--output-lib',include_ccallable_roots=True,
        abi_output=str(PRIOR_BUILD/'entry-abi.json'),workload_state_initialized=False)
    assert set(options) == set(expected_options) and all(type(options[key]) is type(value) and options[key] == value for key,value in expected_options.items())
    abi = base.entry_abi(PRIOR_BUILD/'entry-abi.json')
    assert abi['all_exported_symbols'] == ['clbg_entry']
    base.verify(pins)
    base.save(output/'actual-build-options.json',dict(format='parsed-exact-saved-Julia-Dict-display-v1',
        original=dict(path=str(PRIOR_BUILD/'actual-build-options.toml'),sha256=ARTIFACTS[3][1]),options=options,evaluation_used=False))
    base.save(output/'safe-build-result.json',dict(status='safe-trim-build-complete-awaiting-native-image-and-fallback-review',
        actual_options=options,artifacts=artifacts,artifact_container_types=dict(object='ar-archive',bitcode='ar-archive-not-yet-member-qualified'),
        exported_entry=abi,safe_build_exit_status=0,compile_receipt=dict(path=str(PRIOR_OUT/'compile/official-safe-trim.json'),sha256=NATIVE_SHA),
        original_failed_controller=dict(path=str(PRIOR_OUT/'outcome.json'),sha256=OUTCOME_SHA),original_controller_failure_retained=True,
        verifier_stdout_empty=True,verifier_stderr_empty=True,compiled_sources_and_original_bytes_preserved=True,
        native_owner_qualified=False,actual_ir_removal_qualified=False,fallback_closure_qualified=False,
        runtime_aot_proven=False,chart_admission=False,actual_native_processes=0,successful_native_compile_reexecutions=0,
        actual_workload_executions=0,actual_benchmark_runs=0,linker_executions=0,LLDB_attempts=0,fixture_reexecutions=0,pins=dict(pins)))


def main():
    parser = argparse.ArgumentParser()
    for name in ('revision','worker-sha256','review-sha256'):parser.add_argument('--'+name,required=True)
    args=parser.parse_args();assert re.fullmatch('[a-z0-9][a-z0-9-]{0,63}',args.revision)
    assert all(re.fullmatch('[a-f0-9]{64}',value) for key,value in vars(args).items() if key.endswith('_sha256'))
    def interrupted(signum,frame):raise KeyboardInterrupt('Julia saved build seal interrupted by signal '+str(signum))
    for signum in (signal.SIGINT,signal.SIGTERM,signal.SIGHUP):signal.signal(signum,interrupted)
    with (PRIOR_JOB/'owner.lock').open('rb') as dependency, (HERE/'saved-safe-build-seal-v1-owner.lock').open('a+') as owner:
        fcntl.flock(dependency,fcntl.LOCK_SH|fcntl.LOCK_NB);fcntl.flock(owner,fcntl.LOCK_EX|fcntl.LOCK_NB)
        opened,named=os.fstat(dependency.fileno()),Path(dependency.name).stat();assert (opened.st_dev,opened.st_ino)==(named.st_dev,named.st_ino)
        opened,named=os.fstat(owner.fileno()),Path(owner.name).stat();assert (opened.st_dev,opened.st_ino)==(named.st_dev,named.st_ino)
        print('WAIT common mutex: saved Julia compile/options/ABI metadata only',flush=True)
        with LOCK.open('r+') as common:
            fcntl.flock(common,fcntl.LOCK_EX);opened,named=os.fstat(common.fileno()),LOCK.stat()
            assert (opened.st_dev,opened.st_ino)==(named.st_dev,named.st_ino) and opened.st_ino==79859134
            output=HERE/'jobs'/args.revision;assert not output.exists() and not output.is_symlink()
            output.mkdir(parents=True,exist_ok=False);pins={}
            outcome=dict(status='failed-saved-safe-build-metadata-seal',runtime_aot_proven=False,chart_admission=False,
                actual_native_processes=0,successful_native_compile_reexecutions=0,actual_workload_executions=0,
                actual_benchmark_runs=0,linker_executions=0,LLDB_attempts=0,fixture_reexecutions=0)
            try:
                base=load_original(pins);base.bind(Path(sys.executable).resolve(),base.sha(Path(sys.executable).resolve()),pins)
                seal(args,output,pins,base);outcome['status']='safe-trim-build-complete-awaiting-native-image-and-fallback-review'
            except BaseException as error:
                outcome.update(error=str(error) or type(error).__name__,error_type=type(error).__name__)
                try:
                    with (output/'error.log').open('x') as stream:stream.write(traceback.format_exc())
                except BaseException as secondary:outcome['error_log_write_failure']=str(secondary)
                raise
            finally:
                failures,available=[],{}
                def digest(path):
                    with Path(path).open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()
                for path,expected in pins.items():
                    try:assert digest(path)==expected
                    except BaseException as error:failures.append(dict(path=path,error=str(error) or type(error).__name__))
                for path in output.rglob('*'):
                    if path.is_file():
                        try:available[str(path)]=digest(path)
                        except BaseException as error:failures.append(dict(path=str(path),error=str(error) or type(error).__name__))
                if failures:outcome.update(status='failed-terminal-integrity',integrity_errors=failures)
                outcome.update(finished_at=datetime.now(timezone.utc).isoformat(),prerequisite_pins=pins,available_evidence_pins=available,
                    exclusive_common_mutex_held=True,common_mutex_inode=79859134,original_native_success_and_failed_controller_preserved=True)
                with (output/'outcome.json').open('x') as stream:json.dump(outcome,stream,indent=2);stream.write('\n')
                if failures:raise RuntimeError('Julia saved build metadata integrity failed; no native command repeated')


if __name__=='__main__':main()
