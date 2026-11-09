from pathlib import Path
import subprocess
p=Path(__file__).parent
for n in ['build-profile-runtime.py','build-profile-hosts.py','run-profiles.py']:
 r=subprocess.run(['python3',str(p/n)]);assert r.returncode==0,n
