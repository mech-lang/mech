import importlib.util
from pathlib import Path
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / "benchmark-polybench.py"
SPEC = importlib.util.spec_from_file_location("benchmark_polybench", SCRIPT)
BENCHMARK = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BENCHMARK)


class BenchmarkPolybenchTests(unittest.TestCase):
    def test_aggregate_weights_kernels_equally_and_keeps_regressions(self):
        # Two radically different runtime scales must not let the large case
        # dominate the primary geometric mean. One kernel really regresses.
        def run(mech, c, reference):
            return {"comparisons": {
                "kernel": {"mech": {"median_seconds": mech},
                           "reference": {"median_seconds": reference},
                           "median_mech_over_c": mech / c},
                "fresh": {"median_mech_over_c": mech / (2 * c)},
            }}
        runs = {"small": run(0.002, 0.004, 0.001), "large": run(1, 0.125, 8)}
        baseline = {"runs": {
            "small-mini": {"comparisons": {"kernel": {"mech": {"median_seconds": 0.001}}}},
            "large-mini": {"comparisons": {"kernel": {"mech": {"median_seconds": 8}}}},
        }}
        result = BENCHMARK.aggregate(["small", "large"], runs, baseline)
        self.assertAlmostEqual(result["geomean_speedup_from_recorded_baseline"], 2)
        self.assertAlmostEqual(result["geomean_speedup_from_contemporaneous_baseline"], 2)
        self.assertAlmostEqual(result["geomean_mech_over_c_kernel"], 2)
        self.assertAlmostEqual(result["geomean_mech_over_c_fresh"], 1)
        self.assertEqual(result["minimum_speedup"], 0.5)
        self.assertEqual(result["regressions_over_five_percent"], 1)
        self.assertEqual(result["faster_than_c_kernel"], 1)
        self.assertEqual(result["worst_mech_over_c_kernel"], 8)
        self.assertAlmostEqual(result["sum_mech_median_seconds"], 1.002)
        with self.assertRaises(KeyError):
            BENCHMARK.aggregate(["small", "large", "missing"], runs, baseline)

    def test_validation_rejects_nonfinite_wrong_length_and_wrong_values(self):
        for actual, expected in [
            ([float("nan")], [0]), ([float("inf")], [float("inf")]),
            (["NaN"], [0]), ([1], [1, 2]), ([1.01], [1]),
        ]:
            with self.subTest(actual=actual, expected=expected):
                with self.assertRaises(ValueError):
                    BENCHMARK.validate(actual, expected)
        self.assertEqual(BENCHMARK.validate([-0.0, 1 + 1e-11], [0.0, 1])["count"], 2)

    def test_frozen_split_covers_all_sources_once_with_verified_hashes(self):
        spec, cases = BENCHMARK.protocol()
        self.assertEqual(set(spec["optimization"]) | set(spec["holdout"]), set(cases))
        self.assertFalse(set(spec["optimization"]) & set(spec["holdout"]))
        for group in [("2mm", "3mm"), ("correlation", "covariance"),
                      ("lu", "ludcmp"), ("syrk", "syr2k"), ("jacobi-1d", "jacobi-2d")]:
            self.assertTrue(any(set(group) <= set(spec[partition])
                                for partition in ("optimization", "holdout")))


if __name__ == "__main__":
    unittest.main()
