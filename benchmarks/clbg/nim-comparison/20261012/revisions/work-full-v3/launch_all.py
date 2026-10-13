"""Queue ten independently owned exact reviewed studies without waiting under the mutex."""
import argparse
import fcntl
import json
from pathlib import Path
import subprocess
import sys

import full_study


def launch(args):
    index_path = full_study.HERE / 'studies' / args.revision / 'index.json'
    print('WAIT common mutex: Nim exact reviewed full-suite launch', flush=True)
    with full_study.LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        assert full_study.sha(index_path) == args.index_sha256
        index = json.loads(index_path.read_text())
        assert index['status'] == 'sealed-awaiting-root-review' and index['samples'] == 3
        assert index['full_processes'] == 90 and set(index['benchmarks']) == set(full_study.BENCHMARKS)
        reviewed = []
        for benchmark in full_study.BENCHMARKS:
            guard = full_study.configure(benchmark, args.revision)
            entry = index['benchmarks'][benchmark]
            assert Path(entry['manifest']) == guard.OUT / 'manifest.json'
            full_study.review(guard, benchmark, entry['sha256'])
            assert not guard.REPORT.exists() and not (guard.OUT / 'launch.json').exists()
            assert not (guard.OUT / 'supervisor.log').exists()
            reviewed.append((benchmark, guard, entry['sha256']))
        receipts = []
        for benchmark, guard, digest in reviewed:
            with (guard.OUT / 'launch-owner.lock').open('a+') as owner:
                fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
                assert not guard.REPORT.exists() and not (guard.OUT / 'launch.json').exists()
                command = [sys.executable, '-u', '-B', str(Path(full_study.__file__).resolve()), '--run',
                    '--benchmark', benchmark, '--revision', args.revision, '--manifest-sha256', digest]
                with (guard.OUT / 'supervisor.log').open('xb') as log:
                    process = subprocess.Popen(command, stdin=subprocess.DEVNULL, stdout=log,
                        stderr=subprocess.STDOUT, start_new_session=True)
                receipt = dict(benchmark=benchmark, pid=process.pid, identity=guard.identity(process.pid),
                    command=command, launched_at=guard.now(), manifest_sha256=digest,
                    reviewed_index_sha256=args.index_sha256, shared_timing_lock=str(full_study.LOCK),
                    waiting_child_not_joined_under_mutex=True)
                with (guard.OUT / 'launch.json').open('x') as stream:
                    json.dump(receipt, stream, indent=2)
                    stream.write('\n')
                receipts.append(receipt)
        print(json.dumps(receipts, indent=2), flush=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--revision', required=True)
    parser.add_argument('--index-sha256', required=True)
    args = parser.parse_args()
    assert args.revision and all(c in 'abcdefghijklmnopqrstuvwxyz0123456789-' for c in args.revision)
    launch(args)


if __name__ == '__main__':
    main()
