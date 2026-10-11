"""N-body simulation of the outer solar system in modern Mojo.

Positions and velocities are held as 4-wide SIMD vectors (3 spatial lanes plus a
zero pad), so every per-body update is a single vector operation. Physical
constants are folded at compile time with `comptime`.
"""

from std.math import sqrt
from std.sys import argv
from common import print_fixed

comptime PI = 3.141592653589793
comptime SOLAR_MASS = 4 * PI * PI
comptime DAYS_PER_YEAR = 365.24
comptime DELTA_T = 0.01

# 3-D vector padded to 4 lanes; the 4th lane stays zero and drops out of sums.
comptime Vec = SIMD[DType.float64, 4]


@fieldwise_init
struct Planet(ImplicitlyCopyable, Movable):
    var pos: Vec
    var vel: Vec
    var mass: Float64




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
def energy(imm bodies: List[Planet]) -> Float64:
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
    var bodies: List[Planet] = [
        Planet(Vec(0, 0, 0, 0), Vec(0, 0, 0, 0), SOLAR_MASS),
        Planet(Vec(4.84143144246472090, -1.16032004402742839, -0.103622044471123109, 0), Vec(0.00166007664274403694, 0.00769901118419740425, -0.0000690460016972063023, 0) * DAYS_PER_YEAR, 0.000954791938424326609 * SOLAR_MASS),
        Planet(Vec(8.34336671824457987, 4.12479856412430479, -0.403523417114321381, 0), Vec(-0.00276742510726862411, 0.00499852801234917238, 0.0000230417297573763929, 0) * DAYS_PER_YEAR, 0.000285885980666130812 * SOLAR_MASS),
        Planet(Vec(12.8943695621391310, -15.1111514016986312, -0.223307578892655734, 0), Vec(0.00296460137564761618, 0.00237847173959480950, -0.0000296589568540237556, 0) * DAYS_PER_YEAR, 0.0000436624404335156298 * SOLAR_MASS),
        Planet(Vec(15.3796971148509165, -25.9193146099879641, 0.179258772950371181, 0), Vec(0.00268067772490389322, 0.00162824170038242295, -0.0000951592254519715870, 0) * DAYS_PER_YEAR, 0.0000515138902046611451 * SOLAR_MASS),
    ]

    var p = Vec(0)
    for ref body in bodies:
        p += body.vel * body.mass
    bodies[0].vel = -p / SOLAR_MASS
    print_fixed(energy(bodies), 9)
    for _ in range(Int(argv()[1])):
        advance(bodies, DELTA_T)
    print_fixed(energy(bodies), 9)
