"""Two bounded generic attach cells; no Julia/canary execution or admission."""
import argparse
from datetime import datetime, timezone
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import time
import traceback
from types import FunctionType, SimpleNamespace

PACKET = Path(__file__).resolve().parent
HERE = PACKET.parents[1]
CLBG = HERE.parents[1]
LOCK = Path('/tmp/mech-clbg-timing.lock')
SERVICE = CLBG/'c3-comparison/20261012/revisions/private-v1/guarded_prepare.py'
SERVICE_PROPOSAL = CLBG/'c3-comparison/20261012/proposals/private-v1.json'
SERVICE_SHA = '43438603d675472393050d05928e488a20f16040493935aca676007fc0ada050'
SUSPENDED = HERE/'revisions/native-observer-v2/suspended_child.py'
ENVIRONMENT_REPORT = HERE/'jobs/attach-environment-v1/report.json'
ENVIRONMENT_SHA = 'aa4d725c15347daecf75be5f8110c125f6e1759ed9e2d8d1cd868ae833470e94'
MODES = ('default-target-default-listener', 'preselected-target-custom-listener')
OWNERSHIP_IDS = [
    'test_guarded_prepare.OwnershipChecks.test_popen_failure_has_no_foreign_cleanup',
    'test_guarded_prepare.OwnershipChecks.test_identity_failure_cleans_known_child',
    'test_guarded_prepare.OwnershipChecks.test_journal_failure_cleans_known_child',
    'test_guarded_prepare.OwnershipChecks.test_timeout_preserves_complete_drained_output',
    'test_guarded_prepare.OwnershipChecks.test_orphan_compiler_child_is_fatal',
    'test_guarded_prepare.OwnershipChecks.test_cleanup_failure_is_fatal_and_waits',
    'test_guarded_prepare.OwnershipChecks.test_live_child_cleanup_failure_is_bounded_and_only_signals_owned_group',
    'test_guarded_prepare.ProbeRestorationChecks.test_setup_failure_restores_present_state',
    'test_guarded_prepare.ProbeRestorationChecks.test_absent_input_key_is_removed_on_failure',
    'test_guarded_prepare.ProbeRestorationChecks.test_parent_failure_archives_actual_sampler_bytes',
    'test_guarded_prepare.ProbeRestorationChecks.test_trace_failure_archives_and_stops']


def now(): return datetime.now(timezone.utc).isoformat()


def sha(path):
    with Path(path).open('rb') as stream: return hashlib.file_digest(stream,'sha256').hexdigest()


def save(path, value):
    Path(path).parent.mkdir(parents=True, exist_ok=True)
    with Path(path).open('x') as stream:
        json.dump(value,stream,indent=2); stream.write('\n'); stream.flush(); os.fsync(stream.fileno())


def pin(path, pins, expected=None):
    path = Path(path).resolve(strict=True); digest = sha(path)
    assert expected is None or digest == expected, ('Changed reviewed evidence',str(path))
    assert str(path) not in pins or pins[str(path)] == digest
    pins[str(path)] = digest
    return digest


def verify(pins):
    for path,digest in pins.items(): assert sha(path) == digest, ('Pinned bytes changed',path)


def fresh(path, name, pins):
    path = Path(path).resolve(strict=True)
    assert name not in sys.modules and sha(path) == pins[str(path)]
    source = path.read_bytes(); assert hashlib.sha256(source).hexdigest() == pins[str(path)]
    module = importlib.util.module_from_spec(importlib.util.spec_from_file_location(name,path))
    sys.modules[name] = module; exec(compile(source,str(path),'exec'),module.__dict__)
    assert sys.modules[name] is module and Path(module.__file__).resolve() == path
    assert sha(path) == pins[str(path)]
    return module


