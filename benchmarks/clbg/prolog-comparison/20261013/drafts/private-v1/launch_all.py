"""Unexecuted sequential full cohort controller; root uses durable launcher externally."""
import argparse
import fcntl
import json
from pathlib import Path

LOCK = Path('/tmp/mech-clbg-timing.lock')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--index', type=Path, required=True)
    parser.add_argument('--index-sha256', required=True)
    parser.add_argument('--run', action='store_true', required=True)
    args = parser.parse_args()
    print('WAIT common mutex: Prolog reviewed complete full index', flush=True)
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        import full_study
        assert full_study.sha(args.index) == args.index_sha256
        index = json.loads(args.index.read_text())
        assert index['status'] == 'sealed-awaiting-root-review'
        assert index['full_processes'] == 90 and index['samples'] == 3
        assert index['chart_admission'] is False
        assert set(index['benchmarks']) == set(full_study.BENCHMARKS)
        revision = args.index.resolve().parent.name
        for bench in full_study.BENCHMARKS:
            declared = index['benchmarks'][bench]
            guard = full_study.configure(bench, revision)
            assert Path(declared['manifest']) == guard.OUT / 'manifest.json'
            full_study.review(guard, bench, declared['sha256'])
    # Each full_study.run acquires/releases the same common mutex for every
    # native lifetime. Never hold it while waiting for a child that needs it.
    for bench in full_study.BENCHMARKS:
        full_study.run(bench, revision, index['benchmarks'][bench]['sha256'])


if __name__ == '__main__':
    main()
