"""Additive revision2 maintenance controller; exact review is required before --watch.

Only a verified waiting Python stage owner may be interrupted. No native
benchmark process, original audit/manifest/harness, or original report is edited.
"""
import argparse
from copy import deepcopy
import fcntl
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import time
from datetime import datetime, timezone

HERE = Path(__file__).resolve().parent
CANDIDATE = HERE.parent
N = CANDIDATE.parent
STATE = N / 'results/coordinator-v4'
LOCK = Path('/tmp/mech-clbg-timing.lock')
PLAN = HERE / 'plan2.json'
QUALIFICATION_SHA = '41462f55b1c1f6c800d08cca9ba9ba46cb5e3c6efd7f0ff3d21cfde514b70b7c'
STUDY_SHA = 'd8b6ad100671b3fef5ae46d90dcb36b044733b25ea8d65e3ba7a4589e5b62513'
ORIGINAL_PINS = {
    'run.py': '5b2a038cfae4112e9e2eee79c3cd57ccc6a1521812828c47bf28986d6a8e139b',
    'prepare.py': '27fcf050ed21db40e747f2e8801ca6f227d82f1ea98f8592bba7b10a768109e5',
    'supervise_v4.py': 'd944986c8393f7a80ec0bfb9cef4a7b7a438501583bc806f346779ea7e78487d',
    'source-audit-uiua.json': '82b15648d0c49c6393a0eec1efb441f1ee11237ceae4a8f41ca854c0b7ee0860',
    'uiua-manifest.json': 'c6cbef6595f2f5a8195b4edbe937ebe28165985245d73f34db17dd42a02dac32',
}


def now():
    return datetime.now(timezone.utc).isoformat()


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def read(path):
    return json.loads(Path(path).read_text())


def save_new(path, value):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    # New immutable metadata; an existing result is never replaced.
    with path.open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')


def identity(pid):
    result = subprocess.run(['ps', '-p', str(pid), '-o', 'lstart=', '-o', 'command='], capture_output=True, text=True)
    return dict(pid=pid, listing=result.stdout.strip()) if result.returncode == 0 and result.stdout.strip() else None


def descendants(pid):
    lines = subprocess.check_output(['ps', '-axo', 'pid,ppid,pgid,stat,command'], text=True).splitlines()[1:]
    processes = []
    for line in lines:
        values = line.split(None, 4)
        if len(values) == 5 and not values[3].startswith('Z'):
            processes.append(dict(pid=int(values[0]), parent=int(values[1]), pgid=int(values[2]), state=values[3], command=values[4]))
    found, parents = [], {pid}
    while True:
        current = [row for row in processes if row['parent'] in parents and row['pid'] not in parents]
        if not current:
            break
        found += current
        parents.update(row['pid'] for row in current)
    return found


def verify(pins):
    for path, digest in pins.items():
        assert sha(path) == digest, ('Pinned input changed', path)