def load_service(pins):
    pin(SERVICE_PROPOSAL,pins,SERVICE_SHA)
    proposal = json.loads(SERVICE_PROPOSAL.read_text())
    assert proposal['status'] == 'sealed-awaiting-root-approval'
    receipt_path = Path(proposal['actual_mock_fixture_receipt']).resolve(strict=True)
    pin(receipt_path,pins,proposal['actual_mock_fixture_receipt_sha256'])
    assert pins[str(receipt_path)] == proposal['pins'][str(receipt_path)]
    receipt = json.loads(receipt_path.read_text())
    assert receipt['status'] == 'passed' and receipt['inventory'] == 'ownership'
    assert receipt['expected_test_ids'] == OWNERSHIP_IDS and receipt['successful'] is True
    assert type(receipt['tests_run']) is type(receipt['discovered_tests']) is int
    assert receipt['tests_run'] == receipt['discovered_tests'] == 11
    assert len(receipt['actual_test_ids']) == len(set(receipt['actual_test_ids'])) == 11
    assert set(receipt['actual_test_ids']) == set(OWNERSHIP_IDS)
    assert all(receipt[key] == [] for key in ('skipped','errors','failures','expected_failures','unexpected_successes'))
    for path,digest in proposal['pins'].items():
        if Path(path).suffix == '.py' or (Path(path).parent == receipt_path.parent and
            Path(path).name.startswith('ownership-fixtures')): pin(path,pins,digest)
    service = fresh(SERVICE,'attach_calibration_c3_owned_service',pins)
    sdk_receipts = [Path(path) for path in proposal['pins'] if path.endswith('/sdk-path.stdout')]
    assert len(sdk_receipts) == 1
    pin(sdk_receipts[0],pins,proposal['pins'][str(sdk_receipts[0])])
    sdk = Path(sdk_receipts[0].read_text().strip()).resolve(strict=True)
    for relative in ('usr/include/libproc.h','usr/include/sys/proc_info.h'):
        pin(sdk/relative,pins)
    verify(pins)
    return service,sdk


class OwnedServices:
    """Unchanged C3 command body, exact Popen registration and bounded queries.

    Its original ps-based identity/group dependencies have no query timeout.
    This local argument adapter never executes those dependencies or mutates
    their modules. All process signals require an exact unreaped owned leader.
    """
    def __init__(self, service, output, lock_fd):
        self.output,self.lock_fd = output,lock_fd
        self.processes = {}; self.allowed_groups = set(); self.query_index = 0
        def recorded_popen(*args,**kwargs):
            assert kwargs['start_new_session'] is True and kwargs['pass_fds'] == (lock_fd,)
            process = subprocess.Popen(*args,**kwargs)
            self.processes[process.pid] = process; self.allowed_groups.add(process.pid)
            return process
        class ProcessAPI:
            Popen = staticmethod(recorded_popen)
            def __getattr__(self,name): return getattr(subprocess,name)
        namespace = dict(service.owned_command.__globals__); namespace['subprocess'] = ProcessAPI()
        self.command = FunctionType(service.owned_command.__code__,namespace,
            service.owned_command.__name__,service.owned_command.__defaults__,service.owned_command.__closure__)
        assert self.command.__code__ is service.owned_command.__code__
        self.guard = SimpleNamespace(now=now,save=save,identity=self.identity)

    def identity(self,pid):
        process = self.processes[pid]
        if process.poll() is not None: return None
        assert os.getpgid(pid) == pid
        return dict(pid=pid,pgid=pid,identity_basis='exact unreaped Popen plus kernel process group',
            command=process.args)

    def group_members(self,pgid):
        assert pgid in self.allowed_groups
        self.query_index += 1
        directory = self.output/'group-queries'/str(self.query_index); directory.mkdir(parents=True)
        argv = ['/bin/ps','-axo','pid=,pgid=,stat=,comm=']
        record = dict(command=argv,queried_owned_pgid=pgid,timeout_seconds=3,
            inherited_common_mutex=True,started_at=now()); process = None; stdout=stderr=b''
        try:
            process = subprocess.Popen(argv,stdin=subprocess.DEVNULL,stdout=subprocess.PIPE,stderr=subprocess.PIPE,
                start_new_session=True,pass_fds=(self.lock_fd,))
            record.update(pid=process.pid,pgid=process.pid)
            stdout,stderr = process.communicate(timeout=3)
            record['exit_status'] = process.returncode
            assert process.returncode == 0
            try: os.killpg(process.pid,0)
            except ProcessLookupError: pass
            else: raise RuntimeError('Owned ps query reaped with a remaining group; no further signal sent')
            result = []
            for line in stdout.decode().splitlines():
                fields = line.split(None,3); assert len(fields) == 4
                if int(fields[1]) == pgid and not fields[2].startswith('Z'):
                    result.append(dict(pid=int(fields[0]),pgid=pgid,state=fields[2],executable=fields[3]))
            record['members'] = result
            return result
        except BaseException as error:
            record.update(error=str(error) or type(error).__name__,error_type=type(error).__name__)
            if isinstance(error,subprocess.TimeoutExpired): stdout,stderr = error.output or b'',error.stderr or b''
            raise
        finally:
            if process is not None:
                if process.poll() is None:
                    try:
                        assert os.getpgid(process.pid) == process.pid
                        os.killpg(process.pid,signal.SIGKILL)
                        stdout,stderr = process.communicate(timeout=2)
                    except BaseException as error:
                        record['cleanup_error'] = str(error) or type(error).__name__
                        if process.poll() is None:
                            try: process.kill(); process.wait(timeout=2)
                            except BaseException as fallback: record['fallback_error'] = str(fallback) or type(fallback).__name__
                record['final_exit_status'] = process.returncode
                if process.stdout: process.stdout.close()
                if process.stderr: process.stderr.close()
            (directory/'stdout').write_bytes(stdout); (directory/'stderr').write_bytes(stderr)
            record['finished_at'] = now(); save(directory/'receipt.json',record)
            if record.get('cleanup_error'): raise RuntimeError('Bounded group query cleanup failed; receipt retained')

    def kill_group(self,pgid):
        process = self.processes[pgid]
        if process.poll() is None:
            assert os.getpgid(pgid) == pgid
            try: os.killpg(pgid,signal.SIGKILL)
            except ProcessLookupError: pass
        # Never signal a reaped/stale leader. An orphan group is a fatal
        # ownership failure, retained for inspection without guessing PIDs.
        deadline = time.monotonic()+5
        while self.group_members(pgid):
            if process.returncode is not None or time.monotonic() >= deadline:
                raise RuntimeError('Owned command group remains after bounded cleanup')
            time.sleep(0.01)

    def run(self,command,environment,directory,timeout,name):
        return self.command(self.guard,self,command,environment,directory,self.lock_fd,timeout,name)


