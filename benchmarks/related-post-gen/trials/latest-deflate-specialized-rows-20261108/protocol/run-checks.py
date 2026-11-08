from pathlib import Path
import subprocess,json,time
P=Path(__file__).parent
for label,script in [
    ('native-tests','/private/tmp/mech-related-latest-deflate-tests-20261108-v2/run.py'),
    ('runner-qualification','/private/tmp/mech-related-latest-deflate-runners-20261108-v1/build-and-qualify.py'),
    ('scalar-deflate','/private/tmp/mech-related-latest-deflate-regressions-20261108-v1/run.py'),
]:
    t=time.monotonic()
    r=subprocess.run(['python3',script],capture_output=True,text=True)
    (P/(label+'.json')).write_text(json.dumps(dict(command=['python3',script],exit=r.returncode,seconds=time.monotonic()-t,stdout=r.stdout,stderr=r.stderr),indent=2)+'\n')
    print(label,r.returncode,r.stdout[-2500:],flush=True)
    if r.returncode:
        print(r.stderr[-5000:],flush=True)
        raise SystemExit(r.returncode)
