"""Read Darwin exit events for the already-running orphaned APL sample.

This control-plane observer sends no signal, starts no benchmark, attaches no
debugger and does not claim qualification. It must register while both saved
process identities are still alive; terminal evidence is additive. Target raw
results and source/manifest pins are reviewed later under the common mutex.
"""
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import re
import select
import subprocess
import sys
import traceback

HERE = Path(__file__).resolve().parent
OUT = HERE / 'jobs/apl-sample84-kqueue-observer-v1'
REPORT = HERE.parent / 'r-apl-uiua-comparison/20261011/uiua-fann-compact-v5/controller_rev2/apl-after-tree-failure-v1/full-owner/timings-apl.json'
HEADER = Path('/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk/usr/include/sys/event.h')
EXPECTED = {68659: 'Tue Oct 13 01:11:26 2026', 68666: 'Tue Oct 13 01:11:27 2026'}


def now():
    return datetime.now(timezone.utc).isoformat()


def save(path, value):
    with path.open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')
        stream.flush()
        os.fsync(stream.fileno())


def identity(pid):
    p = subprocess.run(['ps', '-p', str(pid), '-o', 'lstart=,command='],
                       capture_output=True, text=True, check=False)
    return p.stdout.strip() or None


def main():
    assert sys.platform == 'darwin'
    OUT.mkdir(parents=True)
    result = dict(status='failed-before-registration', observer_pid=os.getpid(),
                  started_at=now(), signals_sent=0, benchmarks_started=0,
                  debugger_attached=False, qualification_granted=False)
    queue = None
    raw_count = 0
    def retain_raw(event):
        nonlocal raw_count
        raw_count += 1
        path = OUT / ('raw-event-' + str(raw_count).zfill(4) + '.json')
        save(path, dict(observed_at=now(), ident=int(event.ident),
             filter=int(event.filter), flags=int(event.flags), fflags=int(event.fflags),
             data=int(event.data), qualification_granted=False))
        return str(path)
    try:
        body = HEADER.read_text()
        assert re.search(r'#define\s+NOTE_EXITSTATUS\s+0x04000000\b', body)
        assert re.search(r'#define\s+NOTE_EXIT\s+0x80000000\b', body)
        note_exitstatus = 0x04000000
        active = json.loads(REPORT.read_text())['active']
        assert active['attempt'] == 84 and active['sample'] == 2
        assert active['sampler_pid'] == 68659 and active['native']['pid'] == active['native']['pgid'] == 68666
        assert active['sampler_inherited_common_mutex'] is active['native']['inherited_common_mutex'] is True
        before = {pid: identity(pid) for pid in EXPECTED}
        for pid, birth in EXPECTED.items():
            assert before[pid] and before[pid].startswith(birth), ('Process birth changed', pid)
        assert 'apl-after-tree-failure-v1/controller.py' in before[68659]
        assert 'fannkuchredux.compact.apl -- 12' in before[68666]
        queue = select.kqueue()
        changes = [select.kevent(pid, filter=select.KQ_FILTER_PROC,
                    flags=select.KQ_EV_ADD | select.KQ_EV_ENABLE | select.KQ_EV_ONESHOT,
                    fflags=select.KQ_NOTE_EXIT | note_exitstatus) for pid in EXPECTED]
        queue.control(changes, 0, 0)
        after = {pid: identity(pid) for pid in EXPECTED}
        assert before == after, 'Identity changed while exit observer was registering'
        save(OUT / 'registered.json', dict(status='both-live-identities-registered',
             registered_at=now(), observer_pid=os.getpid(), identities=before,
             report=str(REPORT), saved_active=active, filter='EVFILT_PROC',
             note_exit=select.KQ_NOTE_EXIT, note_exitstatus=note_exitstatus,
             api_basis_header=str(HEADER),
             api_basis_primary_source='https://github.com/apple-oss-distributions/xnu/blob/main/bsd/kern/kern_event.c',
             signals_sent=0, benchmarks_started=0, debugger_attached=False,
             common_mutex_required_for_later_pin_review=True, qualification_granted=False))
        seen = {}
        while len(seen) < len(EXPECTED):
            events = queue.control(None, len(EXPECTED), 30)
            captured = [(event, retain_raw(event)) for event in events]
            for event, raw_path in captured:  # The entire one-shot batch is durable first.
                pid = int(event.ident)
                assert pid in EXPECTED and pid not in seen
                assert event.filter == select.KQ_FILTER_PROC
                assert not event.flags & select.KQ_EV_ERROR
                assert event.fflags & select.KQ_NOTE_EXIT and event.fflags & note_exitstatus
                status = int(event.data)
                record = dict(pid=pid, observed_at=now(), raw_event=raw_path, filter=int(event.filter),
                     flags=int(event.flags), fflags=int(event.fflags), kernel_wait_status=status,
                     exited_normally=os.WIFEXITED(status),
                     exit_code=os.WEXITSTATUS(status) if os.WIFEXITED(status) else None,
                     signalled=os.WIFSIGNALED(status),
                     signal_number=os.WTERMSIG(status) if os.WIFSIGNALED(status) else None,
                     qualification_granted=False)
                save(OUT / ('exit-' + str(pid) + '.json'), record)
                seen[pid] = record
        result.update(status='both-kernel-exit-statuses-captured-awaiting-raw-pin-review',
                      exit_events=seen, exit_status_origin='Darwin EVFILT_PROC NOTE_EXITSTATUS; not synthetic waitpid')
    except BaseException as error:
        result.update(error_type=type(error).__name__, error=str(error) or type(error).__name__)
        if queue is not None:
            try:
                result['failure_drain_raw_events'] = [retain_raw(event)
                    for event in queue.control(None, len(EXPECTED), 0)]
            except BaseException as drain_error:
                result['failure_drain_error'] = dict(error_type=type(drain_error).__name__,
                    error=str(drain_error) or type(drain_error).__name__)
        try:
            (OUT / 'failure.log').write_text(traceback.format_exc())
        except OSError:
            pass
        raise
    finally:
        if queue is not None:
            queue.close()
        result['finished_at'] = now()
        save(OUT / 'terminal.json', result)


if __name__ == '__main__':
    main()
