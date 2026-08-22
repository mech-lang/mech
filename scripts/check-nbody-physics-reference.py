#!/usr/bin/env python3
"""Independent scalar reference checks for the public ten-body Mech example."""

from __future__ import annotations

import hashlib
import math
import struct


DAYS_PER_YEAR = 365.24
MECH_PI = 3.141592654
MECH_SOLAR_MASS = 4.0 * MECH_PI**2
MECH_DT = 0.002
MECH_TURNS = 4_096
STATE_QUANTUM = 1.0e-8
EXPECTED_TEN_BODY_STATE = "0c2206f579e31f4c8fa4efa1cd32cab53386224fda0f4593274843e12440f8f8"
JPL_MERCURY_SEMIMAJOR_AXIS_AU = 0.38709927
JPL_MERCURY_ECCENTRICITY = 0.20563593


def mech_bodies() -> list[list[object]]:
    year = DAYS_PER_YEAR
    mass = MECH_SOLAR_MASS
    bodies = [
        [[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], mass],
        [
            [-0.3895481522339008, -0.1503099879924185, 0.02350666145222942],
            [
                0.004308277706530398 * year,
                -0.02502425088636981 * year,
                -0.002439088939653241 * year,
            ],
            1.660120e-07 * mass,
        ],
        [
            [-0.71803521454713496, 0.041947688018485076, 0.041304042456734767],
            [
                -0.0013897739822126357 * year,
                -0.020119890903497486 * year,
                -0.00030021717325320866 * year,
            ],
            2.447838e-06 * mass,
        ],
        [
            [-0.17713546150023925, 0.96724162210078579, -0.0000039007362829064718],
            [
                -0.017201146327519165 * year,
                -0.0031864352617521142 * year,
                0.00000018827814191274 * year,
            ],
            3.003489e-06 * mass,
        ],
        [
            [1.3907159267594730, -0.013415706106135855, -0.034467796700612273],
            [
                0.00021345415375975118 * year,
                0.015123072614264662 * year,
                0.00030512951698556953 * year,
            ],
            3.227151e-07 * mass,
        ],
        [
            [4.84143144246472090, -1.16032004402742839, -0.103622044471123109],
            [
                0.00166007664274403694 * year,
                0.00769901118419740425 * year,
                -0.0000690460016972063023 * year,
            ],
            0.000954791938424326609 * mass,
        ],
        [
            [8.34336671824457987, 4.12479856412430479, -0.403523417114321381],
            [
                -0.00276742510726862411 * year,
                0.00499852801234917238 * year,
                0.0000230417297573763929 * year,
            ],
            0.000285885980666130812 * mass,
        ],
        [
            [12.8943695621391310, -15.1111514016986312, -0.223307578892655734],
            [
                0.00296460137564761618 * year,
                0.00237847173959480950 * year,
                -0.0000296589568540237556 * year,
            ],
            0.0000436624404335156298 * mass,
        ],
        [
            [15.3796971148509165, -25.9193146099879641, 0.179258772950371181],
            [
                0.00268067772490389322 * year,
                0.00162824170038242295 * year,
                -0.000095159225451971587 * year,
            ],
            0.0000515138902046611451 * mass,
        ],
        [
            [-9.87512510193949936, -27.9392880241831424, 5.06873275440839440],
            [
                0.00344178030872987300 * year,
                -0.00152819214839188910 * year,
                -0.00129137458409475460 * year,
            ],
            6.547e-09 * mass,
        ],
    ]
    momentum = [sum(body[1][axis] * body[2] for body in bodies) for axis in range(3)]
    bodies[0][1] = [-component / mass for component in momentum]
    return bodies


