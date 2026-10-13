"""Timing-boundary qualification tests; no benchmark processes are launched."""
import importlib.util
import json
from pathlib import Path
import unittest

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('source_modes_preparation_test', HERE / 'prepare.py')
preparation = importlib.util.module_from_spec(spec)
spec.loader.exec_module(preparation)


class TimingProtocolTests(unittest.TestCase):
    def result(self, timing=None, wall=2.0):
        timing = {'prepare_seconds': 0.25, 'kernel_seconds': 1.5} if timing is None else timing
        return dict(seconds=wall, stderr='TIMING ' + json.dumps(timing) + '\n')

    def test_jit_retains_fresh_process_time(self):
        result = preparation.source_timing(self.result(), 'jit')
        self.assertEqual(result['reported_seconds'], 2.0)
        self.assertEqual(result['wall_seconds'], 2.0)

    def test_interpreted_scores_loaded_execution_and_retains_prepare_wall(self):
        result = preparation.source_timing(self.result(), 'resident')
        self.assertEqual(result, dict(wall_seconds=2.0, prepare_seconds=0.25,
                                     kernel_seconds=1.5, reported_seconds=1.5))

    def test_missing_or_duplicate_timing_record_is_fatal(self):
        for stderr in ('', self.result()['stderr'] * 2):
            with self.subTest(stderr=stderr), self.assertRaises(AssertionError):
                preparation.source_timing(dict(seconds=2.0, stderr=stderr), 'resident')

    def test_wrong_fields_or_nonfinite_negative_or_boolean_values_are_fatal(self):
        bad = [{'kernel_seconds': 1.0}, {'prepare_seconds': 0, 'kernel_seconds': 1, 'extra': 2}]
        for key in ('prepare_seconds', 'kernel_seconds'):
            for value in (float('nan'), float('inf'), -1, True):
                bad.append({'prepare_seconds': 0.25, 'kernel_seconds': 1.5} | {key: value})
        bad.append({'prepare_seconds': 0.25, 'kernel_seconds': 0})
        for timing in bad:
            with self.subTest(timing=timing), self.assertRaises(AssertionError):
                preparation.source_timing(self.result(timing), 'resident')

    def test_preparation_and_execution_must_fit_wall(self):
        with self.assertRaises(AssertionError):
            preparation.source_timing(self.result(wall=1.0), 'jit')

    def test_wall_must_be_finite_positive_and_mode_explicit(self):
        for value in (float('nan'), float('inf'), -1, 0, True):
            with self.subTest(wall=value), self.assertRaises(AssertionError):
                preparation.source_timing(self.result(wall=value), 'jit')
        with self.assertRaises(AssertionError):
            preparation.source_timing(self.result(), 'aot-control')


if __name__ == '__main__':
    unittest.main()
