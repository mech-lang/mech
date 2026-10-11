/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   contributed by Martin Jambrek
   based off the Java #2 program contributed by Mark C. Lewis and modified
   slightly by Chad Whipkey
   modified by Basit Ayantunde
*/

#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <utility>

constexpr double PI = 3.141592653589793;
constexpr double SOLAR_MASS = 4 * PI * PI;
constexpr double DAYS_PER_YEAR = 365.24;

struct vec3 {
  double x = 0, y = 0, z = 0, __pad = 0;
  constexpr vec3() {}
  constexpr vec3(double x, double y, double z) : x{x}, y{y}, z{z} {}

  constexpr vec3 operator+(vec3 const &other) const {
    return {x + other.x, y + other.y, z + other.z};
  }

  constexpr vec3 operator-(vec3 const &other) const {
    return {x - other.x, y - other.y, z - other.z};
  }

  constexpr vec3 operator*(vec3 const &other) const {
    return {x * other.x, y * other.y, z * other.z};
  }

  constexpr vec3 operator*(double other) const {
    return {x * other, y * other, z * other};
  }

  constexpr double sum() const { return x + y + z; }

  constexpr vec3 sqrt() const {
    return {std::sqrt(x), std::sqrt(y), std::sqrt(z)};
  }

  constexpr double distance(vec3 const &other) const {
    vec3 d = *this - other;
    return std::sqrt((d * d).sum());
  }
};

struct Body {
  vec3 x;
  vec3 v;
  double mass = 0;
};

constexpr size_t N_BODIES = 5;

constexpr Body jupiter{{4.84143144246472090e+00, -1.16032004402742839e+00,
                        -1.03622044471123109e-01},
                       {1.66007664274403694e-03 * DAYS_PER_YEAR,
                        7.69901118419740425e-03 * DAYS_PER_YEAR,
                        -6.90460016972063023e-05 * DAYS_PER_YEAR},
                       9.54791938424326609e-04 * SOLAR_MASS};

constexpr Body saturn{{8.34336671824457987e+00, 4.12479856412430479e+00,
                       -4.03523417114321381e-01},
                      {-2.76742510726862411e-03 * DAYS_PER_YEAR,
                       4.99852801234917238e-03 * DAYS_PER_YEAR,
                       2.30417297573763929e-05 * DAYS_PER_YEAR},
                      2.85885980666130812e-04 * SOLAR_MASS};

constexpr Body uranus{{1.28943695621391310e+01, -1.51111514016986312e+01,
                       -2.23307578892655734e-01},
                      {2.96460137564761618e-03 * DAYS_PER_YEAR,
                       2.37847173959480950e-03 * DAYS_PER_YEAR,
                       -2.96589568540237556e-05 * DAYS_PER_YEAR},
                      4.36624404335156298e-05 * SOLAR_MASS};

constexpr Body neptune{{1.53796971148509165e+01, -2.59193146099879641e+01,
                        1.79258772950371181e-01},
                       {2.68067772490389322e-03 * DAYS_PER_YEAR,
                        1.62824170038242295e-03 * DAYS_PER_YEAR,
                        -9.51592254519715870e-05 * DAYS_PER_YEAR},
                       5.15138902046611451e-05 * SOLAR_MASS};

constexpr void offset_momentum(Body bodies[N_BODIES]) {
  Body &sun = bodies[0];

  for (size_t i = 1; i < N_BODIES; i++) {
    double m_ratio = bodies[i].mass / SOLAR_MASS;
    sun.v = sun.v - bodies[i].v * m_ratio;
  }
}

constexpr double energy(Body const bodies[N_BODIES]) {
  double e = 0;

  for (size_t i = 0; i < N_BODIES; i++) {
    Body const &body = bodies[i];
    e += body.mass * (body.v * body.v).sum() * 0.5;

    for (size_t j = i + 1; j < N_BODIES; j++) {
      double distance = body.x.distance(bodies[j].x);
      e -= body.mass * bodies[j].mass / distance;
    }
  }

  return e;
}

constexpr void advance(Body bodies[N_BODIES], double dt) {
  constexpr size_t N = N_BODIES * (N_BODIES - 1) / 2;

  vec3 r[N] = {};

  {
    size_t i = 0;

    for (size_t j = 0; j < N_BODIES; j++) {
      for (size_t k = j + 1; k < N_BODIES; k++) {
        r[i] = bodies[j].x - bodies[k].x;
        i += 1;
      }
    }
  }

  double mag[N] = {};
  size_t i = 0;

  while (i < N) {
    double d2s_x = (r[i] * r[i]).sum();
    double d2s_y = (r[i + 1] * r[i + 1]).sum();
    double dmag_x = dt / (d2s_x * std::sqrt(d2s_x));
    double dmag_y = dt / (d2s_y * std::sqrt(d2s_y));
    mag[i] = dmag_x;
    mag[i + 1] = dmag_y;
    i += 2;
  }

  i = 0;
  for (size_t j = 0; j < N_BODIES; j++) {
    for (size_t k = j + 1; k < N_BODIES; k++) {
      vec3 f = r[i] * mag[i];
      bodies[j].v = bodies[j].v - f * bodies[k].mass;
      bodies[k].v = bodies[k].v + f * bodies[j].mass;
      i += 1;
    }
  }

  for (size_t i = 0; i < N_BODIES; i++) {
    bodies[i].x = bodies[i].x + bodies[i].v * dt;
  }
}

int main(int argc, char *argv[]) {
  const auto n = std::atoi(argv[1]);

  Body system[N_BODIES] = {
      {{}, {}, SOLAR_MASS}, jupiter, saturn, uranus, neptune};

  offset_momentum(system);

  std::printf("%.9f\n", energy(system));

  for (size_t i = 0; i < n; ++i)
    advance(system, 0.01);

  std::printf("%.9f\n", energy(system));
}
