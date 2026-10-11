// The Computer Language Benchmarks Game
// https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
//
// Based on Swift program #6 by Ralph Ganszky
// Contributed by Arseniy Zlobintsev

using System.Runtime.CompilerServices;
using Body = (
    (double X, double Y, double Z) P,
    (double X, double Y, double Z) V, double M);

const double SOLAR_MASS = 4 * Math.PI * Math.PI;
const double DAYS_PER_YEAR = 365.24;

var n = args.Length > 0 ? int.Parse(args[0]) : 10000;

var sun = ((0.0, 0.0, 0.0), (0.0, 0.0, 0.0), SOLAR_MASS);
var jupiter = ((
    X: 4.8414314424647209, Y: -1.16032004402742839, Z: -0.103622044471123109), (
    X: 1.66007664274403694e-03 * DAYS_PER_YEAR,
    Y: 7.69901118419740425e-03 * DAYS_PER_YEAR,
    Z: -6.90460016972063023e-05 * DAYS_PER_YEAR),
    M: 9.54791938424326609e-04 * SOLAR_MASS);
var saturn = ((
    X: 8.34336671824457987, Y: 4.12479856412430479, Z: -4.03523417114321381e-01), (
    X: -2.76742510726862411e-03 * DAYS_PER_YEAR,
    Y: 4.99852801234917238e-03 * DAYS_PER_YEAR,
    Z: 2.30417297573763929e-05 * DAYS_PER_YEAR),
    M: 2.85885980666130812e-04 * SOLAR_MASS);
var uranus = ((
    X: 1.2894369562139131e+01, Y: -1.51111514016986312e+01, Z: -2.23307578892655734e-01), (
    X: 2.96460137564761618e-03 * DAYS_PER_YEAR,
    Y: 2.3784717395948095e-03 * DAYS_PER_YEAR,
    Z: -2.96589568540237556e-05 * DAYS_PER_YEAR),
    M: 4.36624404335156298e-05 * SOLAR_MASS);
var neptune = ((
    X: 1.53796971148509165e+01, Y: -2.59193146099879641e+01, Z: 1.79258772950371181e-01), (
    X: 2.68067772490389322e-03 * DAYS_PER_YEAR,
    Y: 1.62824170038242295e-03 * DAYS_PER_YEAR,
    Z: -9.5159225451971587e-05 * DAYS_PER_YEAR),
    M: 5.15138902046611451e-05 * SOLAR_MASS);

Span<Body> bodies = [sun, jupiter, saturn, uranus, neptune];

var (px, py, pz) = (0.0, 0.0, 0.0);
foreach (var (_, v, m) in bodies) {
    px += v.X * m;
    py += v.Y * m;
    pz += v.Z * m;
}
bodies[0].V = (-px / SOLAR_MASS, -py / SOLAR_MASS, -pz / SOLAR_MASS);

Console.WriteLine("{0:F9}", Energy(bodies));
for (var i = 0; i < n; i++) Advance(bodies, 0.01);
Console.WriteLine("{0:F9}", Energy(bodies));

[MethodImpl(MethodImplOptions.AggressiveInlining)]
static void Advance(Span<Body> bodies, double dt) {
    for (var i = 0; i < bodies.Length; i++) {
        var (p, (vx, vy, vz), mi) = bodies[i];
        for (var j = i + 1; j < bodies.Length; j++) {
            var dx = p.X - bodies[j].P.X;
            var dy = p.Y - bodies[j].P.Y;
            var dz = p.Z - bodies[j].P.Z;

            var mj = bodies[j].M;
            var d2 = dx * dx + dy * dy + dz * dz;
            var mag = dt / (d2 * Math.Sqrt(d2));

            vx -= dx * mj * mag;
            vy -= dy * mj * mag;
            vz -= dz * mj * mag;
            bodies[j].V.X += dx * mi * mag;
            bodies[j].V.Y += dy * mi * mag;
            bodies[j].V.Z += dz * mi * mag;
        }
        bodies[i].V = (vx, vy, vz);
    }
    foreach (ref var body in bodies) {
        body.P = (
            body.P.X + dt * body.V.X,
            body.P.Y + dt * body.V.Y,
            body.P.Z + dt * body.V.Z);
    }
}

static double Energy(Span<Body> bodies) {
    var e = 0.0;
    for (var i = 0; i < bodies.Length; i++) {
        e += 0.5 * bodies[i].M * (
            bodies[i].V.X * bodies[i].V.X +
            bodies[i].V.Y * bodies[i].V.Y +
            bodies[i].V.Z * bodies[i].V.Z);
        for (var j = i + 1; j < bodies.Length; j++) {
            var dx = bodies[i].P.X - bodies[j].P.X;
            var dy = bodies[i].P.Y - bodies[j].P.Y;
            var dz = bodies[i].P.Z - bodies[j].P.Z;
            var distance = Math.Sqrt(dx * dx + dy * dy + dz * dz);
            e -= bodies[i].M * bodies[j].M / distance;
        }
    }
    return e;
}
