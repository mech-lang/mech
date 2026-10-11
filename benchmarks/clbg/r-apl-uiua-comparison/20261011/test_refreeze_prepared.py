"""Bounded metadata-only guard revision fixtures; no benchmark is launched."""
import copy
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import refreeze_prepared as revision
import resume_after_guard_repair as resume
from prepare import save, sha


class PreparedRevisionTests(unittest.TestCase):
    def test_selected_fresh_controls_and_prior_unchanged_identity(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            archive = root / 'archive'
            archive.mkdir()
            rows = [dict(benchmark=bench, key=bench + ':regular', id=bench + '-regular',
                         source='/source', source_sha256='a'*64, binary_sha256='b'*64,
                         command=['/bin', '{n}']) for bench in ['mandelbrot', 'regexredux']]
            previous = copy.deepcopy(rows)
            previous[1].update(key='regexredux:previous-fresh', id='previous-fresh')
            old = dict(controls=previous, control_revision={'revision': 'older'})
            path = root / 'source-audit-uiua.json'
            save(archive / path.name, old)
            save(path, dict(controls=rows, baseline_endpoints={row['benchmark']:
                {'control_key': row['key']} for row in rows}, pins={}))
            plan_path = root / 'plan.json'
            save(plan_path, {'revision': 'medium-v3'})
            with patch.object(revision, 'G', root):
                revision.revise_controls('uiua', old, archive,
                    {'fresh_controls': ['mandelbrot']}, {'revision': 'medium-v3', 'reason': 'approved'}, plan_path)
            result = json.loads(path.read_text())
            self.assertEqual(result['controls'][0]['key'], 'mandelbrot:regular-medium-v3')
            self.assertEqual(result['controls'][1]['key'], 'regexredux:previous-fresh')
            for before, after in zip(previous, result['controls']):
                strip = lambda row: {k: v for k, v in row.items() if k not in ['key', 'id']}
                self.assertEqual(strip(before), strip(after))
            self.assertEqual(result['prepared_revision']['plan_sha256'], sha(plan_path))

    def test_preview_retires_without_inventing_a_failure(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            source = root / 'candidate.ua'
            source.write_text('One ← 1\n')
            row = dict(key='mandelbrot:new', benchmark='mandelbrot', language='uiua',
                       source=str(source), source_sha256=sha(source),
                       small_validation={'reference_match': True},
                       algorithm={'strict': True, 'native_benchmark_kernel': False})
            original = dict(row, key='mandelbrot:old', samples_seconds=[1000.0])
            save(root / 'uiua-manifest.json', dict(programs=[original], language='uiua'))
            addition = root / 'new-manifest.json'
            save(addition, dict(programs=[row], language='uiua', status='prepared-validated'))
            descriptor = dict(additions=[dict(path=str(addition), sha256=sha(addition))],
                              retire={'mandelbrot:old': 'Retained first real trial; source-reviewed overhead.'})
            with patch.object(revision, 'G', root):
                result = revision.preview('uiua', descriptor, {'revision': 'v3', 'reason': 'approved'})
            old = result['programs'][0]
            self.assertTrue(old['excluded'])
            self.assertEqual(old['samples_seconds'], [1000.0])
            self.assertNotIn('full_failure', old)
            self.assertEqual(result['programs'][1]['source_sha256'], row['source_sha256'])

    def test_migration_preserves_external_classification_and_archive_hop(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            (root / 'results').mkdir()
            archive = root / 'archive'
            archive.mkdir()
            row = dict(key='mandelbrot:app', benchmark='mandelbrot', language='uiua',
                       source_sha256='a'*64, binary_sha256='b'*64, command=['/bin', '{n}'],
                       cwd='/work', runtime_environment={}, loc=11)
            old_path = archive / 'source-audit-uiua.json'
            save(old_path, dict(programs=[row], controls=[]))
            save(root / 'source-audit-uiua.json', dict(programs=[row], controls=[],
                timing_boundary='same', baseline_endpoints={}))
            attempt = dict(key=row['key'], attempt_number=1, seconds=19.0,
                error='External maintenance', error_type='PostMeasurementExclusion',
                post_measurement_exclusion={'event_sha256': 'c'*64}, valid_environment=True,
                prior_guard_revision_attempt_number=36)
            report = dict(programs={row['key']: dict(row, samples_seconds=[],
                sample_validation=[], attempts=[1])}, attempts=[attempt], discarded=[],
                source_audit_sha256=sha(old_path), post_measurement_exclusions=[{'reason': 'outside work'}])
            save(root / 'results/timings-uiua.json', report)
            save(archive / 'timings-uiua.json', report)
            with patch.object(resume, 'G', root):
                resume.migrate('uiua', archive, 'Explicit approved new guard')
            result = json.loads((root / 'results/timings-uiua.json').read_text())
            current = result['attempts'][0]
            self.assertEqual(current['post_measurement_exclusion'], attempt['post_measurement_exclusion'])
            self.assertEqual(current['seconds'], 19.0)
            self.assertTrue(current['valid_environment'])
            self.assertEqual(current['prior_guard_revision_attempt_number'], 1)
            self.assertEqual(json.loads((archive / 'timings-uiua.json').read_text())['attempts'][0]
                ['prior_guard_revision_attempt_number'], 36)
            self.assertEqual(result['programs'][row['key']]['samples_seconds'], [])
            self.assertEqual(result['post_measurement_exclusions'], report['post_measurement_exclusions'])


if __name__ == '__main__':
    unittest.main()
