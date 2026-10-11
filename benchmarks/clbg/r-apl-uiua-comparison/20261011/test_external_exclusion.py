"""Metadata correction fixtures; no benchmark process is launched."""
import copy
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import apply_external_exclusion as exclusion
import resume_after_guard_repair as resume
from prepare import save, sha


def fixture():
    row = dict(key='generic:application', id='application', benchmark='generic',
               language='r', source_sha256='a' * 64, command=['/runtime', '/source'],
               loc=10, runtime_environment={'LC_ALL': 'C'}, samples_seconds=[1., 2., 3.],
               sample_validation=[{'reference_match': True}] * 3, attempts=[1, 2, 3],
               measurement_status='measured', median_seconds=2.)
    attempts = [dict(key=row['key'], id=row['id'], benchmark=row['benchmark'],
                     language=row['language'], source_sha256=row['source_sha256'],
                     command=row['command'], attempt_number=i, seconds=float(i),
                     full_workload=True, valid_environment=True,
                     stdout_sha256='b' * 64, stdout_bytes=20, validation={'reference_match': True},
                     started_at='2026-10-11T12:00:00+00:00', finished_at='2026-10-11T12:01:00+00:00',
                     monitor_errors=[], competing_compilers=[])
                for i in [1, 2, 3]]
    report = dict(programs={row['key']: row}, attempts=attempts, completed_benchmarks=['generic'],
                  status='complete', discarded=[])
    event = dict(reason='Verified external maintenance', conservative_window={
        'start': '2026-10-11T12:00:30+00:00', 'end': '2026-10-11T12:00:40+00:00'},
        affected_attempts=[{'language': 'r', 'signature': {'key': row['key'], 'seconds': 1.,
                            'stdout_sha256': 'b' * 64}}])
    return report, event


class ExternalExclusionTests(unittest.TestCase):
    def test_classification_establishes_immediate_archive_hop(self):
        report, event = fixture()
        report['attempts'][0]['prior_guard_revision_attempt_number'] = 9
        original = copy.deepcopy(report)
        exclusion.classify(report, [1], Path('/event.json'), 'c' * 64, event, '2026-10-11T13:00:00+00:00')
        exclusion.link_prior_guard(report)
        self.assertEqual(original['attempts'][0]['prior_guard_revision_attempt_number'], 9)
        self.assertEqual([a['prior_guard_revision_attempt_number'] for a in report['attempts']], [1, 2, 3])
        self.assertEqual([a['attempt_number'] for a in report['attempts']], [1, 2, 3])
        self.assertEqual(report['attempts'][1]['seconds'], original['attempts'][1]['seconds'])

    def test_only_post_classification_changes(self):
        report, event = fixture()
        before = copy.deepcopy(report)
        self.assertEqual(exclusion.selection(event, {'r': report}), {'r': [1]})
        exclusion.classify(report, [1], Path('/event.json'), 'c' * 64, event, '2026-10-11T13:00:00+00:00')
        changed = report['attempts'][0]
        self.assertEqual({k: v for k, v in changed.items() if k not in exclusion.CLASSIFICATION_FIELDS},
                         before['attempts'][0])
        self.assertIs(changed['valid_environment'], True)
        self.assertEqual(changed['error_type'], 'PostMeasurementExclusion')
        self.assertEqual(report['attempts'][1:], before['attempts'][1:])
        row = report['programs']['generic:application']
        self.assertEqual(row['samples_seconds'], [2., 3.])
        self.assertEqual(row['median_seconds'], 2.5)
        self.assertEqual(row['measurement_status'], 'partial')
        self.assertNotIn('full_failure', row)
        self.assertEqual(report['completed_benchmarks'], [])

    def test_later_guard_migration_preserves_classification(self):
        report, event = fixture()
        exclusion.classify(report, [1], Path('/event.json'), 'c' * 64, event, '2026-10-11T13:00:00+00:00')
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            archive = root / 'archive'
            archive.mkdir()
            (root / 'results').mkdir()
            row = report['programs']['generic:application']
            audit = dict(programs=[row], controls=[], timing_boundary='unchanged', baseline_endpoints={})
            old_path = archive / 'source-audit-r.json'
            save(old_path, audit)
            report['source_audit_sha256'] = sha(old_path)
            save(archive / 'timings-r.json', report)
            save(root / 'results/timings-r.json', report)
            save(root / 'source-audit-r.json', audit)
            with patch.object(resume, 'G', root):
                resume.migrate('r', archive, 'Independent additive preparation')
            migrated = json.loads((root / 'results/timings-r.json').read_text())
            self.assertEqual(migrated['post_measurement_exclusions'], report['post_measurement_exclusions'])
            actual = migrated['attempts'][0]
            self.assertEqual(actual['post_measurement_exclusion'], report['attempts'][0]['post_measurement_exclusion'])
            self.assertEqual(actual['error_type'], 'PostMeasurementExclusion')
            self.assertEqual(actual['seconds'], 1.)
            self.assertEqual(migrated['programs'][row['key']]['samples_seconds'], [2., 3.])


if __name__ == '__main__':
    unittest.main()
