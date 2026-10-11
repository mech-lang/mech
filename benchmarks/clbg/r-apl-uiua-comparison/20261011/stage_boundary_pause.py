"""Pause only a waiting coordinator after named full benchmarks finish."""
import argparse
import fcntl
import json
import os
import signal
import subprocess
import time
from datetime import datetime, timezone
from pathlib import Path
from prepare import G, LOCK, save, sha


def now():
    return datetime.now(timezone.utc).isoformat()


def processes():
    result = {}
    for line in subprocess.check_output(['ps', '-axo', 'pid,ppid,command'], text=True).splitlines()[1:]:
        fields = line.split(None, 2)
        if len(fields) == 3:
            result[int(fields[0])] = {'ppid': int(fields[1]), 'command': fields[2]}
    return result


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--coordinator-pid', type=int, required=True)
    p.add_argument('--languages', default='r,apl,uiua')
    p.add_argument('--benchmarks', required=True)
    p.add_argument('--expected-arguments', required=True)
    p.add_argument('--output', required=True)
    p.add_argument('--after-key', help='Pause after the first fully persisted full attempt for this key, rather than completing named benchmarks.')
    args = p.parse_args()
    languages = args.languages.split(',')
    benchmarks = args.benchmarks.split(',')
    record = dict(reason='Root-approved pause between full attempts for reviewed '
                  'preparation and explicit external-maintenance correction. '
                  'No active benchmark attempt is interrupted.', requested_at=now(),
                  coordinator_pid=args.coordinator_pid, benchmarks=benchmarks)
    if args.after_key:
        record['after_first_persisted_full_attempt_key'] = args.after_key
    print('WAIT APPROVED STAGE BOUNDARY', benchmarks, flush=True)
    while True:
        with LOCK.open('a+') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            snapshots = {}
            complete = True
            boundary_attempt = None
            for language in languages:
                path = G / 'results' / ('timings-' + language + '.json')
                report = json.loads(path.read_text())
                if args.after_key:
                    matching = [a for a in report.get('attempts', [])
                                if a.get('key') == args.after_key and a.get('full_workload')
                                and a.get('finished_at')]
                    if matching and not report.get('active'):
                        boundary_attempt = matching[-1]
                rows = [r for r in report['programs'].values() if r['benchmark'] in benchmarks]
                done = all(len(r.get('samples_seconds', [])) >= report['sample_count']
                           or r.get('full_failure') for r in rows)
                complete = complete and done
                snapshots[language] = {'report': str(path), 'report_sha256': sha(path),
                                       'complete': done, 'keys': [r['key'] for r in rows],
                                       'samples': {r['key']: r.get('samples_seconds', []) for r in rows}}
            listing = processes()
            coordinator = listing.get(args.coordinator_pid)
            ready = boundary_attempt is not None if args.after_key else complete
            if ready or coordinator is None:
                record.update(lock_acquired_at=now(), snapshots=snapshots,
                              requested_benchmarks_complete=complete)
                if boundary_attempt:
                    record['persisted_boundary_attempt'] = boundary_attempt
                if coordinator:
                    assert 'run.py ' in coordinator['command']
                    assert args.expected_arguments in coordinator['command'], coordinator
                    children = {pid: row for pid, row in listing.items()
                                if row['ppid'] == args.coordinator_pid}
                    assert not children, ('Active coordinator children, refusing to signal', children)
                    record['coordinator_command'] = coordinator['command']
                    os.kill(args.coordinator_pid, signal.SIGINT)
                    record['signal'] = 'SIGINT'
                    deadline = time.monotonic() + 5
                    while args.coordinator_pid in processes():
                        assert time.monotonic() < deadline, 'Coordinator did not stop at boundary'
                        time.sleep(.02)
                else:
                    record['coordinator_already_exited'] = True
                record['finished_at'] = now()
                save(Path(args.output).resolve(), record)
                print('PAUSED SAFE STAGE BOUNDARY', record['requested_benchmarks_complete'], flush=True)
                return
        # Join the lock queue for the next complete attempt without repeatedly
        # acquiring it during a coordinator's short unlocked bookkeeping gap.
        time.sleep(1)


if __name__ == '__main__':
    main()
