"""Wait for native measurement to exit, then compile, audit and measure its repair."""
import json
import os
import subprocess
import sys
import time
from pathlib import Path

D = Path(__file__).resolve().parent
report = D/'results/native-timings-v2.json'
native_pid = int(sys.argv[1])
print('WAITING for the native timing batch to finish and exit.',flush=True)
while True:
    current = json.loads(report.read_text())
    try:
        command = subprocess.check_output(['ps','-p',str(native_pid),'-o','command='],text=True).strip()
    except subprocess.CalledProcessError:
        command = ''
    native_alive = str(D/'run.py') in command or ('run.py' in command and 'native-timings-v2.json' in command)
    if current['status']=='complete' and not current.get('active') and not native_alive:
        break
    if not native_alive and current['status']!='complete':
        raise SystemExit('Native batch stopped before completion; repair remains deferred.')
    time.sleep(5)

env = os.environ.copy()
env['PYTHONPATH'] = str(D/'support/python')
for command in (
    [sys.executable,str(D/'prepare_ada_regex_heap.py')],
    [sys.executable,str(D/'run_ada_regex_repair.py'),'--audit','ada-regex-repair-source-audit.json',
        '--report','ada-regex-repair.json','--benchmarks','regexredux'],
    [sys.executable,str(D/'summarize.py')],
):
    print('STEP',Path(command[1]).name,flush=True)
    subprocess.run(command,cwd=D,env=env,check=True)
print('COMPLETE: corrected full-workload timings and summary.',flush=True)
