"""Bounded metadata-only guard revision fixtures; no benchmark is launched."""
import copy
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import refreeze_fasta_v4 as revision
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
                         command=['/bin', '{n}']) for bench in ['fasta', 'mandelbrot']]
            previous = copy.deepcopy(rows)
            previous[1].update(key='mandelbrot:previous-fresh', id='previous-fresh')
            old = dict(controls=previous, control_revision={'revision': 'older'})
            path = root / 'source-audit-uiua.json'
            save(archive / path.name, old)
            save(path, dict(controls=rows, baseline_endpoints={row['benchmark']:
                {'control_key': row['key']} for row in rows}, pins={}))
            plan_path = root / 'plan.json'
            save(plan_path, {'revision': 'fasta-v4'})
            with patch.object(revision, 'G', root):
                revision.revise_controls('uiua', old, archive,
                    {'fresh_controls': ['fasta']}, {'revision': 'fasta-v4', 'reason': 'approved'}, plan_path)
            result = json.loads(path.read_text())
            self.assertEqual(result['controls'][0]['key'], 'fasta:regular-fasta-v4')
            self.assertEqual(result['controls'][1]['key'], 'mandelbrot:previous-fresh')
            for before, after in zip(previous, result['controls']):
                strip = lambda row: {k: v for k, v in row.items() if k not in ['key', 'id']}
                self.assertEqual(strip(before), strip(after))
            self.assertEqual(result['prepared_revision']['plan_sha256'], sha(plan_path))

    def test_preview_retains_prior_candidate_and_adds_only_fasta(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            source = root / 'candidate.ua'
            source.write_text('One ← 1\n')
            row = dict(key='fasta:new', benchmark='fasta', language='uiua',
                       source=str(source), source_sha256=sha(source),
                       small_validation={'reference_match': True},
                       algorithm={'strict': True, 'native_benchmark_kernel': False})
            original = dict(row, key='fasta:old', samples_seconds=[25.0, 26.0, 27.0])
            save(root / 'uiua-manifest.json', dict(programs=[original], language='uiua'))
            addition = root / 'new-manifest.json'
            save(addition, dict(programs=[row], language='uiua', status='prepared-validated'))
            descriptor = dict(additions=[dict(path=str(addition), sha256=sha(addition))],
                              retire={}, fresh_controls=['fasta'])
            with patch.object(revision, 'G', root):
                result = revision.preview('uiua', descriptor, {'revision': 'fasta-v4', 'reason': 'approved'})
            self.assertEqual(result['programs'][0], original)
            self.assertEqual(result['programs'][1]['source_sha256'], row['source_sha256'])
            descriptor['retire'] = {'fasta:old': 'Unjustified replacement'}
            with patch.object(revision, 'G', root):
                with self.assertRaises(AssertionError):
                    revision.preview('uiua', descriptor, {'revision': 'fasta-v4', 'reason': 'approved'})

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
                source_audit_sha256=sha(old_path), post_measurement_exclusions=[{'reason': 'outside work'}],
                completed_benchmarks=['mandelbrot','fasta'], orders={'mandelbrot':[[row['key']]],'fasta':[['old-fasta-key']]})
            save(root / 'results/timings-uiua.json', report)
            save(archive / 'timings-uiua.json', report)
            with patch.object(resume, 'G', root), patch.object(revision, 'G', root):
                revision.migrate_preserving_unaffected('uiua', archive, 'Explicit approved new guard', ['fasta'])
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
            self.assertEqual(result['completed_benchmarks'], ['mandelbrot'])
            self.assertEqual(result['orders'], {'mandelbrot':[[row['key']]]})
            self.assertEqual(result.get('historical_attempts', []), [])


if __name__ == '__main__':
    unittest.main()
