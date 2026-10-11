#!/usr/bin/env python3
"""Pure NumPy EKF variants for a separately recorded implementation pilot.

These use the same matrix recurrence as numpy_batched.py. The alternatives
change NumPy's matrix-contraction primitive and array layout, not the EKF
equations, precision, or Joseph covariance update. There are no Python loops
over filters and no generated, compiled, or scalar-expanded custom kernel.

The unchanged numpy_batched.py remains the original campaign baseline.
"""

import argparse
import json
import time
from pathlib import Path

import numpy as np

from numpy_batched import BatchedEKF, DT, Q, R, load_inputs


VARIANTS = ("einsum-aos", "einsum-soa", "matmul-soa")


class VariantEKF(BatchedEKF):
    """Logical shapes stay (N,3) and (N,3,3), including in dump() output.

    AOS stores each filter's components together, as the original control
    does. SOA stores each component across filters contiguously. SOA's
    logical state/covariance properties are views of this storage; there is
    no per-turn packing or unpacking. Input storage is unchanged.
    """

    def __init__(self, instances, inputs=None, variant="einsum-soa"):
        if variant not in VARIANTS:
            raise ValueError(f"unknown variant {variant!r}; choose from {VARIANTS}")
        super().__init__(instances, inputs)
        self.variant = variant
        if variant.endswith("-soa"):
            retained = {"state", "covariance", "f", "g"}
            arrays = (
                "state", "covariance", "f", "g", "left", "predicted_p",
                "process_p", "a", "ap", "corrected_p", "measurement_p",
                "process_left", "predicted_state", "h", "pht", "gain",
                "state_correction", "innovation_variance_matrix",
            )
            for name in arrays:
                current = getattr(self, name)
                replacement = np.empty((*current.shape[1:], instances), dtype=np.float32)
                replacement = np.moveaxis(replacement, -1, 0)
                if name in retained:
                    replacement[:] = current
                setattr(self, name, replacement)
            self.innovation_variance = self.innovation_variance_matrix[:, 0, 0]
            self.ft = self.f.swapaxes(1, 2)
            self.gt = self.g.swapaxes(1, 2)
            self.at = self.a.swapaxes(1, 2)
            self.h_row = self.h[:, None, :]
            self.h_column = self.h[:, :, None]
            self.pht_column = self.pht[:, :, None]
            self.gain_row = self.gain[:, None, :]
            self.gain_column = self.gain[:, :, None]
        self._product = self._einsum_product if variant.startswith("einsum-") else self._matmul_product

    @staticmethod
    def _einsum_product(left, right, output):
        # Two operands: no contraction-path search or intermediates needed.
        # A shared 2-D right operand (Q) is broadcast across independent filters.
        subscripts = "nij,njk->nik" if right.ndim == 3 else "nij,jk->nik"
        np.einsum(subscripts, left, right, out=output, optimize=False)

    @staticmethod
    def _matmul_product(left, right, output):
        np.matmul(left, right, out=output)

    def step(self):
        """Return a complete unchecked turn; no integrity/rollback is claimed."""
        np.sin(self.state[:, 2], out=self.sin_theta)
        np.cos(self.state[:, 2], out=self.cos_theta)
        np.multiply(self.inputs["velocity"], DT, out=self.distance)
        np.multiply(self.distance, self.cos_theta, out=self.control)
        np.add(self.state[:, 0], self.control, out=self.predicted_state[:, 0])
        np.multiply(self.distance, self.sin_theta, out=self.control)
        np.add(self.state[:, 1], self.control, out=self.predicted_state[:, 1])
        np.multiply(self.inputs["angular_velocity"], DT, out=self.control)
        np.add(self.state[:, 2], self.control, out=self.predicted_state[:, 2])

        np.multiply(self.distance, self.sin_theta, out=self.f[:, 0, 2])
        np.negative(self.f[:, 0, 2], out=self.f[:, 0, 2])
        np.multiply(self.distance, self.cos_theta, out=self.f[:, 1, 2])
        np.multiply(self.cos_theta, DT, out=self.g[:, 0, 0])
        np.multiply(self.sin_theta, DT, out=self.g[:, 1, 0])
        self._product(self.f, self.covariance, self.left)
        self._product(self.left, self.ft, self.predicted_p)
        self._product(self.g, Q, self.process_left)
        self._product(self.process_left, self.gt, self.process_p)
        np.add(self.predicted_p, self.process_p, out=self.predicted_p)

        np.subtract(np.float32(140.0), self.predicted_state[:, 0], out=self.delta_x)
        np.subtract(np.float32(12.0), self.predicted_state[:, 1], out=self.delta_y)
        np.multiply(self.delta_x, self.delta_x, out=self.squared_range)
        np.multiply(self.delta_y, self.delta_y, out=self.squared_y)
        np.add(self.squared_range, self.squared_y, out=self.squared_range)
        np.arctan2(self.delta_y, self.delta_x, out=self.predicted_bearing)
        np.subtract(self.predicted_bearing, self.predicted_state[:, 2], out=self.predicted_bearing)
        np.subtract(self.inputs["bearing"], self.predicted_bearing, out=self.innovation)
        np.sin(self.innovation, out=self.sin_innovation)
        np.cos(self.innovation, out=self.cos_innovation)
        np.arctan2(self.sin_innovation, self.cos_innovation, out=self.innovation)
        np.divide(self.delta_y, self.squared_range, out=self.h[:, 0])
        np.negative(self.delta_x, out=self.h[:, 1])
        np.divide(self.h[:, 1], self.squared_range, out=self.h[:, 1])
        self.h[:, 2] = np.float32(-1.0)
        self._product(self.predicted_p, self.h_column, self.pht_column)
        self._product(self.h_row, self.pht_column, self.innovation_variance_matrix)
        np.add(self.innovation_variance, R, out=self.innovation_variance)
        np.divide(self.pht, self.innovation_variance[:, None], out=self.gain)
        np.multiply(self.gain, self.innovation[:, None], out=self.state_correction)
        np.add(self.predicted_state, self.state_correction, out=self.state)

        np.multiply(self.gain_column, self.h_row, out=self.a)
        np.subtract(self.identity, self.a, out=self.a)
        self._product(self.a, self.predicted_p, self.ap)
        self._product(self.ap, self.at, self.corrected_p)
        np.multiply(self.gain_column, self.gain_row, out=self.measurement_p)
        np.multiply(self.measurement_p, R, out=self.measurement_p)
        np.add(self.corrected_p, self.measurement_p, out=self.covariance)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--variant", choices=VARIANTS, required=True)
    parser.add_argument("--instances", type=int, default=10_000)
    parser.add_argument("--turns", type=int, default=20)
    parser.add_argument("--warmup", type=int, default=5)
    parser.add_argument("--input", type=Path)
    parser.add_argument("--dump", type=Path)
    args = parser.parse_args()
    if args.instances < 1 or args.turns < 1 or args.warmup < 0:
        parser.error("instances and turns must be positive; warmup must be nonnegative")
    supplied = load_inputs(args.input, args.instances) if args.input else None
    ekf = VariantEKF(args.instances, supplied, args.variant)
    for _ in range(args.warmup):
        ekf.step()
    ekf.reset()
    started = time.perf_counter()
    for _ in range(args.turns):
        ekf.step()
    elapsed = time.perf_counter() - started
    if args.dump:
        ekf.dump(args.dump)
    print(json.dumps({
        "implementation": f"NumPy {args.variant}",
        "variant": args.variant,
        "layout": "component-major" if args.variant.endswith("-soa") else "filter-major",
        "contraction": "einsum" if args.variant.startswith("einsum-") else "matmul",
        "numpy_version": np.__version__,
        "dtype": "float32",
        "checked": False,
        "instances": args.instances,
        "turns": args.turns,
        "warmup": args.warmup,
        "elapsed_s": elapsed,
        "filter_turns_per_second": args.instances * args.turns / elapsed,
        "checksum": ekf.checksum(),
    }, sort_keys=True))


if __name__ == "__main__":
    main()
