"""Detached orchestration of unchanged, frozen full benchmark stages.

This file owns scheduling and report leases only. It supplies no benchmark
implementation, measurement, output validator, or extrapolated chart data.
"""
import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import time
from datetime import datetime, timezone

ROOT = Path(__file__).resolve().parent
STATE = ROOT / 'results' / 'coordinator-v4'
LOCK = Path('/tmp/mech-clbg-timing.lock')
EVIDENCE = ROOT / 'evidence' / 'coordinator-loss-20261011-v4'
AUDITS = {
    'r': 'd76be20504617addb9efd5915bf7074877e727e3a49ccbd36585b4be5c763af3',
    'uiua': '82b15648d0c49c6393a0eec1efb441f1ee11237ceae4a8f41ca854c0b7ee0860',
    'apl': '9b26c06182bb97b4d3e26d70d9edd6e5ec78e57f1b59a77a91874869c9e83936',
}
STAGES = [
    ('knucleotide-r-uiua', ['--languages', 'r,uiua', '--benchmarks', 'knucleotide', '--samples', '3']),
    ('long-r-uiua', ['--languages', 'r,uiua', '--benchmarks', 'binarytrees,nbody,fannkuchredux', '--pilot', '--samples', '3']),
    ('medium-apl', ['--languages', 'apl', '--benchmarks', 'mandelbrot,fasta,knucleotide', '--samples', '3']),
    ('long-apl', ['--languages', 'apl', '--benchmarks', 'binarytrees,nbody,fannkuchredux', '--pilot', '--samples', '3']),
]


def now():
    return datetime.now(timezone.utc).isoformat()


def save(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + '.tmp')
    temporary.write_text(json.dumps(value, indent=2) + '\n')
    temporary.replace(path)


def process_identity(pid):
    def field(name):
        result = subprocess.run(['ps', '-p', str(pid), '-o', name + '='], capture_output=True, text=True)
        return result.stdout.strip() if result.returncode == 0 else None
    started, command = field('lstart'), field('command')
    return {'pid': pid, 'started': started, 'command': command} if started and command else None


def read_report(language):
    return json.loads((ROOT / 'results' / ('timings-' + language + '.json')).read_text())


def reject_other_report_writers():
    listing = subprocess.check_output(['ps', '-axo', 'pid,command'], text=True)
    for line in listing.splitlines()[1:]:
        fields = line.strip().split(None, 1)
        if len(fields) != 2 or int(fields[0]) == os.getpid():
            continue
        command = fields[1]
        if 'run.py' not in command or '--languages' not in command:
            continue
        # Ignore shells and process-list searches; inspect only Python owners.
        executable = Path(command.split(None, 1)[0]).name.lower()
        if not executable.startswith('python'):
            continue
        if str(ROOT) in command or 'r-apl-uiua-comparison/20261011' in command:
            raise RuntimeError('Existing report writer must finish before continuation: ' + line)
        cwd = subprocess.run(['lsof', '-a', '-p', fields[0], '-d', 'cwd', '-Fn'],
                             capture_output=True, text=True).stdout
        if 'n' + str(ROOT) + '\n' in cwd:
            raise RuntimeError('Existing report writer must finish before continuation: ' + line)


def verify_audits():
    for language, expected in AUDITS.items():
        path = ROOT / ('source-audit-' + language + '.json')
        assert hashlib.sha256(path.read_bytes()).hexdigest() == expected, ('Frozen audit changed', language)
        assert read_report(language)['source_audit_sha256'] == expected


def run_stage(stage):
    # open_report verifies large pinned inputs/runtime files before its first
    # sample. Serialize that initialization too, without editing the harness.
    import run
    original = run.open_report

    def locked_open_report(*args, **kwargs):
        with LOCK.open('a+') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            return original(*args, **kwargs)

    run.open_report = locked_open_report
    sys.argv = [str(ROOT / 'run.py'), *dict(STAGES)[stage]]
    run.main()


def reconcile_orphan(identity):
    path = ROOT / 'results' / 'timings-apl.json'
    report = read_report('apl')
    archived = json.loads((EVIDENCE / 'event.json').read_text())
    historical = json.loads((EVIDENCE / 'timings-apl.json').read_text())
    assert report['attempts'][:len(historical['attempts'])] == historical['attempts'], 'Historical attempts changed'
    active = report.get('active')
    expected = archived['stale_active']
    applied = [event for event in report.get('coordinator_loss_events', [])
               if event.get('event_id') == archived['event_id']]
    if applied:
        assert len(applied) == 1 and applied[0]['stale_active'] == expected
        assert applied[0]['never_chart_eligible'] is True and applied[0]['attempt_number'] is None
        assert active is None, ('Another active record must be reconciled before resuming', active)
        print('RECONCILE ALREADY APPLIED', archived['event_id'], flush=True)
        return
    cleanup = json.loads((EVIDENCE / 'cleanup.json').read_text())
    assert cleanup['group_exit_confirmed'] is True and cleanup['remaining_group_members'] == []
    assert active == expected, ('Unexpected active report ownership', active, expected)
    before = path.read_bytes()
    (EVIDENCE / 'timings-apl.before-reconcile.json').write_bytes(before)
    event = dict(archived, reconciled_at=now(), observed_process_identity=identity,
                 exit_observed_at=cleanup['confirmed_at'], exit_status=None, seconds=None,
                 cleanup_record=str(EVIDENCE / 'cleanup.json'), cleanup_status='group-exit-confirmed',
                 trial_continues_uninterrupted=False,
                 disposition='Coordinator-loss event; no sample, fabricated timeout, or source failure recorded.',
                 prior_report_sha256=hashlib.sha256(before).hexdigest())
    report.setdefault('coordinator_loss_events', []).append(event)
    report.pop('active', None)
    report['status'] = 'partial'
    report['updated_at'] = now()
    save(path, report)
    save(EVIDENCE / 'reconciled-event.json', event)
    after = path.read_bytes()
    (EVIDENCE / 'timings-apl.after-reconcile.json').write_bytes(after)
    result = {'event_id': event['event_id'], 'reconciled_at': event['reconciled_at'],
              'prior_report_sha256': event['prior_report_sha256'],
              'reconciled_report_sha256': hashlib.sha256(after).hexdigest(),
              'preserved_attempts_count': len(historical['attempts']),
              'new_attempts_added': 0, 'new_samples_added': 0,
              'source_audit_sha256': report['source_audit_sha256']}
    save(EVIDENCE / 'reconcile-result.json', result)
    print('RECONCILED', json.dumps(result), flush=True)


