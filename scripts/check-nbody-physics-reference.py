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
EXPECTED_TEN_BODY_STATE = "f9194e44d424d554c68dfcecbd8f0af95e764b65aff5387fd99d1a781c78d3c8"
JPL_ORBITS = (
    ("Mercury", 0.3870982252718477, 0.2056302515978038),
    ("Venus", 0.7233268496756070, 0.006755697268576816),
    ("Earth", 1.000371833994387, 0.01704239718110438),
    ("Mars", 1.523678184286835, 0.09331460654156362),
    ("Jupiter", 5.205108585205607, 0.04892305962953223),
    ("Saturn", 9.581451990528134, 0.05559928887285597),
    ("Uranus", 19.22993812529615, 0.04439367187710320),
    ("Neptune", 30.09700542229719, 0.01114790154011905),
    ("Pluto", 39.50058973957585, 0.2478572758892915),
)


def mech_bodies() -> list[list[object]]:
    year = DAYS_PER_YEAR
    mass = MECH_SOLAR_MASS
    return [
        [
            [0.0, 0.0, 0.0],
            [
                5.37374287703080437e-06 * year,
                -7.41169057513150263e-06 * year,
                -9.41516755244529502e-08 * year,
            ],
            mass,
        ],
        [
            [-0.1407280797108344, -0.4439009580270337, -0.02334555919971206],
            [
                0.0211742451018037403 * year,
                -0.00710538711113244733 * year,
                -0.00252292518239450756 * year,
            ],
            1.660120e-07 * mass,
        ],
        [
            [-0.7186302169204941, -0.02250380069428597, 0.04117184128636830],
            [
                0.000518906490022535321 * year,
                -0.0203135533130517909 * year,
                -0.000307268671743680998 * year,
            ],
            2.447838e-06 * mass,
        ],
        [
            [-0.1685246483858782, 0.9687833049070306, -0.000004120490278477264],
            [
                -0.0172285720895999014 * year,
                -0.00301507194043644251 * year,
                -0.0000000585259466404432129 * year,
            ],
            3.003489e-06 * mass,
        ],
        [
            [1.390361066039004, -0.02100972225898463, -0.03461801440927048],
            [
                0.000753300878828797971 * year,
                0.0151788869860826885 * year,
                0.000299659058967668502 * year,
            ],
            3.227151e-07 * mass,
        ],
        [
            [4.003460488693537, 2.935353187887882, -0.1018230443988181],
            [
                -0.00455837705250217509 * year,
                0.00643986253216750643 * year,
                0.0000753759450047445577 * year,
            ],
            0.000954791938424326609 * mass,
        ],
        [
            [6.408556035505925, 6.568042752621957, -0.3691272880681217],
            [
                -0.00428516675602203662 * year,
                0.00388457920257733250 * year,
                0.000102515602671251747 * year,
            ],
            0.000285885980666130812 * mass,
        ],
        [
            [14.43051609648136, -13.73565967460644, -0.2381293855338772],
            [
                0.00268383937076146876 * year,
                0.00266501521249669056 * year,
                -0.0000248452866168716821 * year,
            ],
            0.0000436624404335156298 * mass,
        ],
        [
            [16.81075807703606, -24.99265146883861, 0.1272705680239183],
            [
                0.00258459075834710852 * year,
                0.00176894334798696740 * year,
                -0.0000962942121658658866 * year,
            ],
            0.0000515138902046611451 * mass,
        ],
        [
            [-9.876866563865008, -27.95802013288459, 5.850814086362886],
            [
                0.00304437716859956966 * year,
                -0.00153730074570079052 * year,
                -0.000717326330158363440 * year,
            ],
            6.547e-09 * mass,
        ],
    ]


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


def relative_body_state(
    bodies: list[list[object]], body_index: int
) -> tuple[float, float, float]:
    position = [bodies[body_index][0][axis] - bodies[0][0][axis] for axis in range(3)]
    velocity = [bodies[body_index][1][axis] - bodies[0][1][axis] for axis in range(3)]
    radius = math.sqrt(sum(component * component for component in position))
    speed = math.sqrt(sum(component * component for component in velocity))
    cross = [
        position[1] * velocity[2] - position[2] * velocity[1],
        position[2] * velocity[0] - position[0] * velocity[2],
        position[0] * velocity[1] - position[1] * velocity[0],
    ]
    areal_rate = 0.5 * math.sqrt(sum(component * component for component in cross))
    return radius, speed, areal_rate