def prepare():
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        assert not PLAN.exists(), 'Existing transition plan requires explicit review, not overwrite'
        qualification_path, study_path = CANDIDATE / 'manifest.json', CANDIDATE / 'full-study/manifest.json'
        assert sha(qualification_path) == QUALIFICATION_SHA and sha(study_path) == STUDY_SHA
        qualification, study = read(qualification_path), read(study_path)
        fixture_path = HERE / 'fixtures.json'
        fixture = read(fixture_path)
        assert fixture['status'] == 'passed' and fixture['cases'] == 9 and fixture['actual_signals_sent'] == 0
        assert fixture['controller_sha256'] == sha(__file__), 'Controller changed after its fixtures'
        assert qualification['status'] == 'prepared-validated'
        assert study['samples'] == 3 and study['size'] == 12 and study['canonical_full_workload'] is True
        pins = dict(study['pins'])
        pins.update({str(N / name): digest for name, digest in ORIGINAL_PINS.items()})
        pins.update({str(qualification_path): QUALIFICATION_SHA, str(study_path): STUDY_SHA,
                     str(Path(__file__).resolve()): sha(__file__), str(HERE / 'README.md'): sha(HERE / 'README.md'),
                     str(fixture_path): sha(fixture_path), str(HERE / 'test_controller.py'): sha(HERE / 'test_controller.py'),
                     str(HERE / 'fixtures.log'): sha(HERE / 'fixtures.log')})
        verify(pins)
        coordinator = read(STATE / 'status.json')
        assert coordinator['status'] == 'running'
        stage = next(row for row in coordinator['stages'] if row['name'] == 'long-r-uiua')
        assert 'exit_status' not in stage and 'finished_at' not in stage
        owner, supervisor = identity(stage['pid']), identity(coordinator['pid'])
        assert owner and supervisor
        assert '--run-stage long-r-uiua' in owner['listing'] and 'supervise_v4.py' in owner['listing']
        assert 'supervise_v4.py' in supervisor['listing']
        audits = {}
        for language, digest in coordinator['source_audits'].items():
            path = N / ('source-audit-' + language + '.json')
            assert sha(path) == digest
            audits[language] = dict(path=str(path), sha256=digest)
            pins[str(path)] = digest
        plan = dict(status='prepared-awaiting-exact-controller-review', created_at=now(), pins=pins,
                    qualification=str(qualification_path), qualification_sha256=QUALIFICATION_SHA,
                    full_study_manifest=str(study_path), full_study_manifest_sha256=STUDY_SHA,
                    coordinator_identity=supervisor, stage_owner_identity=owner, stage='long-r-uiua', stage_log=stage['log'],
                    audit_inputs=audits, report_inputs={language: str(N / 'results' / ('timings-' + language + '.json')) for language in audits},
                    maximum_watch_seconds=21600, maximum_exit_wait_seconds=30, maximum_boundary_settle_seconds=30,
                    original_leases=[str(STATE / ('owner-' + language + '.lock')) for language in audits],
                    boundary='All R/Uiua nbody source/control rows have three qualified full samples; all reports inactive; exact owner identities; next-row WAIT LOCK is fannkuch; owner has no descendant.',
                    signal_policy='SIGINT only the recorded waiting Python stage owner. Never signal a native process/group or the supervisor. Require parent/owner exit and free leases before copying metadata.',
                    revision_directory=str(HERE / 'revision'),
                    mutation_policy='This controller writes only new transition/revision files. Original reports remain unchanged during transition; a separately reviewed continuation owner may resume existing R/APL reports afterwards. All original audits/manifests/helpers/sources stay byte-identical.',
                    remaining_stage_plan=[dict(languages='r', benchmarks='fannkuchredux', samples=3),
                                          dict(languages='apl', benchmarks='mandelbrot,fasta,knucleotide,binarytrees,nbody,fannkuchredux', samples=3)],
                    legacy_attempt_policy='Every prior Uiua Fann attempt, valid sample, failure, discarded attempt and order is retained. An active native trial must finish normally or by its own recorded timeout before this controller may reach a child-free WAIT.',
                    result_policy='No cloned Uiua report or combined attempt orders. Retain original non-Fann report snapshot and overlay completed separate Fann study only after independent review of three actual N12 samples. A separately reviewed owner resumes existing R/APL samples. This boundary controller launches no benchmarks.')
        verify(pins)
        save_new(PLAN, plan)
        print(json.dumps(dict(plan=str(PLAN), sha256=sha(PLAN), controller=str(Path(__file__).resolve()), controller_sha256=sha(__file__)), indent=2), flush=True)


def nbody_qualified(report):
    rows = [row for row in report['programs'].values() if row['benchmark'] == 'nbody']
    assert rows, 'Missing nbody source/control group'
    for row in rows:
        if len(row['samples_seconds']) != 3 or len(row['sample_validation']) != 3 or row.get('full_failure'):
            return False
        if not all(value.get('reference_match') is True for value in row['sample_validation']):
            return False
        qualified = [attempt for attempt in report['attempts'] if attempt['key'] == row.get('timing_alias_of', row['key'])
                     and not attempt.get('error') and attempt.get('valid_environment') is True
                     and attempt.get('validation', {}).get('reference_match') is True]
        if len(qualified) < 3:
            return False
        for attempt in qualified[-3:]:
            if not (attempt.get('full_workload') is True and attempt.get('workload_n') == 50000000 and attempt.get('exit_status') == 0):
                return False
            if any(attempt.get(key) for key in ('resource_violation', 'resource_monitor_errors', 'monitor_errors', 'monitor_finish_error', 'cleanup_error', 'remaining_group_members')):
                return False
    return True


