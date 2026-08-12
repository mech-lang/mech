"""Benchmarks Game Python n-body with an in-process steady-state timer."""

# Source: https://benchmarksgame-team.pages.debian.net/benchmarksgame/program/nbody-python3-1.html
# Originally by Kevin Carson; modified by Tupteq, Fredrik Johansson,
# Daniel Nanz, and Maciej Fijalkowski. Timing/CSV telemetry added here.

import gc
import sys
import time


def combinations(values):
    result = []
    for x in range(len(values) - 1):
        for y in values[x + 1 :]:
            result.append((values[x], y))
    return result


PI = 3.14159265358979323
SOLAR_MASS = 4 * PI * PI
DAYS_PER_YEAR = 365.24

BODIES = {
    "sun": ([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], SOLAR_MASS),
    "jupiter": (
        [4.84143144246472090e00, -1.16032004402742839e00, -1.03622044471123109e-01],
        [
            1.66007664274403694e-03 * DAYS_PER_YEAR,
            7.69901118419740425e-03 * DAYS_PER_YEAR,
            -6.90460016972063023e-05 * DAYS_PER_YEAR,
        ],
        9.54791938424326609e-04 * SOLAR_MASS,
    ),
    "saturn": (
        [8.34336671824457987e00, 4.12479856412430479e00, -4.03523417114321381e-01],
        [
            -2.76742510726862411e-03 * DAYS_PER_YEAR,
            4.99852801234917238e-03 * DAYS_PER_YEAR,
            2.30417297573763929e-05 * DAYS_PER_YEAR,
        ],
        2.85885980666130812e-04 * SOLAR_MASS,
    ),
    "uranus": (
        [1.28943695621391310e01, -1.51111514016986312e01, -2.23307578892655734e-01],
        [
            2.96460137564761618e-03 * DAYS_PER_YEAR,
            2.37847173959480950e-03 * DAYS_PER_YEAR,
            -2.96589568540237556e-05 * DAYS_PER_YEAR,
        ],
        4.36624404335156298e-05 * SOLAR_MASS,
    ),
    "neptune": (
        [1.53796971148509165e01, -2.59193146099879641e01, 1.79258772950371181e-01],
        [
            2.68067772490389322e-03 * DAYS_PER_YEAR,
            1.62824170038242295e-03 * DAYS_PER_YEAR,
            -9.51592254519715870e-05 * DAYS_PER_YEAR,
        ],
        5.15138902046611451e-05 * SOLAR_MASS,
    ),
}

SYSTEM = list(BODIES.values())
PAIRS = combinations(SYSTEM)


def advance(dt, n, bodies=SYSTEM, pairs=PAIRS):
    for _ in range(n):
        for (([x1, y1, z1], v1, m1), ([x2, y2, z2], v2, m2)) in pairs:
            dx = x1 - x2
            dy = y1 - y2
            dz = z1 - z2
            mag = dt * ((dx * dx + dy * dy + dz * dz) ** (-1.5))
            b1m = m1 * mag
            b2m = m2 * mag
            v1[0] -= dx * b2m
            v1[1] -= dy * b2m
            v1[2] -= dz * b2m
            v2[0] += dx * b1m
            v2[1] += dy * b1m
            v2[2] += dz * b1m
        for position, [vx, vy, vz], _ in bodies:
            position[0] += dt * vx
            position[1] += dt * vy
            position[2] += dt * vz


def energy(bodies=SYSTEM, pairs=PAIRS):
    total = 0.0
    for (((x1, y1, z1), _, m1), ((x2, y2, z2), _, m2)) in pairs:
        dx = x1 - x2
        dy = y1 - y2
        dz = z1 - z2
        total -= (m1 * m2) / ((dx * dx + dy * dy + dz * dz) ** 0.5)
    for _, [vx, vy, vz], mass in bodies:
        total += mass * (vx * vx + vy * vy + vz * vz) / 2.0
    return total


def offset_momentum(ref, bodies=SYSTEM):
    px = py = pz = 0.0
    for _, [vx, vy, vz], mass in bodies:
        px -= vx * mass
        py -= vy * mass
        pz -= vz * mass
    _, velocity, mass = ref
    velocity[0] = px / mass
    velocity[1] = py / mass
    velocity[2] = pz / mass


def main(turns):
    offset_momentum(BODIES["sun"])
    initial_energy = energy()
    gc.collect()
    collections_before = sum(generation["collections"] for generation in gc.get_stats())
    started = time.perf_counter_ns()
    advance(0.01, turns)
    elapsed_ns = time.perf_counter_ns() - started
    collections_after = sum(generation["collections"] for generation in gc.get_stats())
    seconds = elapsed_ns / 1e9
    print(
        f"python-game,{turns},{seconds:.9f},{elapsed_ns / turns:.3f},"
        f"{turns / seconds:.3f},{initial_energy:.12f},{energy():.12f},"
        f"{collections_after - collections_before},0"
    )


if __name__ == "__main__":
    main(int(sys.argv[1]) if len(sys.argv) > 1 else 1_000_000)