def prerequisites(args,pins):
    report_path = Path(args.environment_report).resolve(strict=True)
    assert report_path == ENVIRONMENT_REPORT and args.environment_report_sha256 == ENVIRONMENT_SHA
    pin(report_path,pins,args.environment_report_sha256)
    report = json.loads(report_path.read_text())
    assert report['status'] == 'read-only-attach-environment-saved-awaiting-interpretation'
    assert report['exclusive_common_timing_mutex_held'] is report['pins_unchanged_at_end'] is True
    assert report['common_timing_mutex'] == str(LOCK)
    assert report['runtime_aot_proven'] is report['chart_admission'] is report['automatic_retry'] is False
    for key in ('actual_attach_attempts','actual_benchmark_runs','native_canary_executions','permission_changes'):
        assert type(report[key]) is int and report[key] == 0
    assert report['source_or_algorithm_modified'] is False
    for path,digest in report['pins'].items(): pin(path,pins,digest)
    for version in ('v1','v2'):
        item = report['failure_inventory'][version]
        pin(item['report'],pins,item['report_sha256'])
        saved = json.loads(Path(item['report']).read_text())
        assert saved['status'] == 'failed' and saved['runtime_aot_proven'] is saved['chart_admission'] is False
        assert saved['actual_benchmark_runs'] == 0 and saved['automatic_retry'] is False
    old = HERE/'jobs/canary-observation-v2'
    original = json.loads((old/'report.json').read_text())
    command = json.loads((old/'debugger/actual-canary-observation.json').read_text())['command']
    lldb = Path(command[0]).resolve(strict=True); pin(lldb,pins,original['pins'][str(lldb)])
    pin(SUSPENDED,pins,original['pins'][str(SUSPENDED)])
    pyroot = Path((old/'tools/lldb-python/python-path.stdout').read_text().strip()).resolve(strict=True)
    for path in sorted(pyroot.rglob('*')):
        if path.is_file() and path.suffix != '.pyc':
            pin(path,pins,original['pins'].get(str(path.resolve())))
    framework = next(path for path in pyroot.parents if path.name == 'LLDB.framework')
    for path in (framework/'LLDB',framework/'Resources/debugserver'):
        if path.is_file(): pin(path,pins,original['pins'].get(str(path.resolve())))
    return lldb,pyroot,dict(path=str(report_path),sha256=args.environment_report_sha256,
        historical_failure_reports=report['failure_inventory'],
        purpose='prerequisite evidence only; no cause inferred from signing/status queries')


