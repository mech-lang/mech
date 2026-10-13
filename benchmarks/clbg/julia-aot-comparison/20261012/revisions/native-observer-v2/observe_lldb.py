"""LLDB-loaded unscored recorder. No result from this file admits Julia AOT."""
import json
import os
from pathlib import Path
import time
import traceback
import lldb


def write_new(path, value):
    with Path(path).open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')


def address_record(address, target):
    module = address.GetModule()
    symbol = address.GetSymbol()
    return dict(file_address=address.GetFileAddress(), loaded_address=address.GetLoadAddress(target),
        module=str(module.GetFileSpec()), module_uuid=module.GetUUIDString(),
        symbol=symbol.GetName() if symbol.IsValid() else None,
        mangled_symbol=symbol.GetMangledName() if symbol.IsValid() else None)


def modules(target):
    return [dict(path=str(module.GetFileSpec()), uuid=module.GetUUIDString(),
        header_file_address=module.GetObjectFileHeaderAddress().GetFileAddress(),
        header_loaded_address=module.GetObjectFileHeaderAddress().GetLoadAddress(target))
        for module in target.module_iter()]


def breakpoint_inventory(target, breakpoints):
    result = []
    for identifier, (descriptor, breakpoint) in breakpoints.items():
        assert breakpoint.IsValid() and breakpoint.IsEnabled()
        assert breakpoint.GetIgnoreCount() == 0 and breakpoint.GetThreadID() == lldb.LLDB_INVALID_THREAD_ID
        locations = []
        for index in range(breakpoint.GetNumLocations()):
            location = breakpoint.GetLocationAtIndex(index)
            assert location.IsEnabled()
            locations.append(dict(location_id=location.GetID(), resolved=location.IsResolved(),
                **address_record(location.GetAddress(), target)))
        result.append(dict(descriptor=descriptor, breakpoint_id=identifier,
            hit_count=breakpoint.GetHitCount(), locations=locations))
    return result