def benchmark_bodies() -> list[list[object]]:
    year = DAYS_PER_YEAR
    mass = 4.0 * math.pi**2
    bodies = [
        [[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], mass],
        [
            [4.84143144246472090, -1.16032004402742839, -0.103622044471123109],
            [0.00166007664274403694 * year, 0.00769901118419740425 * year, -0.0000690460016972063023 * year],
            0.000954791938424326609 * mass,
        ],
        [
            [8.34336671824457987, 4.12479856412430479, -0.403523417114321381],
            [-0.00276742510726862411 * year, 0.00499852801234917238 * year, 0.0000230417297573763929 * year],
            0.000285885980666130812 * mass,
        ],
        [
            [12.8943695621391310, -15.1111514016986312, -0.223307578892655734],
            [0.00296460137564761618 * year, 0.00237847173959480950 * year, -0.0000296589568540237556 * year],
            0.0000436624404335156298 * mass,
        ],
        [
            [15.3796971148509165, -25.9193146099879641, 0.179258772950371181],
            [0.00268067772490389322 * year, 0.00162824170038242295 * year, -0.000095159225451971587 * year],
            0.0000515138902046611451 * mass,
        ],
    ]
    momentum = [sum(body[1][axis] * body[2] for body in bodies) for axis in range(3)]
    bodies[0][1] = [-component / mass for component in momentum]
    return bodies


def advance(bodies: list[list[object]], dt: float) -> None:
    pairs: list[tuple[int, int, list[float], float]] = []
    for left in range(len(bodies)):
        for right in range(left + 1, len(bodies)):
            delta = [bodies[left][0][axis] - bodies[right][0][axis] for axis in range(3)]
            distance_squared = sum(component * component for component in delta)
            pairs.append((left, right, delta, dt * distance_squared**-1.5))
    for left, right, delta, magnitude in pairs:
        for axis in range(3):
            bodies[left][1][axis] -= delta[axis] * bodies[right][2] * magnitude
    for left, right, delta, magnitude in pairs:
        for axis in range(3):
            bodies[right][1][axis] += delta[axis] * bodies[left][2] * magnitude
    for position, velocity, _ in bodies:
        for axis in range(3):
            position[axis] += velocity[axis] * dt


def energy(bodies: list[list[object]]) -> float:
    kinetic = sum(
        0.5 * body[2] * sum(component * component for component in body[1])
        for body in bodies
    )
    potential = 0.0
    for left in range(len(bodies)):
        for right in range(left + 1, len(bodies)):
            distance = math.sqrt(
                sum((bodies[left][0][axis] - bodies[right][0][axis]) ** 2 for axis in range(3))
            )
            potential += bodies[left][2] * bodies[right][2] / distance
    return kinetic - potential


def momentum(bodies: list[list[object]]) -> list[float]:
    return [
        sum(body[1][axis] * body[2] for body in bodies)
        for axis in range(3)
    ]


def rust_round(value: float) -> int:
    return math.floor(value + 0.5) if value >= 0.0 else math.ceil(value - 0.5)


def state_hash(bodies: list[list[object]]) -> str:
    digest = hashlib.sha256()
    for field in (0, 1):
        for axis in range(3):
            for body in bodies:
                quantized = rust_round(body[field][axis] / STATE_QUANTUM)
                digest.update(struct.pack("<q", quantized))
    return digest.hexdigest()


def relative_mercury_state(bodies: list[list[object]]) -> tuple[float, float, float]:
    position = [bodies[1][0][axis] - bodies[0][0][axis] for axis in range(3)]
    velocity = [bodies[1][1][axis] - bodies[0][1][axis] for axis in range(3)]
    radius = math.sqrt(sum(component * component for component in position))
    speed = math.sqrt(sum(component * component for component in velocity))
    cross = [
        position[1] * velocity[2] - position[2] * velocity[1],
        position[2] * velocity[0] - position[0] * velocity[2],
        position[0] * velocity[1] - position[1] * velocity[0],
    ]
    areal_rate = 0.5 * math.sqrt(sum(component * component for component in cross))
    return radius, speed, areal_rate


def mercury_orbital_elements(bodies: list[list[object]]) -> tuple[float, float]:
    position = [bodies[1][0][axis] - bodies[0][0][axis] for axis in range(3)]
    velocity = [bodies[1][1][axis] - bodies[0][1][axis] for axis in range(3)]
    radius = math.sqrt(sum(component * component for component in position))
    speed_squared = sum(component * component for component in velocity)
    cross = [
        position[1] * velocity[2] - position[2] * velocity[1],
        position[2] * velocity[0] - position[0] * velocity[2],
        position[0] * velocity[1] - position[1] * velocity[0],
    ]
    gravitational_parameter = bodies[0][2] + bodies[1][2]
    semimajor_axis = 1.0 / (2.0 / radius - speed_squared / gravitational_parameter)
    eccentricity = math.sqrt(
        1.0
        - sum(component * component for component in cross)
        / (gravitational_parameter * semimajor_axis)
    )
    return semimajor_axis, eccentricity