def checked_identity(value,pid,ppid,pgid):
    for key in ('observer_pid','pid','ppid','pgid','birth_seconds','birth_microseconds','bsd_status',
        'bsd_info_bytes','bsd_info_size','proc_pidpath_return'):
        assert type(value[key]) is int
    assert value['pid'] == pid and value['pgid'] == pgid and value['ppid'] > 0
    assert ppid is None or value['ppid'] == ppid
    assert value['birth_seconds'] > 0 and 0 <= value['birth_microseconds'] < 1000000
    assert value['bsd_info_bytes'] == value['bsd_info_size'] > 0
    assert value['proc_pidpath_return'] > 0


def close_child(child,services,directory):
    cleanup = dict(pid=child.pid,only_this_parent_owned_child=True,cleanup_errors=[])
    if child.pid is not None: services.allowed_groups.add(child.pid)
    try:
        if child.pid is not None and not child.reaped:
            # After the owned LLDB command has ended, Darwin may still be
            # restoring the original ptrace parent. Wait at most five seconds
            # without signalling while ECHILD says this parent cannot wait.
            deadline=time.monotonic()+5; temporary_reparent=False
            while True:
                try: waited,status=os.waitpid(child.pid,os.WNOHANG)
                except ChildProcessError:
                    temporary_reparent=True
                    if time.monotonic()>=deadline: raise
                    time.sleep(0.01); continue
                if waited: child.reaped=True; child.final_wait_status=status
                break
            cleanup['temporary_not_waitable_observed']=temporary_reparent
        cleanup.update(child.close(services.group_members))
    except BaseException as error:
        cleanup['cleanup_errors'].append(dict(stage='original-bounded-child-close',error=str(error) or type(error).__name__))
        if child.pid is not None and not child.reaped:
            # Darwin ptrace can temporarily reparent the fork child. Require
            # a successful direct-child wait before any PID fallback signal;
            # ECHILD never grants ownership and never triggers a signal.
            try:
                waited,status=os.waitpid(child.pid,os.WNOHANG)
                if waited: child.reaped=True; child.final_wait_status=status
                else:
                    os.kill(child.pid,signal.SIGKILL)
                    deadline=time.monotonic()+5
                    while time.monotonic()<deadline:
                        waited,status=os.waitpid(child.pid,os.WNOHANG)
                        if waited: child.reaped=True; child.final_wait_status=status; break
                        time.sleep(0.01)
                assert child.reaped
            except BaseException as fallback:
                cleanup['cleanup_errors'].append(dict(stage='exact-unreaped-child-fallback',error=str(fallback) or type(fallback).__name__))
        cleanup.update(reaped=child.reaped,final_wait_status=child.final_wait_status)
        try: cleanup['remaining_native_group_members'] = services.group_members(child.pid) if child.pid else []
        except BaseException as remaining: cleanup['cleanup_errors'].append(dict(stage='bounded-group-reconciliation',error=str(remaining) or type(remaining).__name__))
    finally:
        save(directory/'parent-owned-cleanup.json',cleanup)
    assert not cleanup['cleanup_errors'] and (child.pid is None or cleanup['reaped'] is True)
    assert cleanup['remaining_native_group_members'] == []
    return cleanup


