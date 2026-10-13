"""Review closed two-cell generic LLDB calibration; no native retries."""
from pathlib import Path
import fcntl,hashlib,json,os
from datetime import datetime,timezone
ROOT=Path('/Users/cim310/.codex/worktrees/clbg-nbody-v04/mech')
CONT=ROOT/'benchmarks/clbg/continuation'
COHORT=ROOT/'benchmarks/clbg/julia-aot-comparison/20261012'
ACTUAL=COHORT/'jobs/generic-attach-calibration-v2'
JOB=CONT/'owned-jobs/julia-generic-attach-calibration-v2'
OUT=CONT/'jobs/julia-calibration-v2-closed-review-v1'
LOCK=Path('/tmp/mech-clbg-timing.lock')

def sha(path):
    with Path(path).open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()

def save(path,value):
    with path.open('x') as stream:json.dump(value,stream,indent=2);stream.write('\n')

print('WAIT common mutex: closed two-cell Julia attach receipt/stage inspection',flush=True)
with LOCK.open('r+') as common:
    fcntl.flock(common,fcntl.LOCK_EX);held,named=os.fstat(common.fileno()),LOCK.stat()
    assert (held.st_dev,held.st_ino)==(named.st_dev,named.st_ino) and held.st_ino==79859134
    with (JOB/'owner.lock').open('r+') as lease:
        fcntl.flock(lease,fcntl.LOCK_EX | fcntl.LOCK_NB)
        OUT.mkdir(parents=True);pins={str(Path(__file__).resolve()):sha(Path(__file__).resolve())}
        def read(path):
            pins[str(path)]=sha(path);return json.loads(path.read_text())
        request,worker,started,terminal=[read(JOB/name) for name in ('request.json','worker-started.json','command-started.json','terminal.json')]
        assert terminal['status']=='command-exited-zero-awaiting-report-review' and type(terminal['exit_code']) is int and terminal['exit_code']==0
        assert terminal['actual_command_started'] is True and terminal['stop_signals']==[] and not terminal.get('error')
        assert worker['worker_identity']==terminal['worker_identity']
        assert request['command']==worker['command']==started['command']
        assert request['cwd']==worker['cwd']==started['cwd']==str(ROOT)
        assert started['command_pid']==terminal['command_pid'] and started['command_identity']==terminal['command_identity']
        actual=read(ACTUAL/'report.json')
        assert actual['status']=='generic-calibration-saved-awaiting-interpretation'
        assert actual['planned_generic_attach_cells']==actual['actual_generic_attach_cells']==2 and actual['generic_cells_reconciled']==0
        assert not actual.get('terminal_integrity_failures') and actual['pins_unchanged_at_end'] is True
        assert actual['runtime_aot_proven'] is actual['chart_admission'] is actual['automatic_retry'] is False
        for field in ('actual_benchmark_runs','native_canary_executions','permission_changes','actual_generic_compilations','architecture_query_reexecutions'):
            assert type(actual[field]) is int and actual[field]==0
        for path,digest in actual['pins'].items():
            assert sha(path)==digest,('Changed actual evidence',path)
            assert path not in pins or pins[path]==digest;pins[path]=digest
        rows=[]
        for cell in actual['cells']:
            directory=ACTUAL/cell['mode']
            assert cell['status']=='failed-generic-cell' and cell['native_helper_executions']==0
            failure=cell['bounded_cli_failure'];assert failure['error_type']=='TimeoutExpired'
            receipt=read(directory/'debugger/generic-attach.json')
            assert receipt['timeout_seconds']==45 and receipt['error_type']=='TimeoutExpired'
            assert type(receipt['final_exit_status']) is int and receipt['final_exit_status']!=0
            assert not any(receipt.get(key) for key in ('cleanup_error','cleanup_errors'))
            pre=read(directory/'observation/pre-attach.json');entered=read(directory/'observation/attach-entered.json')
            assert entered['pid']==cell['cleanup']['pid'] and entered['api']=='SBTarget.AttachToProcessWithID'
            assert {path.name for path in (directory/'observation').iterdir()}=={'pre-attach.json','attach-entered.json'}
            assert cell['cleanup']['cleanup_errors']==cell['cleanup']['remaining_native_group_members']==[] and cell['cleanup']['reaped'] is True
            assert (directory/'child/native.stdout').read_bytes()==(directory/'child/native.stderr').read_bytes()==b''
            stdout=(directory/'debugger/generic-attach.stdout').read_bytes();stderr=(directory/'debugger/generic-attach.stderr').read_bytes()
            assert stderr==b'' and stdout.decode().startswith('(lldb) script import hashlib, importlib.util, sys;')
            rows.append(dict(mode=cell['mode'],owned_child_pid=entered['pid'],entered_attach=True,
                durable_attach_returned=False,pre_attach_module_count=len(pre['modules']),pre_attach_breakpoints=pre['breakpoints'],
                listener_is_valid=pre['listener_is_valid'],stdout_bytes=len(stdout),stderr_bytes=len(stderr),
                timeout_seconds=45,LLDB_final_exit_status=receipt['final_exit_status'],native_target_executions=0,
                original_cleanup=cell['cleanup'],stage='after attach-entered; before returned receipt or event loop',
                caveat='API call or immediate post-return introspection: existing returned receipt evaluates state/modules before persisting'))
        for name in ('command.stdout','command.stderr','worker.log'):pins[str(JOB/name)]=sha(JOB/name)
        pilot=read(COHORT/'jobs/nbody-library-pilot-v1/report.json')
        pilot_summary={key:value for key,value in pilot.items() if key!='pins'}
        save(OUT/'existing-pilot-summary.json',pilot_summary)
        # Existing pilot source/symbol plan is inspected, not retrospectively admitted.
        for path,digest in pins.items():assert sha(path)==digest
        review=dict(status='closed-two-attach-timeouts-reviewed-no-native-retry',completed_at=datetime.now(timezone.utc).isoformat(),
            actual_report=str(ACTUAL/'report.json'),actual_report_sha256=sha(ACTUAL/'report.json'),
            durable_terminal=str(JOB/'terminal.json'),durable_finished_at=terminal['finished_at'],durable_exit_code=0,stop_signals=[],
            common_mutex_inode=held.st_ino,private_lease_closed=True,cells=rows,native_helper_executions=0,
            new_native_commands=0,permission_changes=0,runtime_aot_proven=False,chart_admission=False,
            existing_pilot_summary=str(OUT/'existing-pilot-summary.json'),existing_pilot_summary_sha256=sha(OUT/'existing-pilot-summary.json'),pins=pins)
        save(OUT/'review.json',review)
        for path,digest in pins.items():assert sha(path)==digest
        save(OUT/'terminal.json',dict(status=review['status'],body_error=None,integrity_errors=[],review=str(OUT/'review.json'),
            review_sha256=sha(OUT/'review.json'),new_native_commands=0,common_mutex_inode=held.st_ino))
        print(json.dumps(dict(status=review['status'],review=str(OUT/'review.json'),review_sha256=sha(OUT/'review.json'),
            terminal_sha256=sha(OUT/'terminal.json'),actual_report_sha256=review['actual_report_sha256'],cells=rows,verified_pins=len(pins))),flush=True)
