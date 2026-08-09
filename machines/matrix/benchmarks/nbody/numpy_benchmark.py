#!/usr/bin/env python3
"""Whole-program NumPy implementation of the Benchmarks Game n-body task."""

import sys

import numpy as np

PI = 3.141592653589793
SOLAR_MASS = 4.0 * PI * PI
DAYS_PER_YEAR = 365.24
DT = 0.01
PAIR_I = np.array([0, 0, 0, 0, 1, 1, 1, 2, 2, 3], dtype=np.intp)
PAIR_J = np.array([1, 2, 3, 4, 2, 3, 4, 3, 4, 4], dtype=np.intp)


def initial_system():
    positions = np.array(
        [
            [0.0, 0.0, 0.0],
            [4.84143144246472090, -1.16032004402742839, -0.103622044471123109],
            [8.34336671824457987, 4.12479856412430479, -0.403523417114321381],
            [12.8943695621391310, -15.1111514016986312, -0.223307578892655734],
            [15.3796971148509165, -25.9193146099879641, 0.179258772950371181],
        ],
        dtype=np.float64,
        order="C",
    )
    velocities = np.array(
        [
            [0.0, 0.0, 0.0],
            [0.00166007664274403694, 0.00769901118419740425, -0.0000690460016972063023],
            [-0.00276742510726862411, 0.00499852801234917238, 0.0000230417297573763929],
            [0.00296460137564761618, 0.00237847173959480950, -0.0000296589568540237556],
            [0.00268067772490389322, 0.00162824170038242295, -0.000095159225451971587],
        ],
        dtype=np.float64,
        order="C",
    )
    velocities *= DAYS_PER_YEAR
    masses = (
        np.array(
            [1.0, 0.000954791938424326609, 0.000285885980666130812, 0.0000436624404335156298, 0.0000515138902046611451],
            dtype=np.float64,
        )
        * SOLAR_MASS
    )
    velocities[0] = -(velocities * masses[:, None]).sum(axis=0) / SOLAR_MASS
    return positions, velocities, masses


def energy(positions, velocities, masses):
    kinetic = 0.5 * np.sum(masses * np.einsum("ij,ij->i", velocities, velocities))
    delta = positions[PAIR_I] - positions[PAIR_J]
    potential = np.sum(masses[PAIR_I] * masses[PAIR_J] / np.sqrt(np.einsum("ij,ij->i", delta, delta)))
    return float(kinetic - potential)


def advance(positions, velocities, masses, steps):
    incidence = np.zeros((10, 5), dtype=np.float64)
    incidence[np.arange(10), PAIR_I] = 1.0
    incidence[np.arange(10), PAIR_J] = -1.0
    velocity_weights = np.zeros((5, 10), dtype=np.float64)
    velocity_weights[PAIR_I, np.arange(10)] = -masses[PAIR_J]
    velocity_weights[PAIR_J, np.arange(10)] = masses[PAIR_I]
    delta = np.empty((10, 3), dtype=np.float64)
    squared_distance = np.empty(10, dtype=np.float64)
    magnitude = np.empty(10, dtype=np.float64)
    weighted = np.empty((5, 10), dtype=np.float64)
    acceleration = np.empty((5, 3), dtype=np.float64)
    position_delta = np.empty((5, 3), dtype=np.float64)

    for _ in range(steps):
        np.matmul(incidence, positions, out=delta)
        np.einsum("ij,ij->i", delta, delta, out=squared_distance)
        np.sqrt(squared_distance, out=magnitude)
        np.multiply(magnitude, squared_distance, out=magnitude)
        np.divide(DT, magnitude, out=magnitude)
        np.multiply(velocity_weights, magnitude, out=weighted)
        np.matmul(weighted, delta, out=acceleration)
        np.add(velocities, acceleration, out=velocities)
        np.multiply(velocities, DT, out=position_delta)
        np.add(positions, position_delta, out=positions)


def main():
    steps = int(sys.argv[1]) if len(sys.argv) > 1 else 1_000
    positions, velocities, masses = initial_system()
    print(f"{energy(positions, velocities, masses):.9f}")
    advance(positions, velocities, masses, steps)
    print(f"{energy(positions, velocities, masses):.9f}")


if __name__ == "__main__":
    main()