def cell(mode,helper,lldb,pyroot,environment,child_module,services,pins,output):
    directory=output/mode; directory.mkdir(); nonce=output.name+'-'+mode
    child=child_module.SuspendedChild(helper,['--run',nonce],environment,HERE,directory/'child',services.lock_fd)
    result=dict(mode=mode,status='failed-generic-cell',started_at=now(),native_helper_executions=0)
    expected_failure=None
    try:
        owned=child.start(); services.allowed_groups.add(child.pid)
        query=services.run([str(helper),'--identity',str(child.pid)],environment,directory/'identity',10,'kernel-before-exec')
        pre=json.loads(query['stdout']); checked_identity(pre,child.pid,os.getpid(),child.pid)
        assert pre['observer_pid'] == query['detail']['pid'] and pre['role'] == 'pre-exec-kernel-query'
        assert Path(pre['executable_path']).resolve() == Path(sys.executable).resolve()
        pin(pre['executable_path'],pins); save(directory/'preexec-kernel-identity.json',pre)
        request=dict(purpose='generic-owned-attach-calibration',mode=mode,output=str(directory/'observation'),
            executable=str(helper),owned_child=owned,preexec_identity=pre,lldb_python_root=str(pyroot),
            observer_common_mutex_fd=services.lock_fd,common_mutex=str(LOCK))
        save(directory/'request.json',request)
        observer=PACKET/'observe_calibration.py'
        assert sha(observer)==pins[str(observer)]
        script=('script import hashlib, importlib.util, sys; sys.dont_write_bytecode = True; '
            '_p = '+repr(str(observer))+'; _b = open(_p, "rb").read(); '
            'assert hashlib.sha256(_b).hexdigest() == '+repr(pins[str(observer)])+'; '
            'assert "observe_calibration" not in sys.modules; '
            '_s = importlib.util.spec_from_file_location("observe_calibration", _p); '
            '_m = importlib.util.module_from_spec(_s); sys.modules["observe_calibration"] = _m; '
            'exec(compile(_b, _p, "exec"), _m.__dict__); '
            'assert _m.__file__ == _p; _m.observe(lldb.debugger, '+repr(str(directory/'request.json'))+')')
        command=[str(lldb),'--no-lldbinit','--batch',
            '-o',script]
        try: services.run(command,environment,directory/'debugger',45,'generic-attach')
        except BaseException as error:
            # Continue the second independent cell only for a genuine bounded
            # attach/CLI failure with fully proven ownership cleanup below.
            receipt=directory/'debugger/generic-attach.json'
            if not isinstance(error,(subprocess.TimeoutExpired,AssertionError)) or not receipt.is_file(): raise
            saved=json.loads(receipt.read_text())
            assert type(saved.get('final_exit_status')) is int
            assert not any(saved.get(key) for key in ('cleanup_error','cleanup_errors'))
            assert saved['error_type'] in ('TimeoutExpired','AssertionError')
            expected_failure=dict(error=str(error) or type(error).__name__,error_type=type(error).__name__,receipt=str(receipt))
        observation=directory/'observation/report.json'
        if observation.is_file():
            actual=json.loads(observation.read_text()); result['observer_report']=actual
            assert not actual.get('ownership_integrity_failure')
            if actual['status'] == 'generic-attach-exec-observed-awaiting-parent-reconciliation':
                assert expected_failure is None
                result.update(status='generic-attach-exec-reconciled',preexec_identity=pre,attached_pid=child.pid)
            else:
                assert actual['status']=='failed-generic-calibration' and actual.get('error')
                assert not actual.get('debugger_cleanup_error')
                result['observer_failure']=actual['error']
        else:
            assert expected_failure is not None
        if expected_failure: result['bounded_cli_failure']=expected_failure
    finally:
        try:
            result['cleanup']=close_child(child,services,directory)
            native=directory/'child/native.stdout'
            if native.is_file() and native.stat().st_size:
                rows=native.read_text().splitlines(); assert len(rows)==1
                post=json.loads(rows[0]); checked_identity(post,child.pid,None,child.pid)
                assert post['observer_pid']==child.pid and post['role']=='post-exec-self-identity' and post['nonce']==nonce
                assert Path(post['executable_path']).resolve()==helper
                assert (post['birth_seconds'],post['birth_microseconds'])==(pre['birth_seconds'],pre['birth_microseconds'])
                save(directory/'postexec-kernel-identity.json',post)
                result.update(native_helper_executions=1,postexec_identity=post,
                    observed_ppid_transition=dict(preexec=pre['ppid'],postexec=post['ppid'],
                        changed=pre['ppid']!=post['ppid'],tracer_identity_verified=False))
            if result['status']=='generic-attach-exec-reconciled':
                assert result['native_helper_executions']==1
                status=result['cleanup']['final_wait_status']; assert type(status)is int
                assert os.WIFEXITED(status) and os.WEXITSTATUS(status)==0
        except BaseException as error:
            result.update(status='failed-generic-cell-integrity-or-cleanup',
                terminal_error=str(error) or type(error).__name__,terminal_error_type=type(error).__name__)
            raise
        finally:
            result['finished_at']=now(); save(directory/'cell.json',result)
    return result


