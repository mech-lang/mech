"""Run the changed selector checks while native benchmark trials are paused."""
import fcntl
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import time
import urllib.request
from datetime import datetime, timezone

HERE=Path(__file__).resolve().parent
NODE='/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin/node'
PW='/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright'
ASSETS=('dist/app.mjs','dist/traits.mjs','dist/index.html','dist/style.css','dist/data.json','test_traits.mjs','verify_traits.cjs','qualify_trait_ui.py')
def pins():
    return {name:hashlib.sha256((HERE/name).read_bytes()).hexdigest() for name in ASSETS}
def main():
    print('WAIT common benchmark mutex: trait selector UI qualification',flush=True)
    with open('/tmp/mech-clbg-timing.lock','a+') as lock:
        fcntl.flock(lock,fcntl.LOCK_EX)
        before=pins()
        subprocess.run([NODE,'--check','dist/app.mjs'],cwd=HERE,check=True)
        subprocess.run([NODE,'--check','dist/traits.mjs'],cwd=HERE,check=True)
        subprocess.run([NODE,'test_traits.mjs'],cwd=HERE,check=True)
        with (HERE/'trait-filters-preview.log').open('w') as log:
            server=subprocess.Popen([sys.executable,'-m','http.server','8766','--bind','127.0.0.1','--directory',str(HERE/'dist')],stdout=log,stderr=log,start_new_session=True)
            try:
                for attempt in range(50):
                    assert server.poll() is None,'The owned preview server exited before readiness'
                    try:
                        with urllib.request.urlopen('http://127.0.0.1:8766/',timeout=1) as response:
                            assert response.status==200
                        break
                    except OSError:
                        time.sleep(.1)
                else:
                    raise RuntimeError('The owned preview server did not become ready')
                environment=dict(os.environ,PLAYWRIGHT_MODULE=PW)
                subprocess.run([NODE,'verify_traits.cjs'],cwd=HERE,env=environment,check=True,timeout=180)
                assert before==pins(),'Qualification assets changed during the checks'
            finally:
                if server.poll() is None:
                    os.killpg(server.pid,signal.SIGTERM)
                server.wait(timeout=10)
        (HERE/'trait-filters-qualification.json').write_text(json.dumps(dict(passed=True,finished_at=datetime.now(timezone.utc).isoformat(),pins=before),indent=2)+'\n')
        print('Trait selector qualification passed; preview server closed.',flush=True)
if __name__=='__main__':
    main()
