/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   An implementation pretty much from scratch, with inspiration from the Rust
   version, which used the idea of saving some of the ingredients of the
   compution in an array instead of recomputing them.
   
   contributed by cvergu
*/

#include <cstdio>
#include <cmath>

constexpr double SOLAR_MASS = 4 * M_PI * M_PI;
constexpr double DAYS_PER_YEAR = 365.24;
constexpr std::size_t BODIES_COUNT = 5;

struct alignas(32) vector3d {
    double x, y, z;

    constexpr double norm() const noexcept
    {
        return x*x + y*y + z*z;
    }

#pragma GCC optimize ("no-math-errno")
    double magnitude(double dt) const noexcept
    {
        double sum = this->norm();
        return dt / (sum * sqrt(sum));
    }
};

constexpr vector3d operator+(vector3d v1, vector3d v2)
{
    return vector3d {v1.x + v2.x, v1.y + v2.y, v1.z + v2.z};
}


constexpr vector3d operator-(vector3d v1, vector3d v2)
{
    return vector3d {v1.x - v2.x, v1.y - v2.y, v1.z - v2.z};
}

constexpr vector3d& operator+=(vector3d& v1, vector3d v2)
{
    v1.x += v2.x;
    v1.y += v2.y;
    v1.z += v2.z;

    return v1;
}

constexpr vector3d& operator-=(vector3d& v1, vector3d v2)
{
    v1.x -= v2.x;
    v1.y -= v2.y;
    v1.z -= v2.z;

    return v1;
}

constexpr vector3d& operator*=(vector3d& v, double mag)
{
    v.x *= mag;
    v.y *= mag;
    v.z *= mag;

    return v;
}

constexpr vector3d operator*(vector3d v, double mag)
{
    return vector3d {v.x * mag, v.y * mag, v.z * mag};
}

constexpr vector3d operator/(vector3d v, double mag)
{
    return vector3d {v.x / mag, v.y / mag, v.z / mag};
}

struct body {
    vector3d position;
    vector3d velocity;
    double mass;
};

void advance(body state[BODIES_COUNT], double dt)
{
    /*
     * We precompute the quantity (r_i - r_j)
     */
    vector3d rij[BODIES_COUNT][BODIES_COUNT];

    for (std::size_t i = 0; i < BODIES_COUNT; ++i)
        for (std::size_t j = i + 1; j < BODIES_COUNT; ++j)
            rij[i][j] = state[i].position - state[j].position;

    double magnitudes[BODIES_COUNT][BODIES_COUNT];

    for (std::size_t i = 0; i < BODIES_COUNT; ++i)
        for (std::size_t j = i + 1; j < BODIES_COUNT; ++j)
            magnitudes[i][j] = rij[i][j].magnitude(dt);

    /*
     * Compute the new speed using v_i = a_i dt, where
     * a_i = \sum_{j \neq i} m_j (r_i - r_j)/|r_i - r_j|^3
     */
    for (std::size_t i = 0; i < BODIES_COUNT; ++i) {
        for (std::size_t j = i + 1; j < BODIES_COUNT; ++j) {
            vector3d dist = rij[i][j];
            double mag = magnitudes[i][j];
            state[i].velocity -= dist * (state[j].mass * mag);
            state[j].velocity += dist * (state[i].mass * mag);
        }
    }

    /*
     * Compute the new positions
     */
    for (std::size_t i = 0; i < BODIES_COUNT; ++i)
        state[i].position += state[i].velocity * dt;
}

void offset_momentum(body state[BODIES_COUNT])
{
    vector3d& sun_velocity = state[0].velocity;

    for (std::size_t i = 1; i < BODIES_COUNT; ++i)
        sun_velocity -= state[i].velocity * state[i].mass / SOLAR_MASS;
}

double energy(const body state[BODIES_COUNT])
{
    double energy = 0;

    for (std::size_t i = 0; i < BODIES_COUNT; ++i) {
        const body& body1 = state[i];
        energy += 0.5 * body1.mass * body1.velocity.norm();
        #pragma clang loop vectorize(enable)
        for (std::size_t j = i + 1; j < BODIES_COUNT; ++j) {
            const body& body2 = state[j];
            vector3d r12 = body1.position - body2.position;
            energy -= body1.mass * body2.mass / sqrt(r12.norm());
        }
    }

    return energy;
}


body state[] = {
        // Sun
        {.position = {0, 0, 0},
         .velocity = {0, 0, 0},
         .mass = SOLAR_MASS},
        // Jupiter
        {.position = {4.84143144246472090e+00,
                      -1.16032004402742839e+00,
                      -1.03622044471123109e-01},
         .velocity = {1.66007664274403694e-03 * DAYS_PER_YEAR,
                      7.69901118419740425e-03 * DAYS_PER_YEAR,
                     -6.90460016972063023e-05 * DAYS_PER_YEAR},
         .mass = 9.54791938424326609e-04 * SOLAR_MASS},
        // Saturn
        {.position = {8.34336671824457987e+00,
                      4.12479856412430479e+00,
                      -4.03523417114321381e-01},
         .velocity = {-2.76742510726862411e-03 * DAYS_PER_YEAR,
                      4.99852801234917238e-03 * DAYS_PER_YEAR,
                      2.30417297573763929e-05 * DAYS_PER_YEAR},
         .mass = 2.85885980666130812e-04 * SOLAR_MASS},
        // Uranus
        {.position = {1.28943695621391310e+01,
                      -1.51111514016986312e+01,
                      -2.23307578892655734e-01},
         .velocity = {2.96460137564761618e-03 * DAYS_PER_YEAR,
                      2.37847173959480950e-03 * DAYS_PER_YEAR,
                      -2.96589568540237556e-05 * DAYS_PER_YEAR},
         .mass = 4.36624404335156298e-05 * SOLAR_MASS},
        // Neptune
        {.position = {1.53796971148509165e+01,
                      -2.59193146099879641e+01,
                      1.79258772950371181e-01},
         .velocity = {2.68067772490389322e-03 * DAYS_PER_YEAR,
                      1.62824170038242295e-03 * DAYS_PER_YEAR,
                      -9.51592254519715870e-05 * DAYS_PER_YEAR},
         .mass = 5.15138902046611451e-05 * SOLAR_MASS}
};


int main(int argc, char** argv) {
    if (argc < 1) return EXIT_FAILURE;

    int n = atoi(argv[1]);

    offset_momentum(state);

    printf("%.9f\n", energy(state));

    for (int i = 0; i < n; ++i)
        advance(state, 0.01);
    printf("%.9f\n", energy(state));
}
