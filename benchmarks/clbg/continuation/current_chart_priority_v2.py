"""Bounded publication scheduling: hold only owned waiting study supervisors.

The predecessor joined the same unfair mutex queue as thirty measurement
supervisors. This revision suspends childless owned waiters before joining it;
all native/sampler processes continue. Once the common lock is held, the last
completed supervisor is held too. Every identity resumes on release, exception,
interruption or a thirty-minute preflight/twenty-minute publication deadline.
"""
from datetime import datetime, timezone
import fcntl
import json
import os
from pathlib import Path
import signal
import subprocess
import time

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
LOCK = Path('/tmp/mech-clbg-timing.lock')
OUT = HERE/'jobs/current-chart-priority-v2'
OWNER_PIDS = (3954,3959,3966,3973,3980,3987,3994,4003,4008,4014,4155,4157,4159,4161,4163,4165,4167,4169,4171,4173,11310,11312,11314,11316,11318,11320,11322,11324,11326,11328)
PREFIXES = tuple(str(ROOT/language/'20261012/revisions'/version/'full_study.py') for language,version in (
    ('clojure-comparison','work-full-v1'),('gleam-comparison','work-full-v4'),('nim-comparison','work-full-v5')))

def listing():
    result = subprocess.run(['/bin/ps','-Ao','pid=,ppid=,lstart=,command='],capture_output=True,text=True,check=True)
    rows={}
    for line in result.stdout.splitlines():
        fields=line.strip().split(None,7)
        if len(fields)==8:
            rows[int(fields[0])]={'pid':int(fields[0]),'ppid':int(fields[1]),'birth':' '.join(fields[2:7]),'command':fields[7]}
    return rows

def matches(before,after):
    return after and all(before[key]==after[key] for key in ('pid','birth','command'))

def journal(name,value):
    with (OUT/name).open('x') as stream: json.dump(value,stream,indent=2); stream.write('\n')

def now(): return datetime.now(timezone.utc).isoformat()

def eligible(row):
    return any(prefix+' --run --benchmark ' in row['command'] for prefix in PREFIXES)

paused=[]
signal.signal(signal.SIGHUP,signal.SIG_IGN)
def interrupted(signum,frame): raise InterruptedError('Bounded publication scheduling interrupted or expired')
for signum in (signal.SIGTERM,signal.SIGINT,signal.SIGALRM): signal.signal(signum,interrupted)

def hold_waiter(row):
    rows=listing(); current=rows.get(row['pid'])
    if not matches(row,current): return False
    if any(child['ppid']==row['pid'] for child in rows.values()): return False
    os.kill(row['pid'],signal.SIGSTOP); paused.append(row)
    # A child created at the snapshot boundary must continue with its original
    # supervisor; no sampler/native child is ever signalled by this process.
    if any(child['ppid']==row['pid'] for child in listing().values()):
        os.kill(row['pid'],signal.SIGCONT); paused.remove(row); return False
    return True

try:
    assert not OUT.exists(); OUT.mkdir(parents=True)
    signal.alarm(1800)
    rows=listing(); selected=[]
    for pid in OWNER_PIDS:
        row=rows.get(pid)
        if row:
            assert eligible(row), ('Changed owned study identity',row)
            selected.append(row)
    for row in selected: hold_waiter(row)
    journal('waiting-supervisors.json',dict(status='owned-childless-waiters-held-for-publication',started_at=now(),
        held_supervisors=paused,active_supervisors_left_running=[row for row in selected if row not in paused],
        native_samples_interrupted=0,common_mutex_held=False,preflight_deadline_seconds=1800))
    print('WAIT common mutex: queued childless timing supervisors held; all active samples continue',flush=True)
    with LOCK.open('a+') as lock:
        fcntl.flock(lock,fcntl.LOCK_EX)
        held,named=os.fstat(lock.fileno()),LOCK.stat()
        assert (held.st_dev,held.st_ino)==(named.st_dev,named.st_ino)
        for row in selected:
            if row not in paused:
                current=listing().get(row['pid'])
                if matches(row,current): assert hold_waiter(row), ('Completed supervisor still collecting child',row['pid'])
        journal('window.json',dict(status='publication-priority-window-open',started_at=now(),held_supervisors=paused,
            common_mutex_held_during_final_suspension=True,native_samples_interrupted=0,maximum_window_seconds=1200))
        print('PUBLICATION PRIORITY WINDOW OPEN: owned waiters held; native samples interrupted=0',flush=True)
    signal.alarm(0)
    deadline=time.monotonic()+1200
    while time.monotonic()<deadline and not (OUT/'release').exists(): time.sleep(1)
finally:
    signal.alarm(0)
    resumed=[]; resume_errors=[]
    for row in paused:
        try:
            after=listing().get(row['pid'])
            if matches(row,after):
                os.kill(row['pid'],signal.SIGCONT); resumed.append(row['pid'])
        except ProcessLookupError: pass
        except BaseException as error: resume_errors.append(dict(pid=row['pid'],error=str(error)))
    if OUT.exists():
        journal('resumed.json',dict(status='publication-window-closed-owned-waiters-resumed' if not resume_errors else 'resume-needs-attention',
            finished_at=now(),resumed_pids=resumed,errors=resume_errors,native_samples_interrupted=0))
    print('PUBLICATION PRIORITY WINDOW CLOSED: owned waiters resumed',flush=True)
    assert not resume_errors, resume_errors
