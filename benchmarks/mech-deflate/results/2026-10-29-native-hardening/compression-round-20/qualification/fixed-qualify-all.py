from pathlib import Path
import subprocess
r=Path(__file__).resolve().parent
for script in ['fixed-full-components.py','fixed-after/build-witness.py','fixed-after/build-audit.py','fixed-object-audit/build.py','fixed-object-audit/sections.py','fixed-engine-units/build.py','final-primary/qualify-selected.py','fixed-engine-units/nonnative-check.py']:
 print('START',script,flush=True)
 subprocess.run(['python3',str(r/script)],check=True,cwd='/private/tmp/mech-deflate-linear-stack')
print('COMPLETE FIXED NONPRIMARY QUALIFICATION',flush=True)
