// The Computer Language Benchmarks Game
// http://shootout.alioth.debian.org/

// contributed by Christoph Bauer
// Class-less version optimized for speed by bearophile
// Compile: dmd -O -release -inline nbody.d

import std.string: atoi;
import std.math: sqrt, PI;

const double SOLAR_MASS = 4 * PI * PI;
const double DAYS_PER_YEAR = 365.24;
const int NBODIES = 5;

struct planet {
    double x, y, z, vx, vy, vz, mass;
}

typedef planet[NBODIES] TyPlanets;

TyPlanets bodies = [
    {                                                             /* SUN */
        0.0, 0., 0., 0., 0., 0., SOLAR_MASS
    },
    {                                                             /* JUPITER */
        4.84143144246472090E+00,
        -1.16032004402742839E+00,
        -1.03622044471123109E-01,
        1.66007664274403694E-03 * DAYS_PER_YEAR,
        7.69901118419740425E-03 * DAYS_PER_YEAR,
        -6.90460016972063023E-05 * DAYS_PER_YEAR,
        9.54791938424326609E-04 * SOLAR_MASS
    },
    {                                                             /* SATURN */
        8.34336671824457987E+00,
        4.12479856412430479E+00,
        -4.03523417114321381E-01,
        -2.76742510726862411E-03 * DAYS_PER_YEAR,
        4.99852801234917238E-03 * DAYS_PER_YEAR,
        2.30417297573763929E-05 * DAYS_PER_YEAR,
        2.85885980666130812E-04 * SOLAR_MASS
    },
    {                                                             /* URANUS */
        1.28943695621391310E+01,
        -1.51111514016986312E+01,
        -2.23307578892655734E-01,
        2.96460137564761618E-03 * DAYS_PER_YEAR,
        2.37847173959480950E-03 * DAYS_PER_YEAR,
        -2.96589568540237556E-05 * DAYS_PER_YEAR,
        4.36624404335156298E-05 * SOLAR_MASS
    },
    {                                                             /* NEPTUNE */
        1.53796971148509165E+01,
        -2.59193146099879641E+01,
        1.79258772950371181E-01,
        2.68067772490389322E-03 * DAYS_PER_YEAR,
        1.62824170038242295E-03 * DAYS_PER_YEAR,
        -9.51592254519715870E-05 * DAYS_PER_YEAR,
        5.15138902046611451E-05 * SOLAR_MASS
    }
];


void offset_momentum(TyPlanets bodies) {
    double px = 0.0, py = 0.0, pz = 0.0;
    foreach(ref b; bodies) {
        px += b.vx * b.mass;
        py += b.vy * b.mass;
        pz += b.vz * b.mass;
    }
    bodies[0].vx = - px / SOLAR_MASS;
    bodies[0].vy = - py / SOLAR_MASS;
    bodies[0].vz = - pz / SOLAR_MASS;
}


double energy(TyPlanets bodies) {
    double e = 0.0;
    foreach(idx, ref b1; bodies) {
        double im = b1.mass;
        e += 0.5 * im * (b1.vx * b1.vx + b1.vy * b1.vy + b1.vz * b1.vz);

        foreach(ref b2; bodies[idx + 1 .. length]) {
            double dx = b1.x - b2.x;
            double dy = b1.y - b2.y;
            double dz = b1.z - b2.z;

            e -= (im * b2.mass) / sqrt(dx*dx + dy*dy + dz*dz);
        }
    }
    return e;
}


void advance(TyPlanets bodies, double dt) {
    foreach(idx, ref b; bodies) {
        double bm = b.mass;
        foreach(ref b2; bodies[idx + 1 .. length]) {
            double b2m = b2.mass;
            double dx = b.x - b2.x;
            double dy = b.y - b2.y;
            double dz = b.z - b2.z;
            double distance = sqrt(dx * dx + dy * dy + dz * dz);
            double mag = dt / (distance * distance * distance);
            b.vx -= dx * b2m * mag;
            b.vy -= dy * b2m * mag;
            b.vz -= dz * b2m * mag;
            b2.vx += dx * bm * mag;
            b2.vy += dy * bm * mag;
            b2.vz += dz * bm * mag;
        }
    }
    foreach(ref b; bodies) {
        b.x += dt * b.vx;
        b.y += dt * b.vy;
        b.z += dt * b.vz;
    }
}


void main(char[][] args) {
    int n = args.length > 1 ? atoi(args[1]) : 1;

    offset_momentum(bodies);
    printf("%.9f\n", energy(bodies));

    for (int i = 1; i <= n; i++)
        advance(bodies, 0.01);

    printf("%.9f\n", energy(bodies));
}