def boundary(plan):
    assert identity(plan['stage_owner_identity']['pid']) == plan['stage_owner_identity'], 'Stage owner exited or identity changed before approved boundary'
    assert identity(plan['coordinator_identity']['pid']) == plan['coordinator_identity'], 'Coordinator exited or identity changed before approved boundary'
    coordinator = read(STATE / 'status.json')
    assert coordinator['status'] == 'running'
    stage = next(row for row in coordinator['stages'] if row['name'] == plan['stage'])
    assert stage['pid'] == plan['stage_owner_identity']['pid'] and 'exit_status' not in stage
    reports = {language: read(path) for language, path in plan['report_inputs'].items()}
    for language, report in reports.items():
        assert report['source_audit_sha256'] == plan['audit_inputs'][language]['sha256']
    if any(report.get('active') is not None for report in reports.values()):
        return None
    if not all(nbody_qualified(reports[language]) for language in ('r', 'uiua')):
        return None
    # The final run_sample group-save/summarize occurs outside the common
    # lock. Once both nbody groups qualify, retain the lock while that Python
    # bookkeeping reaches its next WAIT LOCK; releasing here permits a race
    # with the next Fann launch. No native trial can start while we hold it.
    deadline = time.monotonic() + plan['maximum_boundary_settle_seconds']
    fann_ids = {row['id'] for report in (reports['r'], reports['uiua']) for row in report['programs'].values() if row['benchmark'] == 'fannkuchredux'}
    while True:
        assert identity(plan['stage_owner_identity']['pid']) == plan['stage_owner_identity']
        assert identity(plan['coordinator_identity']['pid']) == plan['coordinator_identity']
        lines = Path(plan['stage_log']).read_text().splitlines()
        waiting_fann = bool(lines and lines[-1].startswith('WAIT LOCK ') and any(lines[-1].endswith(' ' + identifier) for identifier in fann_ids))
        children = descendants(plan['stage_owner_identity']['pid'])
        if waiting_fann and not children:
            # Capture fresh bytes only after the owner has completed its final
            # save/summarize and is blocked on this held common mutex.
            fresh = {language: read(path) for language, path in plan['report_inputs'].items()}
            assert all(report.get('active') is None for report in fresh.values())
            assert all(nbody_qualified(fresh[language]) for language in ('r', 'uiua'))
            assert all(fresh[language]['source_audit_sha256'] == plan['audit_inputs'][language]['sha256'] for language in fresh)
            return dict(observed_at=now(), reports=fresh, last_stage_log_line=lines[-1], stage_descendants=children,
                        report_sha256={language: sha(path) for language, path in plan['report_inputs'].items()})
        assert time.monotonic() < deadline, 'Inactive owner did not settle at a child-free Fann WAIT within30s; no process signalled'
        time.sleep(.05)


def revision(plan, receipt):
    root = Path(plan['revision_directory'])
    root.mkdir(parents=True, exist_ok=False)
    evidence = root / 'before'
    evidence.mkdir()
    originals = [Path(path) for path in plan['report_inputs'].values()] + [Path(row['path']) for row in plan['audit_inputs'].values()]
    originals += [N / 'uiua-manifest.json', STATE / 'status.json', Path(plan['stage_log'])]
    lineage = {}
    for original in originals:
        destination = evidence / original.name
        destination.write_bytes(original.read_bytes())
        lineage[str(original)] = dict(snapshot=str(destination), sha256=sha(destination))
    old_report = receipt['reports']['uiua']
    qualification = read(plan['qualification'])
    study = read(plan['full_study_manifest'])
    retired = {key: row for key, row in old_report['programs'].items() if row['benchmark'] == 'fannkuchredux' and row['language'] == 'uiua'}
    original_controls = {key: row for key, row in old_report['programs'].items() if row['benchmark'] == 'fannkuchredux' and row['language'] == 'mech'}
    endpoint_audit = dict(status='qualified-sources-separate-full-study-awaiting-import-review', created_at=now(),
                          qualification=plan['qualification'], qualification_sha256=plan['qualification_sha256'],
                          full_study_manifest=plan['full_study_manifest'], full_study_manifest_sha256=plan['full_study_manifest_sha256'],
                          source_endpoints=[{key: row[key] for key in ('id', 'key', 'source', 'source_sha256', 'loc', 'maximum_workers')} for row in qualification['programs']],
                          fresh_mech_control=next(row for row in study['profiles'] if row['id'] == 'mech59-aot-control'),
                          original_uiua_report=lineage[plan['report_inputs']['uiua']],
                          original_uiua_audit=lineage[plan['audit_inputs']['uiua']['path']],
                          preserved_non_fann_keys=[key for key, row in old_report['programs'].items() if row['benchmark'] != 'fannkuchredux'],
                          retained_legacy_v2_keys=list(retired), retained_legacy_v2_rows=retired,
                          retained_legacy_fann_attempts=[attempt for attempt in old_report['attempts'] if attempt.get('benchmark') == 'fannkuchredux'],
                          retained_legacy_fann_orders=old_report.get('orders', {}).get('fannkuchredux', []),
                          retained_legacy_discarded_attempts=old_report.get('discarded', []),
                          retained_original_fann_controls=original_controls,
                          imported_performance_samples=0, chart_admission=False,
                          publication_policy='Root importer may overlay completed independently reviewed three-actual-N12 study endpoints on the unchanged non-Fann Uiua snapshot; no combined attempt order or synthetic timing.')
    audit_path = root / 'uiua-fann-v5-endpoint-revision.json'
    save_new(audit_path, endpoint_audit)
    mapping = dict(status='prepared-continuation-not-launched', created_at=now(), lineage=lineage,
                   retained_uiua_report=lineage[plan['report_inputs']['uiua']],
                   continuing_reports={language: plan['report_inputs'][language] for language in ('r', 'apl')},
                   continuing_audits={language: plan['audit_inputs'][language] for language in ('r', 'apl')},
                   uiua_endpoint_revision=dict(path=str(audit_path), sha256=sha(audit_path)),
                   separate_uiua_fann_study=dict(manifest=plan['full_study_manifest'], manifest_sha256=plan['full_study_manifest_sha256'],
                                         report=str(CANDIDATE / 'full-study/report.json'), import_requires_complete_three_actual_samples=True),
                   remaining_stage_plan=plan['remaining_stage_plan'], no_benchmarks_launched=True,
                   original_reports_preserved=True, original_five_pinned_inputs_preserved=True)
    save_new(root / 'report-mapping.json', mapping)
    return dict(revision_directory=str(root), endpoint_revision=str(audit_path), endpoint_revision_sha256=sha(audit_path),
                mapping=str(root / 'report-mapping.json'), mapping_sha256=sha(root / 'report-mapping.json'))


