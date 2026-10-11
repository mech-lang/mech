"""Archive and explicitly exclude measured trials overlapped by external work.

Preparation/maintenance can be found after a sample's automatic monitor has
finished. This generic correction preserves measured time, output and original
monitor/environment fields, and classifies the affected trial separately. It
must only run after the owning coordinator exits; ordinary three-sample runs
then replace the missing evidence. No benchmark source or workload is changed.
"""
import argparse
import copy
import fcntl
import json
import shutil
import statistics
from datetime import datetime, timezone
from pathlib import Path

from prepare import G, LANGUAGES, LOCK, save, sha
from run import PinGuard

CLASSIFICATION_FIELDS = ['error', 'error_type', 'post_measurement_exclusion']


def now():
    return datetime.now(timezone.utc).isoformat()


def matches(attempt, signature):
    return all(attempt.get(k) == v for k, v in signature.items())


def selection(event, reports):
    result = {}
    for descriptor in event['affected_attempts']:
        language = descriptor['language']
        assert language in LANGUAGES
        found = [a for a in reports[language]['attempts']
                 if matches(a, descriptor['signature'])]
        assert len(found) == 1, ('Attempt signature must identify exactly one trial', descriptor)
        attempt = found[0]
        assert not attempt.get('error') and not attempt.get('post_measurement_exclusion'), attempt
        assert attempt['full_workload'] and attempt['validation']['reference_match']
        assert attempt['valid_environment'] is True
        begin = datetime.fromisoformat(attempt['started_at'])
        end = datetime.fromisoformat(attempt['finished_at'])
        maintenance_begin = datetime.fromisoformat(event['conservative_window']['start'])
        maintenance_end = datetime.fromisoformat(event['conservative_window']['end'])
        assert begin < maintenance_end and end > maintenance_begin, ('No recorded overlap', descriptor)
        result.setdefault(language, []).append(attempt['attempt_number'])
    assert sum(map(len, result.values())) == len(event['affected_attempts'])
    for numbers in result.values():
        assert len(numbers) == len(set(numbers))
    return result


def accepted_attempts(report, row):
    owner = row.get('timing_alias_of') or row['key']
    return [a for a in report['attempts'] if a['key'] == owner
            and a.get('full_workload') is True and a.get('valid_environment') is True
            and not a.get('error') and a.get('validation', {}).get('reference_match') is True
            and a.get('source_sha256') == row['source_sha256']]


def classify(report, numbers, event_path, event_hash, event, at):
    original = copy.deepcopy(report)
    affected = [a for a in report['attempts'] if a['attempt_number'] in numbers]
    owners = {a['key'] for a in affected}
    benches = {a['benchmark'] for a in affected}
    rows = [r for r in report['programs'].values()
            if (r.get('timing_alias_of') or r['key']) in owners]
    for row in rows:
        assert row['samples_seconds'] == [a['seconds'] for a in accepted_attempts(original, row)]
    for attempt in affected:
        attempt['post_measurement_exclusion'] = {
            'event': str(event_path), 'event_sha256': event_hash,
            'reason': event['reason'], 'classified_at': at,
            'original_attempt_number': attempt['attempt_number'],
            'original_automatic_valid_environment': attempt['valid_environment'],
            'conservative_maintenance_window': event['conservative_window'],
        }
        attempt['error_type'] = 'PostMeasurementExclusion'
        attempt['error'] = 'Post-measurement external-maintenance exclusion: ' + event['reason']
        before = next(a for a in original['attempts'] if a['attempt_number'] == attempt['attempt_number'])
        assert {k: v for k, v in attempt.items() if k not in CLASSIFICATION_FIELDS} == {
            k: v for k, v in before.items() if k not in CLASSIFICATION_FIELDS}
    for row in rows:
        accepted = accepted_attempts(report, row)
        row['samples_seconds'] = [a['seconds'] for a in accepted]
        row['sample_validation'] = [copy.deepcopy(a['validation']) for a in accepted]
        row['measurement_status'] = 'partial' if accepted else 'pending'
        if accepted:
            row['median_seconds'] = statistics.median(row['samples_seconds'])
        else:
            row.pop('median_seconds', None)
        if len(accepted) >= 2:
            row['sample_stdev_seconds'] = statistics.stdev(row['samples_seconds'])
        else:
            row.pop('sample_stdev_seconds', None)
        # Application output validation remains valid; this excludes timing
        # evidence for external interference rather than asserting a program failure.
        assert not row.get('full_failure')
    report['completed_benchmarks'] = [b for b in report.get('completed_benchmarks', []) if b not in benches]
    report.update(status='partial', updated_at=at)
    report.setdefault('post_measurement_exclusions', []).append({
        'event': str(event_path), 'event_sha256': event_hash, 'at': at,
        'attempt_numbers': numbers, 'reason': event['reason']})
    return owners


