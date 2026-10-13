"""Unhooked metadata fixtures for the next reviewed, locked audit pass.

These fixtures use synthetic JSON identities only. They run no benchmarks,
normalizers, lexers, compilers or hashes, and never read historical job outputs.
"""
from copy import deepcopy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest


HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location('comparison_metadata_fixtures', HERE / 'comparisons.py')
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)

FAST = '1' * 64
SMALL = '2' * 64
SNAPSHOT_PIN = 'a' * 64
NUMERATOR = '9' * 64
ZIG_SUMMARY = 'zig-odin-roc-elm-comparison/20261010/results/summary.json'
REMAINING_SUMMARY = 'remaining-languages-comparison/20261011/results/summary.json'


class FrozenComparisonMetadata(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.root = Path(self.directory.name)

    def tearDown(self):
        self.directory.cleanup()

    def write_json(self, relative, data):
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(data))

    def fixture(self, summary_path, benchmark='nbody', report='native-timings-v2.json',
                source_report='remaining-languages-comparison/20261011/source-audit-v2.json',
                fast_loc=25, small_loc=20):
        schema = MODULE.FROZEN_SUMMARY_SCHEMAS[summary_path]
        fast_key, small_key = benchmark + ':historical-fast', benchmark + ':historical-small'
        key = benchmark + ':foreign-source'
        control = dict(key=benchmark + ':historical-control', chart_key=fast_key,
            seconds=2.0, fastest_loc=fast_loc, smallest_loc=small_loc, source_sha256=FAST)
        if schema['selector'] == 'benchmark':
            selector = benchmark
            control.update(smallest_key=small_key, smallest_source_sha256=[SMALL])
        else:
            selector = report + ':' + benchmark
            control['timing_report'] = 'results/' + report
        summary = dict(status='passed', controls={selector:control},
                       **{schema['snapshot_pin']:SNAPSHOT_PIN})
        snapshot = dict(benchmarks=[dict(key=benchmark, groups=[
            dict(language='mech', fast=fast_key, small=small_key)])],
            programs={fast_key:dict(key=fast_key, language='mech', loc=fast_loc, files=[FAST]),
                      small_key:dict(key=small_key, language='mech', loc=small_loc, files=[SMALL])},
            files={FAST:dict(path='frozen/fast.mec', sha256=FAST),
                   SMALL:dict(path='frozen/small.mec', sha256=SMALL)})
        paired = dict(seconds=2.0, fastest_loc=fast_loc, smallest_loc=small_loc,
                      fastest_loc_x_mech=74 / fast_loc, smallest_loc_x_mech=74 / small_loc,
                      time_x_mech=3.0)
        program = dict(key=key, benchmark=benchmark, language='foreign', loc=74,
            report=summary_path.rsplit('/results/', 1)[0] + '/results/' + report,
            source_report=source_report, loc_report=summary_path, files=[NUMERATOR],
            mech_comparison=paired)
        current = dict(programs={key:program},
            files={NUMERATOR:dict(path='current/foreign.source', sha256=NUMERATOR)})
        self.write_json(summary_path, summary)
        self.write_json(schema['snapshot'], snapshot)
        return current, summary, snapshot

    def bindings(self, current):
        before = deepcopy(current)
        result = MODULE.collect_display_comparisons(self.root, current)
        self.assertEqual(current, before, 'Producer fields must remain immutable')
        self.assertEqual(result['unresolved'], [])
        self.assertEqual(len(result['comparisons']), 1)
        bindings = result['comparisons'][0]['bindings']
        self.assertEqual({b['field_path'][-1]:b['denominator_source']['sha256'] for b in bindings},
                         {'fastest_loc':FAST, 'smallest_loc':SMALL})
        return result, bindings

    def test_zig_family_historical_profiles(self):
        current, _, _ = self.fixture(ZIG_SUMMARY, report='timings-complete.json',
            source_report='zig-odin-roc-elm-comparison/20261010/source-audit-complete.json')
        result, bindings = self.bindings(current)
        for binding in bindings:
            evidence = binding['denominator_source']['identity_evidence']
            self.assertEqual(evidence['control_snapshot'], MODULE.FROZEN_SUMMARY_SCHEMAS[ZIG_SUMMARY]['snapshot'])
            self.assertEqual(evidence['expected_snapshot_sha256'], SNAPSHOT_PIN)
            self.assertEqual(evidence['control_selector'], 'nbody')
        self.assertIn(ZIG_SUMMARY, result['report_paths'])

    def test_remaining_family_exact_report_selector(self):
        current, _, _ = self.fixture(REMAINING_SUMMARY)
        _, bindings = self.bindings(current)
        self.assertTrue(all(b['denominator_source']['identity_evidence']['control_selector'] ==
                            'native-timings-v2.json:nbody' for b in bindings))

    def test_ada_repair_keeps_distinct_source_pins_even_when_locs_equal(self):
        current, _, _ = self.fixture(REMAINING_SUMMARY, benchmark='regexredux',
            report='ada-regex-repair.json', fast_loc=24, small_loc=24,
            source_report='remaining-languages-comparison/20261011/ada-regex-repair-source-audit.json')
        _, bindings = self.bindings(current)
        self.assertTrue(all(b['denominator_source']['identity_evidence']['control_selector'] ==
                            'ada-regex-repair.json:regexredux' for b in bindings))

    def test_later_compiler_override_preserves_paired_source_provenance(self):
        current, _, _ = self.fixture(ZIG_SUMMARY, report='timings-complete.json')
        program = next(iter(current['programs'].values()))
        old_report = program['report']
        program.update(report='nbody/experiments/20261011-shared-products/replication-50m.json',
            source_report='nbody/experiments/20261011-shared-products/summary.json',
            previous_measurement=dict(report=old_report, seconds=6.0))
        _, bindings = self.bindings(current)
        self.assertTrue(all(b['denominator_source']['identity_evidence']['producer_timing_report'] == old_report
                            for b in bindings))

    def test_rejects_wrong_historical_fast_source_pin(self):
        current, summary, _ = self.fixture(ZIG_SUMMARY)
        summary['controls']['nbody']['source_sha256'] = '3' * 64
        self.write_json(ZIG_SUMMARY, summary)
        with self.assertRaisesRegex(AssertionError, 'Historical fast source pin differs'):
            MODULE.collect_display_comparisons(self.root, current)

    def test_rejects_pair_count_mismatch_without_number_based_source_inference(self):
        current, _, _ = self.fixture(REMAINING_SUMMARY)
        next(iter(current['programs'].values()))['mech_comparison']['smallest_loc'] = 999
        with self.assertRaisesRegex(AssertionError, 'Displayed paired LOC'):
            MODULE.collect_display_comparisons(self.root, current)


if __name__ == '__main__':
    unittest.main()
