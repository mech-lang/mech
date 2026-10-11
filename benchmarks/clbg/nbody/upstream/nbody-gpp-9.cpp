/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   contributed by Martin Jambrek
   based off the Java #2 program contributed by Mark C. Lewis and modified slightly by Chad Whipkey
*/

#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <utility>

static constexpr double PI = 3.141592653589793;
static constexpr double SOLAR_MASS = 4 * PI * PI;
static constexpr double DAYS_PER_YEAR = 365.24;

template <auto start, auto stop, auto step = 1, class F>
constexpr void static_for(F&& f)
{
    if constexpr (start < stop) {
        f(std::integral_constant<decltype(start), start>());
        static_for<start + step, stop, step>(std::move(f));
    }
}

struct alignas(32) Body {
    double x, y, z, pad0;
    double vx, vy, vz, pad1;
    double mass;

    constexpr Body(double x, double y, double z,
        double vx, double vy, double vz,
        double mass)
        : x(x)
        , y(y)
        , z(z)
        , pad0()
        , vx(vx)
        , vy(vy)
        , vz(vz)
        , pad1()
        , mass(mass)
    {
    }
};

static constexpr size_t N_BODIES = 5;
using System = Body[N_BODIES];

static constexpr Body sun = {
    0, 0, 0, 0, 0, 0, SOLAR_MASS
};

static constexpr Body jupiter = {
    4.84143144246472090e+00,
    -1.16032004402742839e+00,
    -1.03622044471123109e-01,
    1.66007664274403694e-03 * DAYS_PER_YEAR,
    7.69901118419740425e-03 * DAYS_PER_YEAR,
    -6.90460016972063023e-05 * DAYS_PER_YEAR,
    9.54791938424326609e-04 * SOLAR_MASS
};

static constexpr Body saturn = {
    8.34336671824457987e+00,
    4.12479856412430479e+00,
    -4.03523417114321381e-01,
    -2.76742510726862411e-03 * DAYS_PER_YEAR,
    4.99852801234917238e-03 * DAYS_PER_YEAR,
    2.30417297573763929e-05 * DAYS_PER_YEAR,
    2.85885980666130812e-04 * SOLAR_MASS
};

static constexpr Body uranus = {
    1.28943695621391310e+01,
    -1.51111514016986312e+01,
    -2.23307578892655734e-01,
    2.96460137564761618e-03 * DAYS_PER_YEAR,
    2.37847173959480950e-03 * DAYS_PER_YEAR,
    -2.96589568540237556e-05 * DAYS_PER_YEAR,
    4.36624404335156298e-05 * SOLAR_MASS
};

static constexpr Body neptune = {
    1.53796971148509165e+01,
    -2.59193146099879641e+01,
    1.79258772950371181e-01,
    2.68067772490389322e-03 * DAYS_PER_YEAR,
    1.62824170038242295e-03 * DAYS_PER_YEAR,
    -9.51592254519715870e-05 * DAYS_PER_YEAR,
    5.15138902046611451e-05 * SOLAR_MASS
};

constexpr void offset_momentum(System& bodies)
{
    double px = 0.0;
    double py = 0.0;
    double pz = 0.0;

    static_for<0, N_BODIES>([&](auto i) {
        px += bodies[i].vx * bodies[i].mass;
        py += bodies[i].vy * bodies[i].mass;
        pz += bodies[i].vz * bodies[i].mass;
    });

    bodies[0].vx = -px / SOLAR_MASS;
    bodies[0].vy = -py / SOLAR_MASS;
    bodies[0].vz = -pz / SOLAR_MASS;
}

constexpr void advance(System& bodies, double dt)
{
    static_for<0, N_BODIES>([&](auto i) {
        static_for<i + 1, N_BODIES>([&](auto j) {
            double dx = bodies[i].x - bodies[j].x;
            double dy = bodies[i].y - bodies[j].y;
            double dz = bodies[i].z - bodies[j].z;

            double dSquared = dx * dx + dy * dy + dz * dz;
            double mag = dt / (dSquared * std::sqrt(dSquared));

            bodies[i].vx -= dx * bodies[j].mass * mag;
            bodies[i].vy -= dy * bodies[j].mass * mag;
            bodies[i].vz -= dz * bodies[j].mass * mag;

            bodies[j].vx += dx * bodies[i].mass * mag;
            bodies[j].vy += dy * bodies[i].mass * mag;
            bodies[j].vz += dz * bodies[i].mass * mag;
        });
    });

    static_for<0, N_BODIES>([&](auto i) {
        bodies[i].x += dt * bodies[i].vx;
        bodies[i].y += dt * bodies[i].vy;
        bodies[i].z += dt * bodies[i].vz;
    });
}

constexpr double energy(const System& bodies)
{
    double e = 0.0;

    static_for<0, N_BODIES>([&](auto i) {
        const Body& iBody = bodies[i];
        e += 0.5 * iBody.mass * (iBody.vx * iBody.vx + iBody.vy * iBody.vy + iBody.vz * iBody.vz);

        static_for<i + 1, N_BODIES>([&](auto j) {
            double dx = iBody.x - bodies[j].x;
            double dy = iBody.y - bodies[j].y;
            double dz = iBody.z - bodies[j].z;

            double distance = std::sqrt(dx * dx + dy * dy + dz * dz);
            e -= (iBody.mass * bodies[j].mass) / distance;
        });
    });

    return e;
}

int main(int argc, char* argv[])
{
    const auto n = std::atoi(argv[1]);

    System system = {
        sun,
        jupiter,
        saturn,
        uranus,
        neptune
    };

    offset_momentum(system);

    std::printf("%.9f\n", energy(system));

    for (size_t i = 0; i < n; ++i)
        advance(system, 0.01);

    std::printf("%.9f\n", energy(system));
}