def watch(digest):
    started = time.monotonic()
    while True:
        with LOCK.open('a+') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            assert sha(PLAN) == digest, 'Reviewed transition plan changed'
            plan = read(PLAN)
            verify(plan['pins'])
            assert not (HERE / 'transition.json').exists(), 'Transition already exists; no duplicate signalling'
            assert time.monotonic() - started < plan['maximum_watch_seconds'], 'Maintenance watcher expired without signalling any process'
            observed = boundary(plan)
            if observed is not None:
                # Receipt is durable before the only signal. Native groups are
                # neither targeted nor owned by this maintenance controller.
                observed_receipt = {key: value for key, value in observed.items() if key != 'reports'}
                save_new(HERE / 'boundary.json', dict(observed_receipt, plan_sha256=digest, owner=plan['stage_owner_identity'], supervisor=plan['coordinator_identity']))
                assert identity(plan['stage_owner_identity']['pid']) == plan['stage_owner_identity']
                assert not descendants(plan['stage_owner_identity']['pid'])
                os.kill(plan['stage_owner_identity']['pid'], signal.SIGINT)
                deadline = time.monotonic() + plan['maximum_exit_wait_seconds']
                while identity(plan['stage_owner_identity']['pid']) is not None or identity(plan['coordinator_identity']['pid']) is not None:
                    assert time.monotonic() < deadline, 'Controller/supervisor did not exit; no further signals or revision writes'
                    time.sleep(.05)
                terminal = read(STATE / 'status.json')
                last = next(row for row in terminal['stages'] if row['name'] == plan['stage'])
                assert terminal['status'] == 'stage-error' and last['exit_status'] != 0
                assert all(sha(path) == observed['report_sha256'][language] for language, path in plan['report_inputs'].items()), 'Original report changed during maintenance stop'
                leases = []
                try:
                    for filename in plan['original_leases']:
                        lease = Path(filename).open('a+')
                        fcntl.flock(lease, fcntl.LOCK_EX | fcntl.LOCK_NB)
                        leases.append(lease)
                    verify(plan['pins'])
                    result = revision(plan, observed)
                    verify(plan['pins'])
                    save_new(HERE / 'transition.json', dict(status='maintenance-complete-new-owner-awaiting-review', finished_at=now(), plan_sha256=digest,
                                                          boundary=observed_receipt, stage_exit_status=last['exit_status'], signal='SIGINT to waiting Python stage owner only',
                                                          native_processes_signalled=False, benchmarks_launched=False, **result))
                    print(json.dumps(read(HERE / 'transition.json'), indent=2), flush=True)
                    return
                finally:
                    for lease in leases:
                        lease.close()
        time.sleep(1)


def main():
    parser = argparse.ArgumentParser()
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--prepare', action='store_true')
    mode.add_argument('--watch', action='store_true')
    parser.add_argument('--plan-sha256')
    args = parser.parse_args()
    if args.prepare:
        prepare()
    else:
        assert args.plan_sha256, 'Exact independently reviewed plan SHA required'
        watch(args.plan_sha256)


if __name__ == '__main__':
    try:
        main()
    except BaseException as error:
        path = HERE / ('controller-error-' + datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%S%fZ') + '.json')
        save_new(path, dict(finished_at=now(), error=str(error), error_type=type(error).__name__, no_automatic_retry=True))
        raise