def coverage():
    result = {}
    for language in AUDITS:
        report = read_report(language)
        groups = {}
        for row in report['programs'].values():
            groups.setdefault(row['benchmark'], []).append(row)
        qualified = []
        failures = []
        pending = []
        for benchmark, rows in groups.items():
            controls = [row for row in rows if row['language'] == 'mech']
            programs = [row for row in rows if row['language'] != 'mech']
            if controls and all(len(row['samples_seconds']) >= 3 for row in controls) and any(len(row['samples_seconds']) >= 3 for row in programs):
                qualified.append(benchmark)
            failures.extend(row['key'] for row in rows if row.get('full_failure'))
            pending.extend(row['key'] for row in rows if len(row['samples_seconds']) < 3 and not row.get('full_failure'))
        result[language] = {'report_status': report['status'], 'qualified_benchmarks': qualified,
                            'qualified_count': len(qualified), 'selected_sources_or_controls_pending': pending,
                            'actual_full_failures': failures, 'active': report.get('active')}
    return result


def supervise(orphan_pid):
    STATE.mkdir(parents=True, exist_ok=True)
    reject_other_report_writers()
    leases = []
    for language in AUDITS:
        lease = (STATE / ('owner-' + language + '.lock')).open('a+')
        fcntl.flock(lease, fcntl.LOCK_EX | fcntl.LOCK_NB)
        leases.append(lease)
    cleanup = json.loads((EVIDENCE / 'cleanup.json').read_text())
    assert cleanup['identity']['pid'] == orphan_pid and cleanup['group_exit_confirmed'] is True
    identity = process_identity(orphan_pid)
    if identity:
        assert str(ROOT / 'sources/apl/mandelbrot.compact.apl') in identity['command'], identity
    status = {'pid': os.getpid(), 'started_at': now(), 'source_audits': AUDITS,
              'report_ownership': list(AUDITS), 'orphan_identity': identity,
              'abandoned_orphan_identity': cleanup['identity'],
              'status': 'waiting-for-orphan', 'stages': [], 'coverage': coverage(),
              'protocol': 'Unchanged frozen run.py; three real full samples and fresh controls; shared mutex; planning only for scheduling.'}
    save(STATE / 'status.json', status)
    print('COORDINATOR', os.getpid(), 'WAIT ORPHAN', identity, flush=True)
    while identity and process_identity(orphan_pid) == identity:
        time.sleep(10)
    assert process_identity(orphan_pid) is None, ('Orphan PID still present or reused', orphan_pid)
    identity = cleanup['identity']
    # There is no surviving old run.py report writer. Acquire the global mutex
    # before bookkeeping and before the first guarded stage initialization.
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        reject_other_report_writers()
        verify_audits()
        reconcile_orphan(identity)
    status['status'] = 'running'
    status['orphan_exit_observed_at'] = now()
    save(STATE / 'status.json', status)
    for name, arguments in STAGES:
        reject_other_report_writers()
        stage = {'name': name, 'arguments': arguments, 'started_at': now(),
                 'log': str(STATE / (name + '.log'))}
        status['stages'].append(stage)
        with Path(stage['log']).open('a') as output:
            proc = subprocess.Popen([sys.executable, '-u', str(Path(__file__).resolve()), '--run-stage', name],
                                    cwd=ROOT, stdin=subprocess.DEVNULL, stdout=output,
                                    stderr=subprocess.STDOUT, pass_fds=tuple(lease.fileno() for lease in leases))
            stage['pid'] = proc.pid
            save(STATE / 'status.json', status)
            print('STAGE', name, 'PID', proc.pid, 'LOG', stage['log'], flush=True)
            stage['exit_status'] = proc.wait()
        stage['finished_at'] = now()
        status['coverage'] = coverage()
        if stage['exit_status'] != 0:
            status['status'] = 'stage-error'
            save(STATE / 'status.json', status)
            raise RuntimeError('Stage failed; preserved reports and no automatic duplicate launch: ' + name)
        save(STATE / 'status.json', status)
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        verify_audits()
        from summarize import summarize
        summarize()
    status['coverage'] = coverage()
    outcomes = list(status['coverage'].values())
    status['status'] = ('full-stages-finished-with-failures' if any(item['actual_full_failures'] for item in outcomes)
                        else 'complete' if all(item['qualified_count'] == 10 and not item['selected_sources_or_controls_pending'] for item in outcomes)
                        else 'partial')
    status['finished_at'] = now()
    save(STATE / 'status.json', status)
    print('FINISHED', status['status'], json.dumps(status['coverage']), flush=True)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--run-stage', choices=[name for name, _ in STAGES])
    parser.add_argument('--orphan-pid', type=int, default=25078)
    arguments = parser.parse_args()
    if arguments.run_stage:
        run_stage(arguments.run_stage)
    else:
        supervise(arguments.orphan_pid)