def main() -> None:
    bodies = mech_bodies()
    initial_momentum = momentum(bodies)
    if any(abs(component) >= 1.0e-12 for component in initial_momentum):
        raise SystemExit(f"ten-body initial momentum is not balanced: {initial_momentum}")
    mercury_semimajor_axis, mercury_eccentricity = mercury_orbital_elements(bodies)
    if abs(mercury_semimajor_axis - JPL_MERCURY_SEMIMAJOR_AXIS_AU) >= 0.002:
        raise SystemExit(
            "Mercury semimajor axis does not match the JPL orbit: "
            f"{mercury_semimajor_axis:.9f} AU"
        )
    if abs(mercury_eccentricity - JPL_MERCURY_ECCENTRICITY) >= 0.002:
        raise SystemExit(
            "Mercury eccentricity does not match the JPL orbit: "
            f"{mercury_eccentricity:.9f}"
        )
    initial_energy = energy(bodies)
    mercury = [relative_mercury_state(bodies)]
    for _ in range(MECH_TURNS):
        advance(bodies, MECH_DT)
        mercury.append(relative_mercury_state(bodies))
    final_energy = energy(bodies)
    final_hash = state_hash(bodies)
    if final_hash != EXPECTED_TEN_BODY_STATE:
        raise SystemExit(
            f"ten-body state mismatch: expected {EXPECTED_TEN_BODY_STATE}, got {final_hash}"
        )

    perihelion = min(mercury, key=lambda sample: sample[0])
    aphelion = max(mercury, key=lambda sample: sample[0])
    if not 0.295 <= perihelion[0] <= 0.320:
        raise SystemExit(f"Mercury perihelion is outside its nominal orbit: {perihelion}")
    if not 0.450 <= aphelion[0] <= 0.480:
        raise SystemExit(f"Mercury aphelion is outside its nominal orbit: {aphelion}")
    if not perihelion[1] > aphelion[1] * 1.2:
        raise SystemExit(
            f"Mercury did not accelerate through perihelion: perihelion={perihelion}, aphelion={aphelion}"
        )
    areal_rates = [sample[2] for sample in mercury]
    areal_rate_spread = (max(areal_rates) - min(areal_rates)) / sum(areal_rates) * len(areal_rates)
    if areal_rate_spread >= 0.01:
        raise SystemExit(f"Mercury areal-rate spread is too large: {areal_rate_spread:.6e}")

    benchmark = benchmark_bodies()
    benchmark_initial = energy(benchmark)
    for _ in range(1_000):
        advance(benchmark, 0.01)
    benchmark_final = energy(benchmark)
    if abs(benchmark_initial - (-0.169075164)) >= 5.0e-10:
        raise SystemExit(f"benchmark initial energy mismatch: {benchmark_initial:.12f}")
    if abs(benchmark_final - (-0.169087605)) >= 5.0e-10:
        raise SystemExit(f"benchmark final energy mismatch: {benchmark_final:.12f}")

    print(
        "NBODY_PYTHON_REFERENCE "
        f"turns={MECH_TURNS} dt={MECH_DT} state={final_hash} "
        f"initial_energy={initial_energy:.12f} final_energy={final_energy:.12f} "
        f"mercury_semimajor_axis={mercury_semimajor_axis:.9f} "
        f"mercury_eccentricity={mercury_eccentricity:.9f} "
        f"mercury_perihelion_radius={perihelion[0]:.9f} "
        f"mercury_perihelion_speed={perihelion[1]:.9f} "
        f"mercury_aphelion_radius={aphelion[0]:.9f} "
        f"mercury_aphelion_speed={aphelion[1]:.9f} "
        f"mercury_areal_rate_spread={areal_rate_spread:.6e} "
        f"benchmark_initial={benchmark_initial:.9f} benchmark_final={benchmark_final:.9f}"
    )


if __name__ == "__main__":
    main()
