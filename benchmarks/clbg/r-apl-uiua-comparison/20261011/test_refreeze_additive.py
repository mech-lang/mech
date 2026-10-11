"""Preparation-metadata canaries; no benchmark process is launched."""
import copy
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import refreeze_additive as revision
import refreeze_knucleotide as nucleotide
import resume_after_guard_repair as resume
from prepare import save, sha


class AdditiveRevisionTests(unittest.TestCase):
    def test_nucleotide_revision_retains_prior_fresh_controls(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            archive = root / 'archive'
            archive.mkdir()
            ordinary = dict(benchmark='regexredux', key='regexredux:regular',
                            id='regular', source='/source', command=['/bin', '{n}'])
            earlier = dict(ordinary, key='regexredux:additive-v3', id='fresh-v3')
            old = dict(controls=[earlier], control_revision={'revision': 'additive-v3'})
            path = root / 'source-audit-apl.json'
            save(archive / path.name, old)
            save(path, dict(controls=[ordinary], baseline_endpoints={
                'regexredux': {'control_key': ordinary['key']}}, pins={}))
            with patch.object(nucleotide, 'G', root):
                nucleotide.retain_control_identities('apl', old, archive)
            new = json.loads(path.read_text())
            self.assertEqual(new['controls'], [earlier])
            self.assertEqual(new['baseline_endpoints']['regexredux']['control_key'], earlier['key'])
            self.assertEqual(new['additive_revision']['prior_control_revision'], old['control_revision'])
            self.assertEqual(new['pins'][str(Path(nucleotide.__file__).resolve())], sha(nucleotide.__file__))

    def test_controls_only_change_key_and_id(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            archive = root / 'archive'
            archive.mkdir()
            rows = [dict(benchmark=b, key=b + ':old-control', id=b + '-control',
                         source='/unchanged/source', source_sha256='a' * 64,
                         command=['/unchanged/bin', '{n}'], binary='/unchanged/bin',
                         binary_sha256='b' * 64, runtime_environment={'LC_ALL': 'C'})
                    for b in ['regexredux', 'revcomp', 'pidigits']]
            old = dict(controls=rows, baseline_endpoints={r['benchmark']: {
                'control_key': r['key']} for r in rows}, pins={})
            path = root / 'source-audit-apl.json'
            save(path, old)
            with patch.object(revision, 'G', root):
                revision.revise_controls('apl', archive)
            new = json.loads(path.read_text())
            for before, after in zip(rows, new['controls']):
                stripped = lambda row: {k: v for k, v in row.items() if k not in ['key', 'id']}
                self.assertEqual(stripped(before), stripped(after))
                changed = before['benchmark'] in ['regexredux', 'revcomp']
                self.assertEqual(before['key'] != after['key'], changed)
            preserved = archive / 'source-audit-apl.pre-revision-prepare.json'
            self.assertEqual(json.loads(preserved.read_text()), old)
            self.assertEqual(new['control_revision']['pre_revision_prepare_audit_sha256'], sha(preserved))
            self.assertEqual(new['pins'][str(Path(revision.__file__).resolve())], sha(revision.__file__))

    def test_new_controls_empty_and_prior_trials_historical(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            archive = root / 'archive'
            archive.mkdir()
            (root / 'results').mkdir()
            application = dict(key='regexredux:app', id='app', benchmark='regexredux',
                               language='apl', source_sha256='a' * 64, loc=33,
                               command=['/runtime', '/source'], cwd='/work',
                               runtime_environment={'LC_ALL': 'C'})
            control = dict(application, key='regexredux:old-control', id='control', language='mech')
            fresh = dict(control, key='regexredux:new-control', id='fresh-control')
            old_audit = dict(programs=[application], controls=[control])
            old_path = archive / 'source-audit-apl.json'
            save(old_path, old_audit)
            new_audit = dict(programs=[application], controls=[fresh], timing_boundary='unchanged',
                             baseline_endpoints={'regexredux': {'control_key': fresh['key']}})
            save(root / 'source-audit-apl.json', new_audit)
            measured_app = dict(application, samples_seconds=[2.0], sample_validation=[{'reference_match': True}],
                                attempts=[1], measurement_status='partial')
            measured_control = dict(control, samples_seconds=[0.1, 0.2, 0.3],
                                    sample_validation=[{'reference_match': True}] * 3,
                                    attempts=[2, 3, 4], measurement_status='measured')
            attempts = [dict(key=application['key'], attempt_number=1, seconds=2.0)] + [
                dict(key=control['key'], attempt_number=i + 2, seconds=seconds)
                for i, seconds in enumerate(measured_control['samples_seconds'])]
            report = dict(programs={application['key']: measured_app, control['key']: measured_control},
                          attempts=attempts, discarded=[], source_audit_sha256=sha(old_path))
            save(root / 'results/timings-apl.json', report)
            save(archive / 'timings-apl.json', report)
            with patch.object(resume, 'G', root):
                resume.migrate('apl', archive, 'Approved fresh control revision')
            result = json.loads((root / 'results/timings-apl.json').read_text())
            self.assertEqual(result['programs'][fresh['key']]['samples_seconds'], [])
            self.assertEqual(result['programs'][application['key']]['samples_seconds'], [2.0])
            self.assertEqual(result['historical_attempts'], attempts[1:])
            self.assertEqual(result['retired_candidates'][control['key']], measured_control)
            self.assertEqual(result['attempts'][0]['seconds'], attempts[0]['seconds'])
            self.assertNotIn('runtime_environment', result['attempts'][0])
            self.assertEqual(result['guard_revisions'][-1]['prior_report_sha256'], sha(archive / 'timings-apl.json'))


if __name__ == '__main__':
    unittest.main()
