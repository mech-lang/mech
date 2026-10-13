"""Loaded only inside one newly owned LLDB CLI; generic attach, no expressions."""
import json
import os
from pathlib import Path
import time
import traceback
import lldb


def save(path, value):
    with Path(path).open('x') as stream:
        json.dump(value, stream, indent=2); stream.write('\n'); stream.flush(); os.fsync(stream.fileno())


def modules(target):
    return [dict(path=str(module.GetFileSpec()), uuid=module.GetUUIDString(), triple=module.GetTriple())
        for module in target.module_iter()]


def breakpoints(target):
    result = []
    for name in ('main', 'calibration_probe'):
        point = target.BreakpointCreateByName(name)
        assert point.IsValid()
        point.SetEnabled(True); point.SetOneShot(False); point.SetIgnoreCount(0)
        result.append(dict(name=name, id=point.GetID(), locations=point.GetNumLocations()))
    return result


def observe(debugger, request_path):
    request = json.loads(Path(request_path).read_text()); output = Path(request['output'])
    output.mkdir()
    assert request['purpose'] == 'generic-owned-attach-calibration' and request['mode'] in (
        'default-target-default-listener', 'preselected-target-custom-listener')
    owned = request['owned_child']; pid = owned['pid']
    assert type(pid) is int and pid > 0 and owned['pgid'] == pid
    assert owned['parent_retains_POSIX_child_ownership'] is owned['stopped_before_exec'] is True
    assert owned['common_mutex_fd'] >= 64
    held = os.fstat(request['observer_common_mutex_fd']); named = os.stat(request['common_mutex'])
    assert (held.st_dev,held.st_ino) == (named.st_dev,named.st_ino)
    assert Path(lldb.__file__).resolve().is_relative_to(Path(request['lldb_python_root']).resolve())
    report = dict(status='failed-generic-calibration', mode=request['mode'], pid=pid,
        runtime_aot_proven=False, chart_admission=False, actual_benchmark_runs=0,
        target_expressions=0, automatic_retry=False, events=[], attach_returned=False)
    process = target = None; old_async = debugger.GetAsync()
    try:
        debugger.SetAsync(True)
        if request['mode'] == 'default-target-default-listener':
            target = debugger.CreateTarget('')
            listener = lldb.SBListener()  # Documented invalid sentinel selects debugger listener.
            assert not listener.IsValid()
            events = debugger.GetListener(); assert events.IsValid()
            before_breakpoints = []
        else:
            target = debugger.CreateTarget(request['executable'])
            command = lldb.SBCommandReturnObject()
            debugger.GetCommandInterpreter().HandleCommand('settings set target.skip-prologue false', command)
            assert command.Succeeded(), command.GetError()
            before_breakpoints = breakpoints(target)
            listener = lldb.SBListener('mech-generic-owned-attach-calibration')
            assert listener.IsValid(); events = listener
        assert target.IsValid()
        save(output/'pre-attach.json', dict(mode=request['mode'], modules=modules(target),
            breakpoints=before_breakpoints, listener_is_valid=listener.IsValid(),
            debugger_listener_selected=not listener.IsValid(), expected_preexec_kernel_identity=request['preexec_identity']))
        error = lldb.SBError()
        save(output/'attach-entered.json', dict(pid=pid, monotonic=time.monotonic(),
            api='SBTarget.AttachToProcessWithID', mode=request['mode']))
        process = target.AttachToProcessWithID(listener, pid, error)
        report['attach_returned'] = True
        save(output/'attach-returned.json', dict(pid=pid, error=str(error), success=error.Success(),
            process_valid=process.IsValid(), actual_pid=process.GetProcessID() if process.IsValid() else None,
            state=process.GetState() if process.IsValid() else None, modules=modules(target)))
        if process.IsValid() and process.GetProcessID() != pid:
            report['ownership_integrity_failure'] = True
        assert error.Success() and process.IsValid() and process.GetProcessID() == pid, str(error)
        if request['mode'] == 'default-target-default-listener':
            command = lldb.SBCommandReturnObject()
            debugger.GetCommandInterpreter().HandleCommand('settings set target.skip-prologue false', command)
            assert command.Succeeded(), command.GetError()
            report['post_attach_breakpoints'] = breakpoints(target)
        deadline = time.monotonic() + 20; last_stop = None; helper_stops = []
        with (output/'events.jsonl').open('x') as log:
            while time.monotonic() < deadline:
                state = process.GetState()
                if state == lldb.eStateExited:
                    report['exit_status'] = process.GetExitStatus(); break
                assert state not in (lldb.eStateDetached,lldb.eStateInvalid,lldb.eStateCrashed,lldb.eStateSuspended)
                if state == lldb.eStateStopped and process.GetStopID() != last_stop:
                    last_stop = process.GetStopID(); assert len(report['events']) < 32
                    row = dict(stop_id=last_stop, state=state, modules=modules(target), threads=[])
                    for thread in process:
                        frame = thread.GetFrameAtIndex(0)
                        module = frame.GetModule()
                        path = str(module.GetFileSpec()) if module.IsValid() else None
                        name = frame.GetFunctionName() if frame.IsValid() else None
                        row['threads'].append(dict(thread_id=thread.GetThreadID(), reason=thread.GetStopReason(),
                            function=name, module=path))
                        if thread.GetStopReason() == lldb.eStopReasonBreakpoint and path and Path(path).resolve() == Path(request['executable']).resolve() and name:
                            if name in ('main','calibration_probe'): helper_stops.append(name)
                    report['events'].append(row); log.write(json.dumps(row)+'\n'); log.flush(); os.fsync(log.fileno())
                    continued = process.Continue(); assert continued.Success(), str(continued)
                event = lldb.SBEvent(); events.WaitForEvent(1, event)
            else: raise TimeoutError('Generic process event loop exceeded twenty seconds')
        assert type(report.get('exit_status')) is int and report['exit_status'] == 0
        assert 'main' in helper_stops and 'calibration_probe' in helper_stops
        report.update(status='generic-attach-exec-observed-awaiting-parent-reconciliation',
            helper_function_stops=helper_stops, final_modules=modules(target))
    except BaseException as error:
        report.update(error=str(error) or type(error).__name__, error_type=type(error).__name__)
        with (output/'failure.log').open('x') as stream: stream.write(traceback.format_exc())
    finally:
        # This API cleanup may itself block. The parent's finite owned-CLI
        # timeout and separately owned unreaped direct child bound that case.
        try:
            if process is not None and process.IsValid() and process.GetProcessID() != pid:
                report['unowned_process_cleanup_skipped'] = True
            else:
                if process is not None and process.IsValid() and process.GetState() != lldb.eStateExited:
                    killed = process.Kill(); report['debugger_kill_error'] = str(killed)
                    if not killed.Success(): report['debugger_cleanup_error'] = str(killed)
                if target is not None and target.IsValid(): report['target_deleted'] = debugger.DeleteTarget(target)
            debugger.SetAsync(old_async)
        except BaseException as error:
            report.update(status='failed-generic-calibration', debugger_cleanup_error=str(error) or type(error).__name__)
        save(output/'report.json', report)
    print(json.dumps(dict(status=report['status'], output=str(output))), flush=True)
