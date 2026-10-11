"""N-body simulation of the outer solar system in modern Mojo.

Positions and velocities are held as 4-wide SIMD vectors (3 spatial lanes plus a
zero pad), so every per-body update is a single vector operation. Physical
constants are folded at compile time with `comptime`.
"""

from std.math import sqrt
from std.sys import argv
from std.time import perf_counter_ns

comptime PI = 3.141592653589793
comptime SOLAR_MASS = 4 * PI * PI
comptime DAYS_PER_YEAR = 365.24
comptime DELTA_T = 0.01
comptime ENERGY_DIFF_THRESHOLD = 1e-4

# 3-D vector padded to 4 lanes; the 4th lane stays zero and drops out of sums.
comptime Vec = SIMD[DType.float64, 4]


@fieldwise_init
struct Planet(ImplicitlyCopyable, Movable):
    var pos: Vec
    var vel: Vec
    var mass: Float64


def create_initial_system() -> List[Planet]:
    """Builds the canonical five-body system (Sun + four gas giants)."""
    return [
        Planet(Vec(0, 0, 0, 0), Vec(0, 0, 0, 0), SOLAR_MASS),
        Planet(
            Vec(
                4.84143144246472090e00,
                -1.16032004402742839e00,
                -1.03622044471123109e-01,
                0,
            ),
            Vec(
                1.66007664274403694e-03 * DAYS_PER_YEAR,
                7.69901118419740425e-03 * DAYS_PER_YEAR,
                -6.90460016972063023e-05 * DAYS_PER_YEAR,
                0,
            ),
            9.54791938424326609e-04 * SOLAR_MASS,
        ),
        Planet(
            Vec(
                8.34336671824457987e00,
                4.12479856412430479e00,
                -4.03523417114321381e-01,
                0,
            ),
            Vec(
                -2.76742510726862411e-03 * DAYS_PER_YEAR,
                4.99852801234917238e-03 * DAYS_PER_YEAR,
                2.30417297573763929e-05 * DAYS_PER_YEAR,
                0,
            ),
            2.85885980666130812e-04 * SOLAR_MASS,
        ),
        Planet(
            Vec(
                1.28943695621391310e01,
                -1.51111514016986312e01,
                -2.23307578892655734e-01,
                0,
            ),
            Vec(
                2.96460137564761618e-03 * DAYS_PER_YEAR,
                2.37847173959480950e-03 * DAYS_PER_YEAR,
                -2.96589568540237556e-05 * DAYS_PER_YEAR,
                0,
            ),
            4.36624404335156298e-05 * SOLAR_MASS,
        ),
        Planet(
            Vec(
                1.53796971148509165e01,
                -2.59193146099879641e01,
                1.79258772950371181e-01,
                0,
            ),
            Vec(
                2.68067772490389322e-03 * DAYS_PER_YEAR,
                1.62824170038242295e-03 * DAYS_PER_YEAR,
                -9.51592254519715870e-05 * DAYS_PER_YEAR,
                0,
            ),
            5.15138902046611451e-05 * SOLAR_MASS,
        ),
    ]


@always_inline
def offset_momentum(mut bodies: List[Planet]):
    """Cancels total momentum via the first body (the Sun)."""
    var p = Vec(0)
    for ref body in bodies:
        p += body.vel * body.mass
    bodies[0].vel = -p / SOLAR_MASS


@always_inline
def advance(mut bodies: List[Planet], dt: Float64):
    """Advances every body by one timestep of length `dt`."""
    var n = len(bodies)
    for i in range(n):
        for j in range(i + 1, n):
            var bi = bodies[i]
            var bj = bodies[j]
            var d = bi.pos - bj.pos
            var d2 = (d * d).reduce_add()
            var mag = dt / (d2 * sqrt(d2))
            bodies[i].vel = bi.vel - d * (bj.mass * mag)
            bodies[j].vel = bj.vel + d * (bi.mass * mag)

    for ref body in bodies:
        body.pos += body.vel * dt


@always_inline
def energy(read bodies: List[Planet]) -> Float64:
    """Total energy (kinetic + gravitational potential) of the system."""
    var e = 0.0
    var n = len(bodies)
    for i in range(n):
        var bi = bodies[i]
        e += 0.5 * bi.mass * (bi.vel * bi.vel).reduce_add()
        for j in range(i + 1, n):
            var bj = bodies[j]
            var d = bi.pos - bj.pos
            e -= bi.mass * bj.mass / sqrt((d * d).reduce_add())
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

    # Machine-readable lines consumed by the benchmark harness. Format the
    # elapsed time to a fixed 3 decimals (matching the Rust runner) so the
    # harness never has to parse Float64 scientific notation.
    var total_us = Int(elapsed_ms * 1000.0 + 0.5)
    var frac = total_us % 1000
    var frac_str = String(frac)
    if frac < 10:
        frac_str = "00" + frac_str
    elif frac < 100:
        frac_str = "0" + frac_str
    print(String("ELAPSED_MS ") + String(total_us // 1000) + "." + frac_str)
    print("CONSERVED", "true" if conserved else "false")
