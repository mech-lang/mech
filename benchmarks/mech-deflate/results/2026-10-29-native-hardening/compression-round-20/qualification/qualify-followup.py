from pathlib import Path
import subprocess
r=Path(__file__).resolve().parent
for script in ['object-audit/build.py','object-audit/sections.py','engine-units/build.py','primary/qualify-selected.py','engine-units/nonnative-check.py','parent-control/build.py']:
 print('START',script,flush=True)
 subprocess.run(['python3',str(r/script)],check=True,cwd='/private/tmp/mech-deflate-linear-stack')
print('COMPLETE NONPRIMARY FOLLOWUP',flush=True)