def link_prior_guard(report):
    # This correction adds a new archived-report hop without reindexing. Every
    # attempt must point into that immediate prior report, including trials
    # unaffected by the classification. Earlier links remain in its exact copy.
    for attempt in report['attempts']:
        attempt['prior_guard_revision_attempt_number'] = attempt['attempt_number']


def apply(event_path, event, chosen, reports, archive):
    with LOCK.open('a+') as timing_lock:
        fcntl.flock(timing_lock, fcntl.LOCK_EX)
        with (G / 'results/.summary.lock').open('a+') as summary_lock:
            fcntl.flock(summary_lock, fcntl.LOCK_EX)
            # Reload after obtaining ownership: live report state cannot be
            # changed while its coordinator has an active child or pending jobs.
            current = {language: json.loads((G / 'results' / ('timings-' + language + '.json')).read_text())
                       for language in chosen}
            assert selection(event, current) == chosen
            for language, report in current.items():
                assert not report.get('active'), ('Coordinator owns an active run', language)
                audit_path = Path(report['source_audit'])
                assert report['source_audit_sha256'] == sha(audit_path)
                PinGuard(json.loads(audit_path.read_text())['pins']).verify()
            # Caller/parent must additionally confirm the owning coordinator
            # has exited; a between-process gap is not a safe mutation boundary.
            archive.mkdir(parents=True, exist_ok=False)
            paths = [event_path, Path(__file__).resolve()] + [G / p for p in
                     ['prepare.py', 'run.py', 'summarize.py', 'resume_after_guard_repair.py']]
            for language in chosen:
                paths += [G / (language + '-manifest.json'),
                          G / ('source-audit-' + language + '.json'),
                          G / 'results' / ('timings-' + language + '.json')]
            for path in paths:
                shutil.copy2(path, archive / path.name)
            evidence = {'type': 'post_measurement_exclusion', 'status': 'archived-before-classification',
                        'created_at': now(), 'event_sha256': sha(event_path),
                        'files': {str(archive / p.name): sha(archive / p.name) for p in paths}}
            save(archive / 'revision.json', evidence)
            for language, report in current.items():
                at = now()
                audit_path = G / ('source-audit-' + language + '.json')
                prior_audit = archive / audit_path.name
                prior_report = archive / ('timings-' + language + '.json')
                audit = json.loads(audit_path.read_text())
                event_hash = sha(event_path)
                classification = {'type': 'post_measurement_exclusion', 'at': at,
                                  'event': str(event_path), 'event_sha256': event_hash,
                                  'reason': event['reason'], 'attempt_numbers': chosen[language],
                                  'permitted_changed_attempt_fields': CLASSIFICATION_FIELDS,
                                  'prior_source_audit': str(prior_audit),
                                  'prior_source_audit_sha256': sha(prior_audit),
                                  'prior_report': str(prior_report), 'prior_report_sha256': sha(prior_report)}
                classify(report, chosen[language], event_path, event_hash, event, at)
                link_prior_guard(report)
                audit['pins'][str(event_path)] = event_hash
                helper = Path(__file__).resolve()
                audit['pins'][str(helper)] = sha(helper)
                audit.setdefault('post_measurement_classification_revisions', []).append(classification)
                save(audit_path, audit)
                report['source_audit_sha256'] = sha(audit_path)
                report.setdefault('guard_revisions', []).append({
                    **classification, 'classification_correction': classification,
                    'preserved_keys': list(report['programs']), 'retired_keys': [], 'new_keys': []})
                save(G / 'results' / ('timings-' + language + '.json'), report)
                print('EXCLUDED EXTERNAL MAINTENANCE', language, chosen[language], flush=True)
            evidence.update(status='complete', finished_at=now())
            save(archive / 'revision.json', evidence)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--event', required=True)
    parser.add_argument('--archive', required=True)
    parser.add_argument('--apply', action='store_true')
    parser.add_argument('--coordinator-exited', action='store_true',
                        help='Explicit parent-confirmed stage boundary, never a between-child gap')
    args = parser.parse_args()
    event_path = Path(args.event).resolve()
    event = json.loads(event_path.read_text())
    languages = {r['language'] for r in event['affected_attempts']}
    reports = {language: json.loads((G / 'results' / ('timings-' + language + '.json')).read_text())
               for language in languages}
    chosen = selection(event, reports)
    print('PREVIEW EXCLUSIONS', chosen, flush=True)
    if args.apply:
        assert args.coordinator_exited, 'Owning coordinator exit must be confirmed by the parent'
        apply(event_path, event, chosen, reports, Path(args.archive).resolve())
    else:
        print('No report, audit, source or timing evidence modified.', flush=True)


if __name__ == '__main__':
    main()
