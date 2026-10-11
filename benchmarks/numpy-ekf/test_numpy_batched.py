"""Small correctness tests, not a throughput campaign.

Run with: python -m unittest discover -s benchmarks/numpy-ekf -p test_numpy_batched.py
"""

import importlib.util
import tempfile
import unittest
from pathlib import Path

import numpy as np

from numpy_batched import BatchedEKF, INITIAL_COVARIANCE, INITIAL_STATE, init_inputs, load_inputs


ARCHIVE = Path(__file__).resolve().parents[1] / "archive/compute/parallel-ekf/numpy_scalar.py"
SPEC = importlib.util.spec_from_file_location("archived_numpy_scalar", ARCHIVE)
SCALAR = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(SCALAR)


class BatchedEKFTests(unittest.TestCase):
    def test_agrees_with_sequential_control(self):
        # Allow ordinary f32 differences from vectorized transcendentals and dot
        # reductions, without accepting a different recurrence or large drift.
        # This bound is elementwise: abs(candidate-reference) <= atol+rtol*abs(reference).
        instances = 17
        inputs = init_inputs(instances)
        batched = BatchedEKF(instances, inputs)
        state = np.broadcast_to(INITIAL_STATE, (instances, 3)).copy()
        covariance = np.broadcast_to(INITIAL_COVARIANCE, (instances, 3, 3)).copy()
        scratch = SCALAR.Scratch()
        for turn in range(1, 101):
            batched.step()
            for lane in range(instances):
                SCALAR.step(state[lane], covariance[lane], inputs["velocity"][lane],
                            inputs["angular_velocity"][lane], inputs["bearing"][lane], scratch)
            if turn in (1, 4, 20, 100):
                with self.subTest(turn=turn):
                    np.testing.assert_allclose(batched.state, state, rtol=2e-5, atol=2e-5)
                    np.testing.assert_allclose(batched.covariance, covariance, rtol=2e-5, atol=2e-5)
                    self.assertEqual(batched.state.dtype, np.dtype("float32"))
                    self.assertEqual(batched.covariance.dtype, np.dtype("float32"))
                    self.assertTrue(np.isfinite(batched.state).all())
                    self.assertTrue(np.isfinite(batched.covariance).all())
                    self.assertTrue((np.diagonal(batched.covariance, axis1=1, axis2=2) > 0).all())

    def test_reset_and_single_filter(self):
        ekf = BatchedEKF(1)
        ekf.step()
        self.assertFalse(np.array_equal(ekf.state[0], INITIAL_STATE))
        ekf.reset()
        np.testing.assert_array_equal(ekf.state[0], INITIAL_STATE)
        np.testing.assert_array_equal(ekf.covariance[0], INITIAL_COVARIANCE)

    def test_shared_input_and_state_files(self):
        inputs = init_inputs(7)
        with tempfile.TemporaryDirectory() as temporary:
            input_path = Path(temporary) / "inputs.f32"
            np.concatenate(list(inputs.values())).astype("<f4").tofile(input_path)
            loaded = load_inputs(input_path, 7)
            for name in inputs:
                np.testing.assert_array_equal(loaded[name], inputs[name])
            ekf = BatchedEKF(7, loaded)
            ekf.step()
            state_path = Path(temporary) / "state.f32"
            ekf.dump(state_path)
            values = np.fromfile(state_path, dtype="<f4")
            self.assertEqual(values.size, 7 * 12)
            np.testing.assert_array_equal(values[:21].reshape(7, 3), ekf.state)
            np.testing.assert_array_equal(values[21:].reshape(7, 3, 3), ekf.covariance)

    def test_rejects_invalid_inputs(self):
        with self.assertRaises(ValueError):
            BatchedEKF(0)
        inputs = init_inputs(2)
        inputs["bearing"] = inputs["bearing"][:1]
        with self.assertRaises(ValueError):
            BatchedEKF(2, inputs)


if __name__ == "__main__":
    unittest.main()
