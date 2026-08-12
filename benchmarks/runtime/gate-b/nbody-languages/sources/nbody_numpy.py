"""NumPy form of the same fixed-pair matrix program used by Mech AOT."""

import gc
import sys
import time

import numpy as np


PI = 3.14159265358979323
SOLAR_MASS = 4 * PI * PI
DAYS_PER_YEAR = 365.24
DT = 0.01

POSITIONS = np.array(
    [
        [0.0, 0.0, 0.0],
        [4.84143144246472090e00, -1.16032004402742839e00, -1.03622044471123109e-01],
        [8.34336671824457987e00, 4.12479856412430479e00, -4.03523417114321381e-01],
        [1.28943695621391310e01, -1.51111514016986312e01, -2.23307578892655734e-01],
        [1.53796971148509165e01, -2.59193146099879641e01, 1.79258772950371181e-01],
    ],
    dtype=np.float64,
)
VELOCITIES = np.array(
    [
        [0.0, 0.0, 0.0],
        [1.66007664274403694e-03, 7.69901118419740425e-03, -6.90460016972063023e-05],
        [-2.76742510726862411e-03, 4.99852801234917238e-03, 2.30417297573763929e-05],
        [2.96460137564761618e-03, 2.37847173959480950e-03, -2.96589568540237556e-05],
        [2.68067772490389322e-03, 1.62824170038242295e-03, -9.51592254519715870e-05],
    ],
    dtype=np.float64,
) * DAYS_PER_YEAR
MASSES = np.array(
    [
        1.0,
        9.54791938424326609e-04,
        2.85885980666130812e-04,
        4.36624404335156298e-05,
        5.15138902046611451e-05,
    ],
    dtype=np.float64,
) * SOLAR_MASS

PAIR_LEFT = np.array([0, 0, 0, 0, 1, 1, 1, 2, 2, 3])
PAIR_RIGHT = np.array([1, 2, 3, 4, 2, 3, 4, 3, 4, 4])
LEFT_INCIDENCE = np.zeros((5, 10), dtype=np.float64)
RIGHT_INCIDENCE = np.zeros((5, 10), dtype=np.float64)
LEFT_INCIDENCE[PAIR_LEFT, np.arange(10)] = 1.0
RIGHT_INCIDENCE[PAIR_RIGHT, np.arange(10)] = 1.0


def offset_momentum(velocities):
    velocities[0] = -(velocities[1:] * MASSES[1:, None]).sum(axis=0) / SOLAR_MASS


def advance(positions, velocities, turns):
    for _ in range(turns):
        delta = positions[PAIR_LEFT] - positions[PAIR_RIGHT]
        distance_squared = np.sum(delta * delta, axis=1)
        magnitude = DT * np.power(distance_squared, -1.5)
        weighted = delta * magnitude[:, None]
        velocities += (
            RIGHT_INCIDENCE @ (weighted * MASSES[PAIR_LEFT, None])
            - LEFT_INCIDENCE @ (weighted * MASSES[PAIR_RIGHT, None])
        )
        positions += velocities * DT


def energy(positions, velocities):
    kinetic = 0.5 * np.sum(MASSES[:, None] * velocities * velocities)
    delta = positions[PAIR_LEFT] - positions[PAIR_RIGHT]
    potential = np.sum(
        MASSES[PAIR_LEFT]
        * MASSES[PAIR_RIGHT]
        / np.sqrt(np.sum(delta * delta, axis=1))
    )
    return float(kinetic - potential)


def main(turns):
    positions = POSITIONS.copy()
    velocities = VELOCITIES.copy()
    offset_momentum(velocities)
    initial_energy = energy(positions, velocities)
    gc.collect()
    collections_before = sum(generation["collections"] for generation in gc.get_stats())
    started = time.perf_counter_ns()
    advance(positions, velocities, turns)
    elapsed_ns = time.perf_counter_ns() - started
    collections_after = sum(generation["collections"] for generation in gc.get_stats())
    seconds = elapsed_ns / 1e9
    print(
        f"numpy-matrix,{turns},{seconds:.9f},{elapsed_ns / turns:.3f},"
        f"{turns / seconds:.3f},{initial_energy:.12f},"
        f"{energy(positions, velocities):.12f},"
        f"{collections_after - collections_before},0"
    )


if __name__ == "__main__":
    main(int(sys.argv[1]) if len(sys.argv) > 1 else 1_000_000)
