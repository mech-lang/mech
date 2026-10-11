"""Correctness checks for the independent pure-NumPy implementation pilot."""

import tempfile
import unittest
from pathlib import Path

import numpy as np

from numpy_batched import BatchedEKF, INITIAL_COVARIANCE, INITIAL_STATE, init_inputs
from numpy_variants import VARIANTS, VariantEKF


class VariantTests(unittest.TestCase):
    def test_full_outputs_match_baseline(self):
        for instances in (1, 17, 257):
            inputs = init_inputs(instances)
            baseline = BatchedEKF(instances, inputs)
            variants = [VariantEKF(instances, inputs, name) for name in VARIANTS]
            for turn in range(1, 101):
                baseline.step()
                for candidate in variants:
                    candidate.step()
                if turn in (1, 4, 20, 100):
                    for candidate in variants:
                        with self.subTest(instances=instances, turn=turn, variant=candidate.variant):
                            # Same predeclared elementwise bound as the original
                            # cross-backend benchmark, including near-zero entries.
                            np.testing.assert_allclose(candidate.state, baseline.state, atol=1e-4, rtol=1e-5)
                            np.testing.assert_allclose(candidate.covariance, baseline.covariance, atol=1e-4, rtol=1e-5)
                            self.assertEqual(candidate.state.dtype, np.dtype("float32"))
                            self.assertEqual(candidate.covariance.dtype, np.dtype("float32"))
                            self.assertTrue(np.isfinite(candidate.state).all())
                            self.assertTrue(np.isfinite(candidate.covariance).all())
                            self.assertTrue((np.diagonal(candidate.covariance, axis1=1, axis2=2) > 0).all())

    def test_soa_is_component_contiguous_without_repacking(self):
        for variant in ("einsum-soa", "matmul-soa"):
            candidate = VariantEKF(17, variant=variant)
            self.assertEqual(candidate.state.strides[0], 4)
            self.assertEqual(candidate.covariance.strides[0], 4)
            state_address = candidate.state.__array_interface__["data"][0]
            covariance_address = candidate.covariance.__array_interface__["data"][0]
            candidate.step()
            self.assertEqual(candidate.state.__array_interface__["data"][0], state_address)
            self.assertEqual(candidate.covariance.__array_interface__["data"][0], covariance_address)

    def test_dump_and_reset(self):
        for variant in VARIANTS:
            candidate = VariantEKF(7, variant=variant)
            candidate.step()
            with tempfile.TemporaryDirectory() as temporary:
                destination = Path(temporary) / "state.f32"
                candidate.dump(destination)
                values = np.fromfile(destination, dtype="<f4")
                np.testing.assert_array_equal(values[:21].reshape(7, 3), candidate.state)
                np.testing.assert_array_equal(values[21:].reshape(7, 3, 3), candidate.covariance)
            candidate.reset()
            np.testing.assert_array_equal(candidate.state, np.broadcast_to(INITIAL_STATE, (7, 3)))
            np.testing.assert_array_equal(candidate.covariance, np.broadcast_to(INITIAL_COVARIANCE, (7, 3, 3)))


if __name__ == "__main__":
    unittest.main()