def body_orbital_elements(
    bodies: list[list[object]], body_index: int
) -> tuple[float, float]:
    position = [bodies[body_index][0][axis] - bodies[0][0][axis] for axis in range(3)]
    velocity = [bodies[body_index][1][axis] - bodies[0][1][axis] for axis in range(3)]
    radius = math.sqrt(sum(component * component for component in position))
    speed_squared = sum(component * component for component in velocity)
    cross = [
        position[1] * velocity[2] - position[2] * velocity[1],
        position[2] * velocity[0] - position[0] * velocity[2],
        position[0] * velocity[1] - position[1] * velocity[0],
    ]
    gravitational_parameter = bodies[0][2] + bodies[body_index][2]
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
    orbital_elements = []
    for body_index, (name, expected_axis, expected_eccentricity) in enumerate(
        JPL_ORBITS, start=1
    ):
        semimajor_axis, eccentricity = body_orbital_elements(bodies, body_index)
        axis_tolerance = max(0.002, expected_axis * 0.0004)
        if abs(semimajor_axis - expected_axis) >= axis_tolerance:
            raise SystemExit(
                f"{name} semimajor axis does not match its JPL J2000 element: "
                f"expected {expected_axis:.9f}, got {semimajor_axis:.9f} AU"
            )
        if abs(eccentricity - expected_eccentricity) >= 0.001:
            raise SystemExit(
                f"{name} eccentricity does not match its JPL J2000 element: "
                f"expected {expected_eccentricity:.9f}, got {eccentricity:.9f}"
            )
        orbital_elements.append((semimajor_axis, eccentricity))
    mercury_semimajor_axis, mercury_eccentricity = orbital_elements[0]
    neptune_axis, _ = orbital_elements[7]
    pluto_axis, pluto_eccentricity = orbital_elements[8]
    pluto_perihelion = pluto_axis * (1.0 - pluto_eccentricity)
    pluto_aphelion = pluto_axis * (1.0 + pluto_eccentricity)
    if not pluto_perihelion < neptune_axis < pluto_aphelion:
        raise SystemExit(
            "Pluto's eccentric orbit must span Neptune's semimajor axis: "
            f"q={pluto_perihelion:.9f}, Neptune a={neptune_axis:.9f}, Q={pluto_aphelion:.9f}"
        )
    initial_energy = energy(bodies)
    mercury = [relative_body_state(bodies, 1)]
    for _ in range(MECH_TURNS):
        advance(bodies, MECH_DT)
        mercury.append(relative_body_state(bodies, 1))
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

    orbit_summary = ",".join(
        f"{name}:{semimajor_axis:.9f}/{eccentricity:.9f}"
        for (name, _, _), (semimajor_axis, eccentricity) in zip(
            JPL_ORBITS, orbital_elements
        )
    )
    print(
        "NBODY_PYTHON_REFERENCE "
        f"turns={MECH_TURNS} dt={MECH_DT} state={final_hash} "
        f"jpl_orbits={orbit_summary} "
        f"initial_energy={initial_energy:.12f} final_energy={final_energy:.12f} "
        f"mercury_semimajor_axis={mercury_semimajor_axis:.9f} "
        f"mercury_eccentricity={mercury_eccentricity:.9f} "
        f"pluto_semimajor_axis={pluto_axis:.9f} "
        f"pluto_eccentricity={pluto_eccentricity:.9f} "
        f"pluto_perihelion={pluto_perihelion:.9f} "
        f"pluto_aphelion={pluto_aphelion:.9f} "
        f"mercury_perihelion_radius={perihelion[0]:.9f} "
        f"mercury_perihelion_speed={perihelion[1]:.9f} "
        f"mercury_aphelion_radius={aphelion[0]:.9f} "
        f"mercury_aphelion_speed={aphelion[1]:.9f} "
        f"mercury_areal_rate_spread={areal_rate_spread:.6e} "
        f"benchmark_initial={benchmark_initial:.9f} benchmark_final={benchmark_final:.9f}"
    )


if __name__ == "__main__":
    main()
