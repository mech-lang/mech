module clbg::nbody;
import libc;
import std::math;

struct Body
{
    double x;
    double y;
    double z;
    double vx;
    double vy;
    double vz;
    double mass;
}

const double SOLAR_MASS = 4.0 * 3.141592653589793 * 3.141592653589793;
const double YEAR = 365.24;

fn double energy(Body* bodies)
{
    double result = 0.0;
    for (int i = 0; i < 5; i++)
    {
        Body* a = &bodies[i];
        result += 0.5 * a.mass * (a.vx * a.vx + a.vy * a.vy + a.vz * a.vz);
        for (int j = i + 1; j < 5; j++)
        {
            Body* b = &bodies[j];
            double dx = a.x - b.x;
            double dy = a.y - b.y;
            double dz = a.z - b.z;
            result -= a.mass * b.mass / math::sqrt(dx * dx + dy * dy + dz * dz);
        }
    }
    return result;
}

struct Pair
{
    int i;
    int j;
    double dx;
    double dy;
    double dz;
    double magnitude;
}

fn void advance(Body* bodies)
{
    Pair[10] pairs;
    int index = 0;
    for (int i = 0; i < 5; i++)
    {
        for (int j = i + 1; j < 5; j++)
        {
            double dx = bodies[i].x - bodies[j].x;
            double dy = bodies[i].y - bodies[j].y;
            double dz = bodies[i].z - bodies[j].z;
            double squared = dx * dx + dy * dy + dz * dz;
            pairs[index] = { i, j, dx, dy, dz, 0.01 / (squared * math::sqrt(squared)) };
            index++;
        }
    }
    for (int p = 0; p < 10; p++)
    {
        Pair* pair = &pairs[p];
        Body* a = &bodies[pair.i];
        Body* b = &bodies[pair.j];
        a.vx -= pair.dx * b.mass * pair.magnitude;
        a.vy -= pair.dy * b.mass * pair.magnitude;
        a.vz -= pair.dz * b.mass * pair.magnitude;
        b.vx += pair.dx * a.mass * pair.magnitude;
        b.vy += pair.dy * a.mass * pair.magnitude;
        b.vz += pair.dz * a.mass * pair.magnitude;
    }
    for (int i = 0; i < 5; i++)
    {
        bodies[i].x += 0.01 * bodies[i].vx;
        bodies[i].y += 0.01 * bodies[i].vy;
        bodies[i].z += 0.01 * bodies[i].vz;
    }
}

fn int main(int argc, char** argv)
{
    Body[5] bodies = {
        { 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, SOLAR_MASS },
        { 4.84143144246472090, -1.16032004402742839, -1.03622044471123109e-1,
          1.66007664274403694e-3 * YEAR, 7.69901118419740425e-3 * YEAR,
          -6.90460016972063023e-5 * YEAR, 9.54791938424326609e-4 * SOLAR_MASS },
        { 8.34336671824457987, 4.12479856412430479, -4.03523417114321381e-1,
          -2.76742510726862411e-3 * YEAR, 4.99852801234917238e-3 * YEAR,
          2.30417297573763929e-5 * YEAR, 2.85885980666130812e-4 * SOLAR_MASS },
        { 1.28943695621391310e1, -1.51111514016986312e1, -2.23307578892655734e-1,
          2.96460137564761618e-3 * YEAR, 2.37847173959480950e-3 * YEAR,
          -2.96589568540237556e-5 * YEAR, 4.36624404335156298e-5 * SOLAR_MASS },
        { 1.53796971148509165e1, -2.59193146099879641e1, 1.79258772950371181e-1,
          2.68067772490389322e-3 * YEAR, 1.62824170038242295e-3 * YEAR,
          -9.51592254519715870e-5 * YEAR, 5.15138902046611451e-5 * SOLAR_MASS }
    };
    double px = 0.0;
    double py = 0.0;
    double pz = 0.0;
    for (int i = 0; i < 5; i++)
    {
        px += bodies[i].vx * bodies[i].mass;
        py += bodies[i].vy * bodies[i].mass;
        pz += bodies[i].vz * bodies[i].mass;
    }
    bodies[0].vx = -px / SOLAR_MASS;
    bodies[0].vy = -py / SOLAR_MASS;
    bodies[0].vz = -pz / SOLAR_MASS;
    long n = libc::atoll(argv[1]);
    if (n < 0)
    {
        return 2;
    }
    libc::printf("%.9f\n", energy(&bodies[0]));
    for (long step = 0; step < n; step++)
    {
        advance(&bodies[0]);
    }
    libc::printf("%.9f\n", energy(&bodies[0]));
    return 0;
}
