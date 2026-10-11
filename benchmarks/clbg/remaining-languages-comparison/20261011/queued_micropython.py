"""Start the slower interpreted cohort only after the native/JIT cohort exits."""
import json
import subprocess
import sys
import time
from pathlib import Path

raise SystemExit('MicroPython was removed from this comparison by the user; this queue is disabled.')

D=Path(__file__).resolve().parent
report=D/'results/native-timings-v2.json'
while True:
    if report.exists() and json.loads(report.read_text()).get('status')=='complete':break
    time.sleep(10)
# The native runner writes its terminal report after its final pin checks.
# Its only remaining action is printing that result and exiting.
time.sleep(1)
with (D/'results/micropython-timing.log').open('a') as log:
    subprocess.run([sys.executable,str(D/'micropython_heap_preflight.py')],
        cwd=D,stdout=log,stderr=subprocess.STDOUT,check=True)
    status=subprocess.call([sys.executable,str(D/'run.py'),'--languages','micropython',
        '--audit','micropython-source-audit.json','--report','micropython-timings-v2.json'],
        cwd=D,stdout=log,stderr=subprocess.STDOUT)
raise SystemExit(status)