def main():
    parser=argparse.ArgumentParser(); parser.add_argument('--revision',required=True)
    parser.add_argument('--environment-report',required=True); parser.add_argument('--environment-report-sha256',required=True)
    args=parser.parse_args()
    assert args.revision and len(args.revision)<=40 and all(c in 'abcdefghijklmnopqrstuvwxyz0123456789-' for c in args.revision)
    assert len(args.environment_report_sha256)==64 and all(c in '0123456789abcdef' for c in args.environment_report_sha256)
    print('WAIT common mutex: two generic owned attach calibration cells',flush=True)
    with LOCK.open('a+') as lock:
        fcntl.flock(lock,fcntl.LOCK_EX); held,named=os.fstat(lock.fileno()),LOCK.stat()
        assert (held.st_dev,held.st_ino)==(named.st_dev,named.st_ino)
        duplicate=fcntl.fcntl(lock.fileno(),fcntl.F_DUPFD_CLOEXEC,64)
        output=HERE/'jobs'/args.revision
        try: output.mkdir()
        except BaseException: os.close(duplicate); raise
        pins={}; report=dict(status='failed-generic-calibration',started_at=now(),
            exclusive_common_timing_mutex_held=True,common_timing_mutex=str(LOCK),automatic_retry=False,
            runtime_aot_proven=False,chart_admission=False,actual_benchmark_runs=0,native_canary_executions=0,
            permission_changes=0,source_or_algorithm_modified=False,planned_generic_attach_cells=2,cells=[])
        previous={}
        def interrupted(signum,frame): raise InterruptedError('Owned calibration interrupted by signal '+str(signum))
        try:
            for signum in (signal.SIGINT,signal.SIGTERM,signal.SIGHUP):
                previous[signum]=signal.getsignal(signum); signal.signal(signum,interrupted)
            for path in sorted(PACKET.iterdir()):
                if path.is_file(): pin(path,pins)
            pin(sys.executable,pins); pin('/bin/ps',pins); pin('/usr/bin/lipo',pins)
            lldb,pyroot,prior=prerequisites(args,pins); report['prerequisite_evidence']=prior
            service,sdk=load_service(pins)
            child_module=fresh(SUSPENDED,'attach_calibration_original_suspended_child',pins)
            services=OwnedServices(service,output,duplicate)
            compiler=Path(service.CLANG).resolve(strict=True); pin(compiler,pins)
            helper=output/'build/generic-helper'; helper.parent.mkdir()
            environment=dict(os.environ,PYTHONDONTWRITEBYTECODE='1')
            report['environment_projection']={key:environment.get(key) for key in ('PATH','LANG','LC_ALL',
                'PYTHONPATH','PYTHONHOME','DYLD_LIBRARY_PATH','DYLD_FRAMEWORK_PATH','PYTHONDONTWRITEBYTECODE')}
            services.run([str(compiler),'-arch','arm64','-isysroot',str(sdk),'-mmacosx-version-min=14.4',
                '-std=c11','-O0','-g','-fno-omit-frame-pointer',str(PACKET/'generic_helper.c'),'-o',str(helper)],
                environment,output/'build',60,'compile-generic-helper')
            pin(helper,pins)
            architecture=services.run(['/usr/bin/lipo','-archs',str(helper)],environment,output/'build',10,'helper-architecture')
            assert architecture['stdout'].decode().strip()=='arm64'
            verify(pins)
            for mode in MODES:
                report['cells'].append(cell(mode,helper,lldb,pyroot,environment,child_module,services,pins,output))
                verify(pins)
            report.update(status='generic-calibration-saved-awaiting-interpretation',
                actual_generic_attach_cells=len(report['cells']),
                generic_cells_reconciled=sum(row['status']=='generic-attach-exec-reconciled' for row in report['cells']),
                observer_setup_cause_proven=False,permission_denial_cause_proven=False,
                Julia_attach_capability_proven=False,Julia_native_aot_proven=False,
                old_failure_preexec_image_retroactively_verified=False,pins_unchanged_at_end=True)
        except BaseException as error:
            report.update(error=str(error) or type(error).__name__,error_type=type(error).__name__)
            with (output/'failure.log').open('x') as stream: stream.write(traceback.format_exc())
            raise
        finally:
            available={}; failures=[]
            candidates=dict(pins)
            for path in output.rglob('*'):
                if path.is_file(): candidates.setdefault(str(path.resolve()),None)
            for path,expected in candidates.items():
                try:
                    digest=sha(path); assert expected is None or expected==digest
                    available[path]=digest
                except BaseException as error: failures.append(dict(path=path,error=str(error) or type(error).__name__))
            if failures: report.update(status='failed-terminal-integrity',terminal_integrity_failures=failures)
            report.update(pins=available,finished_at=now())
            try: save(output/'report.json',report)
            finally:
                for signum,handler in previous.items(): signal.signal(signum,handler)
                os.close(duplicate)
            if failures: raise RuntimeError('Calibration terminal evidence incomplete; partial pins retained')
        print(json.dumps(dict(status=report['status'],report=str(output/'report.json'),report_sha256=sha(output/'report.json'))),flush=True)


if __name__=='__main__': main()
