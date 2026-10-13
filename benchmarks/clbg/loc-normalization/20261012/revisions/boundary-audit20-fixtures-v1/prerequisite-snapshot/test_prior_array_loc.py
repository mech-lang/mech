"""Unhooked synthetic metadata fixtures for source-specific prior array policies.

Source SHA verification is the caller's separate responsibility and is mocked
here with exact registered identities. No source files, hashes, lexers, runtime
programs or historical audits are executed by these fixtures.
"""
from copy import deepcopy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest


HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location('prior_array_metadata_fixtures', HERE / 'prior_array_loc.py')
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class PriorArrayMetadata(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.root = Path(self.directory.name)

    def tearDown(self):
        self.directory.cleanup()

    def write_json(self, relative, data):
        path = Path(relative)
        path = path if path.is_absolute() else self.root / path
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(data))

    def fixture(self, key):
        language, sha, loc, policy, report = MODULE.SOURCES[key]
        source = str(self.root / 'synthetic' / ('app.apl' if language == 'apl' else 'app.ua'))
        field = 'loc_rule' if language == 'apl' else 'loc_review'
        row = dict(key=key, language=language, source=source, source_sha256=sha,
                   loc=loc, **{field:policy})
        if language == 'uiua':
            row.update(physical_loc=loc, loc_expansions=[])
        current = dict(key=key, language=language, loc=loc, physical_loc=loc,
            loc_expansions=[], loc_metric='statement-normalized', source_report=report, files=[sha])
        if report == MODULE.MANDEL_MANIFEST:
            producer = dict(status='prepared-awaiting-root-review', profiles=[deepcopy(row)], pins={source:sha})
            current['study_manifest_sha256'] = MODULE.MANIFEST_PINS['mandelbrot-study']
            self.write_json(report, producer)
            manifest = producer
            manifest_path = report
        else:
            manifest_path = MODULE.COHORT + '/' + language + '-manifest.json'
            manifest = dict(status='prepared', language=language, programs=[deepcopy(row)])
            producer = dict(status='reviewed', language=language, programs=[deepcopy(row)],
                pins={source:sha}, manifest=manifest_path, manifest_sha256=MODULE.MANIFEST_PINS[language])
            self.write_json(report, producer)
            self.write_json(manifest_path, manifest)
        # These are metadata fixtures, not cryptographic fixtures. The caller's
        # expected-byte verification is mocked; no actual digest is computed.
        producer_pin = MODULE.PRODUCER_PINS[report]
        archived = (deepcopy(producer) if report == MODULE.MANDEL_MANIFEST else
                    dict(source_audit=str(self.root / report), source_audit_sha256=producer_pin['sha256']))
        receipt = dict(snapshots={producer_pin['receipt_snapshot_key']:dict(
            snapshot='benchmarks/clbg/' + producer_pin['archive'], sha256=producer_pin['archive_sha256'])})
        self.write_json(producer_pin['archive'], archived)
        self.write_json(MODULE.RECEIPT, receipt)
        return current, producer, manifest, manifest_path

    def test_exact_uiua_source_policy_is_preserved_without_generic_certificate(self):
        current, _, _, _ = self.fixture('fasta:fasta-uiua-compact')
        before = deepcopy(current)
        metric = MODULE.preserve('Caller verified bytes separately', 'uiua', current['files'][0], current, self.root)
        self.assertEqual(metric['normalized_loc'], 22)
        self.assertEqual(metric['review_status'], 'prior-source-audit-preserved')
        self.assertEqual(metric['loc_expansions'], [])
        self.assertEqual(metric['provenance']['expected_producer_sha256'],
                         MODULE.PRODUCER_PINS[current['source_report']]['sha256'])
        self.assertIn(str(self.root / MODULE.RECEIPT), metric['provenance']['provenance_pins'])
        self.assertEqual(current, before)

    def test_unknown_source_identity_is_not_grandfathered(self):
        current, _, _, _ = self.fixture('fasta:fasta-uiua-compact')
        self.assertIsNone(MODULE.preserve('', 'uiua', '0' * 64, current, self.root))

    def test_changed_producer_policy_is_not_preserved(self):
        current, producer, _, _ = self.fixture('fasta:fasta-uiua-compact')
        producer['programs'][0]['loc_review'] = 'Raw physical LOC without independent-binding review'
        self.write_json(current['source_report'], producer)
        self.assertIsNone(MODULE.preserve('', 'uiua', current['files'][0], current, self.root))

    def test_changed_manifest_source_pin_is_not_preserved(self):
        current, _, manifest, manifest_path = self.fixture('fasta:fasta-uiua-compact')
        manifest['programs'][0]['source_sha256'] = '0' * 64
        self.write_json(manifest_path, manifest)
        self.assertIsNone(MODULE.preserve('', 'uiua', current['files'][0], current, self.root))

    def test_archived_producer_pin_mismatch_is_not_preserved(self):
        current, _, _, _ = self.fixture('fasta:fasta-uiua-compact')
        producer_pin = MODULE.PRODUCER_PINS[current['source_report']]
        self.write_json(producer_pin['archive'], dict(
            source_audit=str(self.root / current['source_report']), source_audit_sha256='0' * 64))
        self.assertIsNone(MODULE.preserve('', 'uiua', current['files'][0], current, self.root))

    def test_mandelbrot_profile_requires_its_frozen_study_manifest_pin(self):
        current, _, _, _ = self.fixture('mandelbrot:uiua-parallel-study:whole16-parallel')
        metric = MODULE.preserve('', 'uiua', current['files'][0], current, self.root)
        self.assertEqual(metric['normalized_loc'], 16)
        current['study_manifest_sha256'] = '0' * 64
        self.assertIsNone(MODULE.preserve('', 'uiua', current['files'][0], current, self.root))

    def test_apl_closing_lines_are_only_proposals_and_quoted_comments_are_protected(self):
        current, _, _, _ = self.fixture('spectralnorm:spectralnorm-apl-compact')
        loc = current['loc']
        lines = ['⍝ comment only', '∇ result←Fn', "s←'quoted⍝marker and ''∇''' ⍝ actual comment"]
        lines.extend('x←1' for _ in range(loc - 2))
        lines.extend(['∇ ⍝ written closing line', ''])
        metric = MODULE.preserve('\n'.join(lines), 'apl', current['files'][0], current, self.root)
        self.assertIsNone(metric['normalized_loc'])
        self.assertEqual(metric['proposed_normalized_loc'], loc + 1)
        self.assertEqual(metric['physical_loc'], loc + 1)
        self.assertEqual(metric['extra_closing_lines'], 1)
        self.assertEqual(metric['closing_line_increments'][0]['line'], loc + 2)
        self.assertEqual(metric['review_status'], 'needs-explicit-closing-line-review')
        self.assertEqual(current['loc'], loc)

    def test_apl_prior_count_mismatch_never_supplies_a_proposed_count(self):
        current, _, _, _ = self.fixture('spectralnorm:spectralnorm-apl-compact')
        metric = MODULE.preserve('∇\n', 'apl', current['files'][0], current, self.root)
        self.assertIsNone(metric['normalized_loc'])
        self.assertIsNone(metric['proposed_normalized_loc'])
        self.assertFalse(metric['prior_nonclosing_count_matches'])


if __name__ == '__main__':
    unittest.main()
