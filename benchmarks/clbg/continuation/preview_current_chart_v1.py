"""Preview qualified staged chart while retaining the common timing mutex."""
import fcntl
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import threading
import urllib.request

HERE=Path(__file__).resolve().parent
JOB=HERE/'owned-jobs/current-crystal-chart-stage-v1'
SITE=Path('/Users/cim310/.codex/visualizations/2026/10/08/01a11c4b-7bf0-75b0-a854-cf4f0e614617/benchmark-explorer-site')
LOCK=Path('/tmp/mech-clbg-timing.lock')
print('WAIT staged chart owner lease: local preview',flush=True)
with (JOB/'owner.lock').open('a+') as owner:
    fcntl.flock(owner,fcntl.LOCK_EX)
    held,named=os.fstat(owner.fileno()),(JOB/'owner.lock').stat()
    assert (held.st_dev,held.st_ino)==(named.st_dev,named.st_ino)
    terminal=json.loads((JOB/'terminal.json').read_bytes())
    assert terminal['exit_code']==0 and terminal['status']=='command-exited-zero-awaiting-report-review'
    assert terminal['stop_signals']==[] and not any(key in terminal for key in ('error','error_type','exception_cleanup'))
print('WAIT common mutex: local chart preview and browser QA',flush=True)
with LOCK.open('a+') as lock:
    fcntl.flock(lock,fcntl.LOCK_EX)
    held,named=os.fstat(lock.fileno()),LOCK.stat()
    assert (held.st_dev,held.st_ino)==(named.st_dev,named.st_ino)
    stage=json.loads((HERE/'jobs/current-crystal-chart-stage-v1/receipt.json').read_bytes())
    assert stage['status']=='verified-qualified-crystal-staged-awaiting-browser-and-publication'
    handler=partial(SimpleHTTPRequestHandler,directory=str(SITE/'dist'))
    server=ThreadingHTTPServer(('127.0.0.1',0),handler)
    thread=threading.Thread(target=server.serve_forever,daemon=True)
    thread.start()
    url='http://127.0.0.1:'+str(server.server_port)+'/'
    try:
        with urllib.request.urlopen(url,timeout=10) as response:
            assert response.status==200 and b'Benchmark' in response.read()
        print('CURRENT CHART PREVIEW READY '+url,flush=True)
        print('Common timing mutex held for browser QA; send done to stop preview and release.',flush=True)
        while True:
            text=input().strip()
            if text=='done':break
    finally:
        server.shutdown();server.server_close();thread.join(timeout=5)
        print('Owned preview stopped; common timing mutex released.',flush=True)