def observe(debugger, request_path):
    request = json.loads(Path(request_path).read_text())
    output = Path(request['output'])
    output.mkdir()
    process = None
    target = None
    old_async = debugger.GetAsync()
    report = dict(status='failed-observation', runtime_aot_proven=False,
        chart_admission=False, actual_benchmark_runs=0, request=request_path,
        all_thread_breakpoints=True, debugger_expressions_evaluated_in_target=0,
        native_arguments=request['arguments'], source_or_algorithm_modified=False)
    breakpoints = {}
    try:
        assert request['actual_common_mutex_held'] is True
        assert request['purpose'] == 'unscored-missing-native-canary-calibration'
        assert request['arguments'] == ['41', '--julia-args', '--compile=no', '--threads=8,0',
            '--startup-file=no', '--history-file=no', '-O3']
        debugger.SetAsync(True)
        command = lldb.SBCommandReturnObject()
        debugger.GetCommandInterpreter().HandleCommand('settings set target.skip-prologue false', command)
        assert command.Succeeded(), command.GetError()
        target = debugger.CreateTarget(request['executable'])
        assert target.IsValid() and 'arm64' in target.GetTriple()
        # Install symbolic resolvers before launch. LLDB also resolves these
        # when a runtime component is dynamically loaded later.
        descriptors = request['hooks'] + [dict(kind='boundary', name=name,
            origin='direct-compiled-C-ABI') for name in ('main', 'init_julia', 'clbg_entry', 'shutdown_julia')]
        assert len({item['name'] for item in descriptors}) == len(descriptors)
        for descriptor in descriptors:
            breakpoint = target.BreakpointCreateByName(descriptor['name'])
            breakpoint.SetEnabled(True)
            breakpoint.SetOneShot(False)
            breakpoint.SetIgnoreCount(0)
            breakpoints[breakpoint.GetID()] = (descriptor, breakpoint)
        prelaunch = breakpoint_inventory(target, breakpoints)
        write_new(output / 'prelaunch-breakpoints.json', prelaunch)
        write_new(output / 'prelaunch-modules.json', modules(target))
        owned = request['suspended_child']
        assert owned['stopped_before_exec'] is owned['parent_retains_POSIX_child_ownership'] is True
        assert type(owned['pid']) is type(owned['parent_pid']) is int
        assert owned['pid'] > 0 and owned['parent_pid'] != owned['pid']
        assert owned['target_executable'] == request['executable']
        assert owned['arguments'] == request['arguments'] and owned['common_mutex_fd'] >= 64
        listener = lldb.SBListener('mech-julia-owned-child-observation')
        error = lldb.SBError()
        # This exact PID was just forked/stopped by our still-live outer parent;
        # attaching happens before its exec and before any target initializer.
        process = target.AttachToProcessWithID(listener, owned['pid'], error)
        assert error.Success() and process.IsValid(), error.GetCString()
        pid = process.GetProcessID()
        assert pid == owned['pid']
        write_new(output / 'owned-native.json', dict(owned,
            ownership='Outer wrapper retains exact direct fork child and bounded kill/reap even if LLDB detaches or blocks'))
        events = output / 'events.jsonl'
        began = time.monotonic()
        main_seen = init_seen = entry_seen = shutdown_seen = False
        hook_events = []
        last_stop = -1
        stop_count = 0
        with events.open('x') as stream:
            while process.GetState() not in (lldb.eStateExited, lldb.eStateDetached, lldb.eStateInvalid):
                assert time.monotonic() - began < 240
                if process.GetState() != lldb.eStateStopped or process.GetStopID() <= last_stop:
                    event = lldb.SBEvent()
                    if listener.WaitForEvent(1, event):
                        description = lldb.SBStream()
                        event.GetDescription(description)
                        stream.write(json.dumps(dict(lifecycle_event=description.GetData(),
                            state=int(process.GetState()), native_pid=pid)) + '\n')
                        stream.flush()
                    continue
                stop = process.GetStopID()
                last_stop = stop
                stop_count += 1
                assert stop_count <= 10000 and time.monotonic() - began < 240
                inventory = breakpoint_inventory(target, breakpoints)
                threads = []
                for thread in process:
                    reason = thread.GetStopReason()
                    hits = []
                    if reason == lldb.eStopReasonBreakpoint:
                        count = thread.GetStopReasonDataCount()
                        assert count > 0 and count % 2 == 0
                        for index in range(0, count, 2):
                            identifier = thread.GetStopReasonDataAtIndex(index)
                            location = thread.GetStopReasonDataAtIndex(index + 1)
                            assert identifier in breakpoints
                            descriptor = breakpoints[identifier][0]
                            hits.append(dict(breakpoint_id=identifier, location_id=location, descriptor=descriptor))
                            name = descriptor['name']
                            if name == 'main': main_seen = True
                            elif name == 'init_julia':
                                assert main_seen and not entry_seen
                                init_seen = True
                            elif name == 'clbg_entry':
                                assert main_seen and init_seen
                                entry_seen = True
                            elif name == 'shutdown_julia':
                                assert entry_seen
                                shutdown_seen = True
                            else:
                                hook_events.append(dict(stop_id=stop, thread_id=thread.GetThreadID(),
                                    descriptor=descriptor, phase='shutdown' if shutdown_seen else
                                    'entry' if entry_seen else 'initialization' if init_seen else 'before-main'))
                    elif reason not in (lldb.eStopReasonNone, lldb.eStopReasonInvalid):
                        assert not main_seen and (reason == lldb.eStopReasonExec or
                            (stop_count == 1 and reason == lldb.eStopReasonSignal)), ('Unexpected native stop', reason)
                    frames = [dict(index=index, function=thread.GetFrameAtIndex(index).GetFunctionName(),
                        **address_record(thread.GetFrameAtIndex(index).GetPCAddress(), target))
                        for index in range(min(thread.GetNumFrames(), 12))]
                    frame = thread.GetFrameAtIndex(0)
                    registers = {name: frame.FindRegister(name).GetValueAsUnsigned()
                        for name in ('x0', 'x1', 'x2', 'x3', 'x4', 'x5', 'x6', 'x7', 'pc', 'lr')}
                    threads.append(dict(thread_id=thread.GetThreadID(), stop_reason=int(reason), hits=hits,
                        frames=frames, arm64_entry_registers=registers))
                record = dict(stop_id=stop, native_pid=pid, threads=threads,
                    loaded_modules=modules(target), breakpoint_inventory=inventory)
                stream.write(json.dumps(record) + '\n')
                stream.flush()
                result = process.Continue()
                assert result.Success(), result.GetCString()
        assert process.GetState() == lldb.eStateExited, 'No detach or invalid-state success'
        assert main_seen and init_seen and entry_seen and shutdown_seen
        final_inventory = breakpoint_inventory(target, breakpoints)
        # The raw calibration is never an automatic AOT approval. Missing
        # addresses, extra routes, actual native method coverage and cleanup
        # still require the outer pinned observation/semantic review.
        report.update(status='raw-canary-observation-complete-awaiting-calibration-review',
            pid=pid, exit_status=process.GetExitStatus(), native_exit_description=process.GetExitDescription(),
            stop_count=stop_count, hooks_armed_before_launch=True,
            main_before_runtime_init=True, entry_and_shutdown_observed=True,
            hook_events=hook_events, final_breakpoints=final_inventory,
            pending_hooks=[item['descriptor']['name'] for item in final_inventory if not item['locations']],
            final_loaded_modules=modules(target))
    except BaseException as error:
        report.update(error=str(error) or type(error).__name__, error_type=type(error).__name__)
        (output / 'actual-error.log').write_text(traceback.format_exc())
        raise
    finally:
        if process is not None and process.IsValid() and process.GetState() not in (
                lldb.eStateExited, lldb.eStateDetached, lldb.eStateInvalid):
            # Only the exact parent-owned child is attached. Its outer parent
            # also kills/reaps independently on detach, invalid state or timeout.
            killed = process.Kill()
            report['owned_native_cleanup'] = dict(success=killed.Success(), error=killed.GetCString(),
                final_state=int(process.GetState()))
            if not killed.Success(): report['status'] = 'failed-owned-native-cleanup'
        if target is not None and target.IsValid():
            debugger.DeleteTarget(target)
        debugger.SetAsync(old_async)
        write_new(output / 'report.json', report)
