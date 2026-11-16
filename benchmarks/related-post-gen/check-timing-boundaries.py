#!/usr/bin/env python3
"""Check timer selection with synthetic counterexamples; these are not timings."""
import copy
from importlib.machinery import SourceFileLoader
from pathlib import Path
import unittest

comparison = SourceFileLoader(
    'related_comparison', str(Path(__file__).with_name('compare-implementations.py'))
).load_module()


def fixture():
    variants = []
    summary = {}
    samples = []
    for name, kind, boundary, processing, whole in [
        ('rust', 'upstream', 'post_json_including_setup', .008, .030),
        ('cold-pool', 'upstream', 'post_json_including_setup', .002, .022),
        ('serial', 'upstream', 'post_json_including_setup', .0018, .021),
        ('warm-pool', 'upstream', 'preinitialized_workers', .001, .020),
        ('input-prepared', 'upstream', 'processing_excluding_input_preparation', .0005, .021),
        ('unknown', 'upstream', 'unspecified', .0001, .025),
        ('mech', 'mech', 'post_json_including_setup', .003, .380),
    ]:
        variants.append(dict(name=name, kind=kind, calculation_boundary=boundary,
                             processing_boundary='Synthetic declared interval'))
        summary[name] = dict(median_seconds=processing)
        for index in range(3):
            row = dict(sample=index, variant=name, seconds=processing, wall_seconds=whole)
            if kind == 'mech':
                row.update(processing_plus_admission_seconds=.004, post_json_seconds=.0045)
            samples.append(row)
        if kind == 'mech':
            summary[name]['median_processing_plus_admission_seconds'] = .004
    return dict(reference='rust', implementations=variants, summary=summary, samples=samples)


class TimerBoundaries(unittest.TestCase):
    def test_whole_process_is_primary_even_with_matching_post_json_timers(self):
        report = fixture()
        comparison.add_timing_comparisons(report)
        self.assertEqual(report['timing_policy_version'], 3)
        self.assertEqual(report['primary_metric'], 'whole_process_over_fastest')
        self.assertEqual(report['primary_reference'], report['whole_process_reference'])
        self.assertEqual(report['secondary_metric'], 'post_json_over_fastest_matched_native')

    def test_preinitialization_preparation_and_unknown_timers_are_excluded(self):
        report = fixture()
        comparison.add_timing_comparisons(report)
        self.assertEqual(report['matched_native_reference']['variant'], 'serial')
        self.assertEqual(set(report['matched_native_reference']['eligible_variants']),
                         {'rust', 'cold-pool', 'serial'})
        for name in ['warm-pool', 'input-prepared', 'unknown']:
            self.assertNotIn('post_json_over_fastest_matched_native', report['summary'][name])

    def test_continuous_interval_includes_the_gap_between_phase_timers(self):
        report = fixture()
        comparison.add_timing_comparisons(report)
        self.assertAlmostEqual(report['summary']['mech']['post_json_over_fastest_matched_native'], 2.5)
        self.assertEqual(report['summary']['mech']['median_post_json_seconds'], .0045)

    def test_whole_process_includes_setup_even_for_a_warm_processing_timer(self):
        report = fixture()
        comparison.add_timing_comparisons(report)
        self.assertEqual(report['whole_process_reference']['variant'], 'warm-pool')
        self.assertAlmostEqual(report['summary']['mech']['whole_process_over_fastest'], 19)

    def test_undeclared_or_phase_sum_only_mech_falls_back_to_complete_process(self):
        for boundary in ['unspecified', 'processing_excluding_admission']:
            report = fixture()
            report['implementations'][-1]['calculation_boundary'] = boundary
            comparison.add_timing_comparisons(report)
            self.assertEqual(report['primary_metric'], 'whole_process_over_fastest')
            self.assertNotIn('post_json_over_fastest_matched_native', report['summary']['mech'])
        report = fixture()
        for variant in report['implementations']:
            variant.pop('calculation_boundary')
        comparison.add_timing_comparisons(report)
        self.assertIsNone(report['matched_native_reference'])

    def test_missing_continuous_interval_fails_closed(self):
        report = fixture()
        next(row for row in report['samples'] if row['variant'] == 'mech').pop('post_json_seconds')
        with self.assertRaises(KeyError):
            comparison.add_timing_comparisons(report)

    def test_median_uses_each_complete_invocation(self):
        report = fixture()
        rows = [row for row in report['samples'] if row['variant'] == 'mech']
        for row, continuous in zip(rows, [.011, .004, .011]):
            row['post_json_seconds'] = continuous
        comparison.add_timing_comparisons(report)
        self.assertEqual(report['summary']['mech']['median_post_json_seconds'], .011)

    def test_a_mech_win_keeps_the_external_denominator(self):
        report = fixture()
        for row in report['samples']:
            if row['variant'] == 'mech':
                row['post_json_seconds'] = .0009
        comparison.add_timing_comparisons(report)
        self.assertEqual(report['matched_native_reference']['variant'], 'serial')
        self.assertAlmostEqual(report['summary']['mech']['post_json_over_fastest_matched_native'], .5)
        self.assertEqual(report['primary_metric'], 'whole_process_over_fastest')
        self.assertGreater(report['summary']['mech']['whole_process_over_fastest'], 1)

    def test_reclassification_discards_stale_ratios(self):
        report = fixture()
        before = copy.deepcopy(report['samples'])
        comparison.add_timing_comparisons(report)
        report['implementations'][-1].pop('calculation_boundary')
        comparison.add_timing_comparisons(report)
        self.assertEqual(report['primary_metric'], 'whole_process_over_fastest')
        self.assertNotIn('median_post_json_seconds', report['summary']['mech'])
        self.assertNotIn('post_json_over_fastest_matched_native', report['summary']['mech'])
        self.assertEqual(before, report['samples'])

    def test_invalid_boundary_is_rejected(self):
        report = fixture()
        report['implementations'][0]['calculation_boundary'] = 'typo'
        with self.assertRaises(ValueError):
            comparison.add_timing_comparisons(report)


if __name__ == '__main__':
    unittest.main()
