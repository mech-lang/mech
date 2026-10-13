"""Give a requested chart publication a bounded window between measurements.

Only known owned full-study supervisors with no descendants are suspended,
while the common timing mutex is held. No native sample is interrupted. Every
suspended identity is resumed in finally, or on the bounded window deadline.
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
OUT = HERE/'jobs/current-chart-priority-v1'
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

def identity_matches(before,after):
    return after and all(before[key]==after[key] for key in ('pid','birth','command'))

def journal(name,value):
    with (OUT/name).open('x') as stream: json.dump(value,stream,indent=2); stream.write('\n')

def now(): return datetime.now(timezone.utc).isoformat()

paused=[]
signal.signal(signal.SIGHUP,signal.SIG_IGN)
def interrupted(signum,frame): raise InterruptedError('Publication priority window interrupted')
for signum in (signal.SIGTERM,signal.SIGINT): signal.signal(signum,interrupted)
print('WAIT common mutex: publication priority between full timing samples',flush=True)
try:
    with LOCK.open('a+') as lock:
        fcntl.flock(lock,fcntl.LOCK_EX)
        held,named=os.fstat(lock.fileno()),LOCK.stat()
        assert (held.st_dev,held.st_ino)==(named.st_dev,named.st_ino)
        assert not OUT.exists(); OUT.mkdir(parents=True)
        rows=listing()
        selected=[]
        for pid in OWNER_PIDS:
            row=rows.get(pid)
            if not row: continue
            assert any(prefix+' --run --benchmark ' in row['command'] for prefix in PREFIXES), ('Changed owned supervisor',row)
            assert not any(child['ppid']==pid for child in rows.values()), ('Owned supervisor still collecting a child',pid)
            selected.append(row)
        # All selected roots are blocked on the mutex held by this process;
        # no timed child can start between identity checks and suspension.
        for row in selected:
            assert identity_matches(row,listing().get(row['pid']))
            os.kill(row['pid'],signal.SIGSTOP); paused.append(row)
        journal('window.json',dict(status='publication-priority-window-open',started_at=now(),suspended_owned_waiting_supervisors=paused,
            common_mutex_held_during_suspension=True,native_samples_interrupted=0,maximum_window_seconds=1200))
        print('PUBLICATION PRIORITY WINDOW OPEN: waiting supervisors paused; native samples interrupted=0',flush=True)
    deadline=time.monotonic()+1200
    while time.monotonic()<deadline and not (OUT/'release').exists(): time.sleep(1)
finally:
    resumed=[]
    for row in paused:
        try:
            after=listing().get(row['pid'])
            if identity_matches(row,after):
                os.kill(row['pid'],signal.SIGCONT); resumed.append(row['pid'])
        except ProcessLookupError: pass
    if OUT.exists():
        journal('resumed.json',dict(status='publication-window-closed-owned-waiters-resumed',finished_at=now(),resumed_pids=resumed,
            known_exited_pids=[row['pid'] for row in paused if row['pid'] not in resumed],native_samples_interrupted=0))
    print('PUBLICATION PRIORITY WINDOW CLOSED: owned waiters resumed',flush=True)
