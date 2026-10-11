"""Naive N-body simulation in Mojo — the deliberately un-optimised counterpart.

Each body keeps its position and velocity as scalar `List[Float64]`, and the
pairwise loop is written the obvious component-by-component way (no SIMD, no
inlining hints). This mirrors the 2024 "naive Mojo" entry ported to Mojo 1.0, to
show how the naive path compares against the SIMD version in `n_body.mojo`.
"""

from std.math import sqrt
from std.sys import argv
from std.time import perf_counter_ns

comptime PI = 3.141592653589793
comptime SOLAR_MASS = 4 * PI * PI
comptime DAYS_PER_YEAR = 365.24
comptime DELTA_T = 0.01
comptime ENERGY_DIFF_THRESHOLD = 1e-4


@fieldwise_init
struct Planet(Copyable, Movable):
    var pos: List[Float64]
    var vel: List[Float64]
    var mass: Float64


def create_initial_system() -> List[Planet]:
    """Builds the canonical five-body system (Sun + four gas giants)."""
    return [
        Planet([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], SOLAR_MASS),
        Planet(
            [
                4.84143144246472090e00,
                -1.16032004402742839e00,
                -1.03622044471123109e-01,
            ],
            [
                1.66007664274403694e-03 * DAYS_PER_YEAR,
                7.69901118419740425e-03 * DAYS_PER_YEAR,
                -6.90460016972063023e-05 * DAYS_PER_YEAR,
            ],
            9.54791938424326609e-04 * SOLAR_MASS,
        ),
        Planet(
            [
                8.34336671824457987e00,
                4.12479856412430479e00,
                -4.03523417114321381e-01,
            ],
            [
                -2.76742510726862411e-03 * DAYS_PER_YEAR,
                4.99852801234917238e-03 * DAYS_PER_YEAR,
                2.30417297573763929e-05 * DAYS_PER_YEAR,
            ],
            2.85885980666130812e-04 * SOLAR_MASS,
        ),
        Planet(
            [
                1.28943695621391310e01,
                -1.51111514016986312e01,
                -2.23307578892655734e-01,
            ],
            [
                2.96460137564761618e-03 * DAYS_PER_YEAR,
                2.37847173959480950e-03 * DAYS_PER_YEAR,
                -2.96589568540237556e-05 * DAYS_PER_YEAR,
            ],
            4.36624404335156298e-05 * SOLAR_MASS,
        ),
        Planet(
            [
                1.53796971148509165e01,
                -2.59193146099879641e01,
                1.79258772950371181e-01,
            ],
            [
                2.68067772490389322e-03 * DAYS_PER_YEAR,
                1.62824170038242295e-03 * DAYS_PER_YEAR,
                -9.51592254519715870e-05 * DAYS_PER_YEAR,
            ],
            5.15138902046611451e-05 * SOLAR_MASS,
        ),
    ]


def offset_momentum(mut bodies: List[Planet]):
    """Cancels total momentum via the first body (the Sun)."""
    var px = 0.0
    var py = 0.0
    var pz = 0.0
    for ref body in bodies:
        px += body.vel[0] * body.mass
        py += body.vel[1] * body.mass
        pz += body.vel[2] * body.mass
    bodies[0].vel[0] = -px / SOLAR_MASS
    bodies[0].vel[1] = -py / SOLAR_MASS
    bodies[0].vel[2] = -pz / SOLAR_MASS


def advance(mut bodies: List[Planet], dt: Float64):
    """Advances every body by one timestep of length `dt`."""
    var n = len(bodies)
    for i in range(n):
        for j in range(i + 1, n):
            var dx = bodies[i].pos[0] - bodies[j].pos[0]
            var dy = bodies[i].pos[1] - bodies[j].pos[1]
            var dz = bodies[i].pos[2] - bodies[j].pos[2]
            var d2 = dx * dx + dy * dy + dz * dz
            var mag = dt / (d2 * sqrt(d2))
            var mi = bodies[i].mass
            var mj = bodies[j].mass
            bodies[i].vel[0] -= dx * mj * mag
            bodies[i].vel[1] -= dy * mj * mag
            bodies[i].vel[2] -= dz * mj * mag
            bodies[j].vel[0] += dx * mi * mag
            bodies[j].vel[1] += dy * mi * mag
            bodies[j].vel[2] += dz * mi * mag

    for ref body in bodies:
        body.pos[0] += dt * body.vel[0]
        body.pos[1] += dt * body.vel[1]
        body.pos[2] += dt * body.vel[2]


def energy(read bodies: List[Planet]) -> Float64:
    """Total energy (kinetic + gravitational potential) of the system."""
    var e = 0.0
    var n = len(bodies)
    for i in range(n):
        var vx = bodies[i].vel[0]
        var vy = bodies[i].vel[1]
        var vz = bodies[i].vel[2]
        e += 0.5 * bodies[i].mass * (vx * vx + vy * vy + vz * vz)
        for j in range(i + 1, n):
            var dx = bodies[i].pos[0] - bodies[j].pos[0]
            var dy = bodies[i].pos[1] - bodies[j].pos[1]
            var dz = bodies[i].pos[2] - bodies[j].pos[2]
            e -= (
                bodies[i].mass
                * bodies[j].mass
                / sqrt(dx * dx + dy * dy + dz * dz)
            )
    return e


def main() raises:
    var steps = 1000
    var args = argv()
    if len(args) > 1:
        steps = Int(args[1])

    var bodies = create_initial_system()
    offset_momentum(bodies)
    var initial_energy = energy(bodies)

    var start = perf_counter_ns()
    for _ in range(steps):
        advance(bodies, DELTA_T)
    var elapsed_ms = Float64(perf_counter_ns() - start) / 1.0e6

    var conserved = abs(energy(bodies) - initial_energy) < ENERGY_DIFF_THRESHOLD
    if not conserved:
        print("warning: energy was not conserved")

    # Fixed 3-decimal ELAPSED_MS (matching the other runners) so the harness
    # never has to parse Float64 scientific notation.
    var total_us = Int(elapsed_ms * 1000.0 + 0.5)
    var frac = total_us % 1000
    var frac_str = String(frac)
    if frac < 10:
        frac_str = "00" + frac_str
    elif frac < 100:
        frac_str = "0" + frac_str
    print(String("ELAPSED_MS ") + String(total_us // 1000) + "." + frac_str)
    print("CONSERVED", "true" if conserved else "false")
