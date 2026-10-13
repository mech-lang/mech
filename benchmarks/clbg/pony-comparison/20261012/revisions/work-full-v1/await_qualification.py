"""Metadata-only prerequisite watcher for one reviewed work-manifest seal.

This process never holds the common timing mutex, imports the guarded runner,
hashes/lexes sources or launches native benchmarks. After exact private-v6
success it starts one metadata seal whose own common-lock checks are mandatory.
The emitted manifest still requires root review before any native work run.
"""
import argparse
from datetime import datetime, timezone
import fcntl
import json
import os
from pathlib import Path
import subprocess
import sys
import time

HERE = Path(__file__).resolve().parent
COHORT = HERE.parents[1]
PASSED = 'bounded-output-primitives-passed-awaiting-work-worker-and-LOC-review'
PROPOSAL = 'ecf8bf803cfae8860309447db518174fc4d991f10bfc5d38cc77c8fb8e246cde'


def now():
    return datetime.now(timezone.utc).isoformat()


def journal(job, status, **fields):
    value = dict(status=status, updated_at=now(), watcher_pid=os.getpid(),
                 native_processes_launched=0, automatic_retry=False,
                 native_work_review_required=True, full_timing_review_required=True, **fields)
    temp = job / 'state.json.tmp'
    temp.write_text(json.dumps(value, indent=2) + '\n')
    temp.replace(job / 'state.json')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--job', type=Path, required=True)
    args = parser.parse_args()
    job = args.job.resolve()
    job.mkdir(parents=True, exist_ok=True)
    with (job / 'watcher-owner.lock').open('a+') as owner:
        fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
        if (job / 'state.json').exists():
            raise RuntimeError('Preserved watcher must never be resumed or relaunched.')
        prerequisite = COHORT / 'qualification/private-v6/report.json'
        journal(job, 'waiting-for-reviewed-native-prerequisite', prerequisite=str(prerequisite),
                prerequisite_proposal_sha256=PROPOSAL, common_mutex_held_by_watcher=False)
        print('WAIT PREREQUISITE: exact private-v6 native success, metadata only', flush=True)
        started = time.monotonic()
        while True:
            if prerequisite.exists():
                report = json.loads(prerequisite.read_text())
                if report.get('proposal_sha256') != PROPOSAL:
                    journal(job, 'stopped-prerequisite-identity-failure', prerequisite=str(prerequisite))
                    return 1
                status = report.get('status')
                if status == PASSED:
                    break
                if status != 'preparing':
                    journal(job, 'stopped-terminal-prerequisite-failure', prerequisite=str(prerequisite),
                            prerequisite_status=status, prerequisite_error=report.get('error'))
                    return 1
            if time.monotonic() - started > 14400:
                journal(job, 'stopped-prerequisite-wait-time-limit', prerequisite=str(prerequisite),
                        native_prerequisite_was_not_retried=True)
                return 1
            time.sleep(5)
        command = [sys.executable, '-B', '-u', str(HERE / 'work_qualify.py'),
                   '--seal', '--revision', 'work-v1', '--qualification', 'private-v6']
        journal(job, 'starting-reviewed-metadata-seal', prerequisite=str(prerequisite),
                prerequisite_status=PASSED, command=command, common_mutex_held_by_watcher=False)
        with (job / 'seal.stdout.log').open('xb') as output:
            process = subprocess.Popen(command, stdin=subprocess.DEVNULL, stdout=output,
                stderr=subprocess.STDOUT, start_new_session=True,
                env=dict(os.environ, PYTHONDONTWRITEBYTECODE='1'))
        with (job / 'seal-launch.json').open('x') as stream:
            json.dump(dict(pid=process.pid, pgid=process.pid, command=command,
                launched_at=now(), preparation_only=True, native_processes_launched=0,
                automatic_retry=False, native_work_review_required=True), stream, indent=2)
            stream.write('\n')
        # This watcher owns no benchmark mutex while the metadata child waits
        # for it. Child failure remains one preserved attempt with no retry.
        status = process.wait()
        journal(job, 'metadata-sealed-awaiting-root-native-review' if status == 0 else 'stopped-metadata-seal-failure',
                seal_exit_status=status, seal_pid=process.pid,
                seal_stdout=str(job / 'seal.stdout.log'),
                manifest=str(COHORT / 'work-qualification/work-v1/manifest.json'))
        return 0 if status == 0 else 1


if __name__ == '__main__':
    raise SystemExit(main())
