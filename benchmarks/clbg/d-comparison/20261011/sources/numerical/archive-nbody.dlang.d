/* The Great Computer Language Shootout
   http://shootout.alioth.debian.org/

   contributed by Mark C. Lewis

   converted to D by Dave Fladebo
   compile: dmd -O -inline -release nbody.d
*/

import std.math, std.stdio, std.string;

int main(char[][] args)
{
    int n = args.length > 1 ? std.string.atoi(args[1]) : 1;

    NBodySystem bodies = new NBodySystem();

    writefln("%0.9f",bodies.energy());
    for(int i = 0; i < n; i++)
    {
        bodies.advance(0.01);
    }
    writefln("%0.9f",bodies.energy());

    return 0;
}

class NBodySystem
{
public:
    this()
    {
        bodies[0] = sun;
        bodies[1] = jupiter;
        bodies[2] = saturn;
        bodies[3] = uranus;
        bodies[4] = neptune;

        double px = 0.0;
        double py = 0.0;
        double pz = 0.0;

        foreach(Body i; bodies)
        {
            px += i.vx * i.mass;
            py += i.vy * i.mass;
            pz += i.vz * i.mass;
        }

        bodies[0].offsetMomentum(px,py,pz);
    }

    void advance(double dt)
    {
        double dx, dy, dz, distance, mag, im, jm;

        foreach(int idx, inout Body i; bodies)
        {
            im = i.mass;
            foreach(inout Body j; bodies[idx + 1 .. length])
            {
                jm = j.mass;

                dx = i.x - j.x;
                dy = i.y - j.y;
                dz = i.z - j.z;

                distance = sqrt(dx*dx + dy*dy + dz*dz);
                mag = dt / (distance * distance * distance);

                i.vx -= dx * jm * mag;
                i.vy -= dy * jm * mag;
                i.vz -= dz * jm * mag;

                j.vx += dx * im * mag;
                j.vy += dy * im * mag;
                j.vz += dz * im * mag;
            }
        }

        foreach(inout Body i; bodies)
        {
            i.x += dt * i.vx;
            i.y += dt * i.vy;
            i.z += dt * i.vz;
        }
    }

    double energy()
    {
        double dx, dy, dz, im, e = 0.0;

        foreach(int idx, Body i; bodies)
        {
            im = i.mass;
            e += 0.5 * im * (i.vx * i.vx + i.vy * i.vy + i.vz * i.vz);

            foreach(Body j; bodies[idx + 1 .. length])
            {
                dx = i.x - j.x;
                dy = i.y - j.y;
                dz = i.z - j.z;

                e -= (im * j.mass) / sqrt(dx*dx + dy*dy + dz*dz);
            }
        }

        return e;
    }

private:
    const double SOLAR_MASS = 4 * PI * PI;
    const double DAYS_PER_YEAR = 365.24;

    struct Body
    {
        double x = 0.0, y = 0.0, z = 0.0, vx = 0.0, vy = 0.0, vz = 0.0, mass = 0.0;

        void offsetMomentum(double px, double py, double pz)
        {
            vx = -px / SOLAR_MASS;
            vy = -py / SOLAR_MASS;
            vz = -pz / SOLAR_MASS;
        }
    }

    Body[5] bodies;

    Body sun = { mass: SOLAR_MASS };

    Body jupiter = {
        x: 4.84143144246472090e+00,
        y: -1.16032004402742839e+00,
        z: -1.03622044471123109e-01,
        vx: 1.66007664274403694e-03 * DAYS_PER_YEAR,
        vy: 7.69901118419740425e-03 * DAYS_PER_YEAR,
        vz: -6.90460016972063023e-05 * DAYS_PER_YEAR,
        mass: 9.54791938424326609e-04 * SOLAR_MASS
    };

    Body saturn = {
        x: 8.34336671824457987e+00,
        y: 4.12479856412430479e+00,
        z: -4.03523417114321381e-01,
        vx: -2.76742510726862411e-03 * DAYS_PER_YEAR,
        vy: 4.99852801234917238e-03 * DAYS_PER_YEAR,
        vz: 2.30417297573763929e-05 * DAYS_PER_YEAR,
        mass: 2.85885980666130812e-04 * SOLAR_MASS
    };

    Body uranus = {
        x: 1.28943695621391310e+01,
        y: -1.51111514016986312e+01,
        z: -2.23307578892655734e-01,
        vx: 2.96460137564761618e-03 * DAYS_PER_YEAR,
        vy: 2.37847173959480950e-03 * DAYS_PER_YEAR,
        vz: -2.96589568540237556e-05 * DAYS_PER_YEAR,
        mass: 4.36624404335156298e-05 * SOLAR_MASS
    };

    Body neptune = {
        x: 1.53796971148509165e+01,
        y: -2.59193146099879641e+01,
        z: 1.79258772950371181e-01,
        vx: 2.68067772490389322e-03 * DAYS_PER_YEAR,
        vy: 1.62824170038242295e-03 * DAYS_PER_YEAR,
        vz: -9.51592254519715870e-05 * DAYS_PER_YEAR,
        mass: 5.15138902046611451e-05 * SOLAR_MASS
    };
}
