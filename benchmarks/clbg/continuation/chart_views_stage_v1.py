"""Check and stage UI-only chart updates during the root's held QA session."""
import argparse
import datetime
import fcntl
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / 'overview/explorer/median-rankings-v1'
WEBSITE = Path('/Users/cim310/mech-lang/website')
DEST = WEBSITE / 'public/benchmarks'
JOB = ROOT / 'continuation/jobs/chart-views-publication-v1'
NODE = '/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin/node'
DATA_SHA = '4fe402c21360fbbbe9d650c15d1fd4c62422c17cfad8cafd24721fe7f0bd2947'
CHANGED = ('app.mjs', 'math.mjs', 'index.html', 'style.css')


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--iteration', required=True, type=int)
    args = parser.parse_args()
    assert args.iteration > 0
    output = JOB / f'stage-{args.iteration}.json'
    assert not output.exists(), 'Preserve the previous stage record.'
    session = json.loads((JOB / 'session.json').read_text())
    os.kill(session['pid'], 0)
    assert not (JOB / 'session-closed.json').exists()
    with open('/tmp/mech-clbg-timing.lock', 'r+') as lock:
        assert os.fstat(lock.fileno()).st_ino == session['inode'] == 79859134
        try:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            pass
        else:
            raise AssertionError('Root QA session no longer holds the common mutex.')
    receipt = dict(status='checking', at=datetime.datetime.now(datetime.timezone.utc).isoformat(),
                   common_session_pid=session['pid'], benchmark_runs=0, checks=[])
    try:
        assert sha(DEST / 'data.json') == sha(ROOT / 'overview/explorer/dist/data.json') == DATA_SHA
        for name in ('traits.mjs', 'mech-palette.css'):
            assert (SOURCE / name).read_bytes() == (DEST / name).read_bytes()
        files = sorted(path for path in SOURCE.iterdir() if path.is_file())
        receipt['source_pins'] = {path.name: sha(path) for path in files}
        tests = sorted(SOURCE.glob('test_*.mjs'))
        assert tests, 'Focused statistical assertions are required.'
        commands = [[NODE, '--check', str(SOURCE / name)] for name in ('app.mjs', 'math.mjs')]
        commands += [[NODE, str(path)] for path in tests]
        for command in commands:
            run = subprocess.run(command, cwd=ROOT, text=True, capture_output=True)
            receipt['checks'].append(dict(command=command, returncode=run.returncode,
                                          stdout=run.stdout, stderr=run.stderr))
            assert run.returncode == 0, f'UI check failed: {command}'
        assert receipt['source_pins'] == {path.name: sha(path) for path in files}
        receipt['previous_assets'] = {name: sha(DEST / name) for name in CHANGED}
        for name in CHANGED:
            shutil.copyfile(SOURCE / name, DEST / name)
        assert sha(DEST / 'data.json') == DATA_SHA
        receipt.update(status='staged-for-browser-QA', dataset_sha256=DATA_SHA,
                       assets={path.name: sha(path) for path in sorted(DEST.iterdir()) if path.is_file()})
    except BaseException as error:
        receipt.update(status='failed', error=str(error), error_type=type(error).__name__)
        raise
    finally:
        output.write_text(json.dumps(receipt, indent=2) + '\n')
        print(json.dumps({key: receipt[key] for key in ('status', 'checks', 'error') if key in receipt}), flush=True)


if __name__ == '__main__':
    main()
