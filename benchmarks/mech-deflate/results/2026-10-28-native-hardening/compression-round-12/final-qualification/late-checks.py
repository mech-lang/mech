import subprocess
commands=[['cargo','fmt','--all','--check'],['git','diff','--check'],['sh','scripts/check-unsafe-boundaries.sh'],['python3','scripts/check-warning-policy.py'],['python3','scripts/check-rust-module-layout.py']]
for c in commands:
 print('COMMAND',c,flush=True)
 subprocess.run(c,check=True)
