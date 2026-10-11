"""N-body simulation — Mojo implementation.

Mojo is a compiled language with Python-like syntax. This is a direct
translation of the official Benchmarks Game n-body algorithm.

Attribution:
  Based on the Benchmarks Game Python #1 implementation
  (https://benchmarksgame-team.pages.debian.net/benchmarksgame/)
"""

from std.math import sqrt


# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

comptime NUM_BODIES = 5
comptime DEFAULT_N = 500_000
comptime PI = 3.14159265358979323
comptime SOLAR_MASS = 4.0 * PI * PI
comptime DAYS_PER_YEAR = 365.24
comptime DT = 0.01

# ---------------------------------------------------------------------------
# N-body system (struct-based to avoid global vars)
# ---------------------------------------------------------------------------


struct NBodySystem:
    var pos: InlineArray[SIMD[DType.float64, 4], NUM_BODIES]
    """Each SIMD: [x, y, z, 0]."""

    var vel: InlineArray[SIMD[DType.float64, 4], NUM_BODIES]
    """Each SIMD: [vx, vy, vz, 0.0]."""

    var mass: InlineArray[SIMD[DType.float64, 1], NUM_BODIES]

    def __init__(out self):
        self.pos = InlineArray[SIMD[DType.float64, 4], NUM_BODIES](fill=0.0)
        self.vel = InlineArray[SIMD[DType.float64, 4], NUM_BODIES](fill=0.0)
        self.mass = InlineArray[SIMD[DType.float64, 1], NUM_BODIES](fill=0.0)

        # Sun
        self.mass[0] = SOLAR_MASS

        # Jupiter
        self.pos[1][0] = 4.84143144246472090e00
        self.pos[1][1] = -1.16032004402742839e00
        self.pos[1][2] = -1.03622044471123109e-01
        self.vel[1][0] = 1.66007664274403694e-03 * DAYS_PER_YEAR
        self.vel[1][1] = 7.69901118419740425e-03 * DAYS_PER_YEAR
        self.vel[1][2] = -6.90460016972063023e-05 * DAYS_PER_YEAR
        self.mass[1] = 9.54791938424326609e-04 * SOLAR_MASS

        # Saturn
        self.pos[2][0] = 8.34336671824457987e00
        self.pos[2][1] = 4.12479856412430479e00
        self.pos[2][2] = -4.03523417114321381e-01
        self.vel[2][0] = -2.76742510726862411e-03 * DAYS_PER_YEAR
        self.vel[2][1] = 4.99852801234917238e-03 * DAYS_PER_YEAR
        self.vel[2][2] = 2.30417297573763929e-05 * DAYS_PER_YEAR
        self.mass[2] = 2.85885980666130812e-04 * SOLAR_MASS

        # Uranus
        self.pos[3][0] = 1.28943695621391310e01
        self.pos[3][1] = -1.51111514016986312e01
        self.pos[3][2] = -2.23307578892655734e-01
        self.vel[3][0] = 2.96460137564761618e-03 * DAYS_PER_YEAR
        self.vel[3][1] = 2.37847173959480950e-03 * DAYS_PER_YEAR
        self.vel[3][2] = -2.96589568540237556e-05 * DAYS_PER_YEAR
        self.mass[3] = 4.36624404335156298e-05 * SOLAR_MASS

        # Neptune
        self.pos[4][0] = 1.53796971148509165e01
        self.pos[4][1] = -2.59193146099879641e01
        self.pos[4][2] = 1.79258772950371181e-01
        self.vel[4][0] = 2.68067772490389322e-03 * DAYS_PER_YEAR
        self.vel[4][1] = 1.62824170038242295e-03 * DAYS_PER_YEAR
        self.vel[4][2] = -9.51592254519715870e-05 * DAYS_PER_YEAR
        self.mass[4] = 5.15138902046611451e-05 * SOLAR_MASS

        # Offset momentum
        p = SIMD[DType.float64, 4](0.0)
        for i in range(NUM_BODIES):
            p -= self.vel[i] * self.mass[i]
        self.vel[0] = p / SOLAR_MASS

    def advance(mut self, n: Int):
        for _ in range(n):
            comptime for i in range(NUM_BODIES):
                comptime for j in range(i + 1, NUM_BODIES):
                    dp = self.pos[i] - self.pos[j]
                    dsq = (dp * dp).reduce_add()
                    mag = DT / (dsq * sqrt(dsq))
                    mi = self.mass[i] * mag
                    mj = self.mass[j] * mag
                    self.vel[i] -= dp * mj
                    self.vel[j] += dp * mi
            comptime for i in range(NUM_BODIES):
                self.pos[i] += DT * self.vel[i]

    def energy(self) -> Float64:
        e = 0.0
        comptime for i in range(NUM_BODIES):
            v = self.vel[i]
            e += self.mass[i] * (v * v).reduce_add() / 2.0
            comptime for j in range(i + 1, NUM_BODIES):
                dp = self.pos[i] - self.pos[j]
                dsq = (dp * dp).reduce_add()
                e -= (self.mass[i] * self.mass[j]) / sqrt(dsq)
        return e


def run_nbody(n: Int = DEFAULT_N) -> Dict[String, Float64]:
    """Returns [energy_before, energy_after]."""
    var sys = NBodySystem()
    var e_before = sys.energy()
    sys.advance(n)
    var e_after = sys.energy()
    return {"energy_before": e_before, "energy_after": e_after}
