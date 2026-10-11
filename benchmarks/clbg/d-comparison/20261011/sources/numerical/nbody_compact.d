// D array-operation adaptation of the archived classless n-body implementation.
import std: PI, sqrt, dotProduct, to, writefln;
enum solar = 4 * PI * PI;
enum year = 365.24;
struct Body {
    double[3] p;
    double[3] v;
    double mass;
}
Body[5] bodies = [
    Body([0, 0, 0], [0, 0, 0], solar),
    Body([4.84143144246472090, -1.16032004402742839, -0.103622044471123139], [0.00166007664274403694*year, 0.00769901118419740425*year, -0.0000690460016972063023*year], 0.000954791938424326609*solar),
    Body([8.34336671824457987, 4.12479856412430479, -0.403523417114321381], [-0.00276742510726862411*year, 0.00499852801234917238*year, 0.0000230417297573763929*year], 0.000285885980666130812*solar),
    Body([12.8943695621391310, -15.1111514016986312, -0.223307578892655734], [0.00296460137564761618*year, 0.00237847173959480950*year, -0.0000296589568540237556*year], 0.0000436624404335156298*solar),
    Body([15.3796971148509165, -25.9193146099879641, 0.179258772950371181], [0.00268067772490389322*year, 0.00162824170038242295*year, -0.0000951592254519715870*year], 0.0000515138902046611451*solar)
];
void advance(double dt) {
    foreach (i; 0..bodies.length) {
        foreach (j; i+1..bodies.length) {
            double[3] d;
            d[] = bodies[i].p[] - bodies[j].p[];
            double q = dotProduct(d, d);
            double mag = dt / (q * sqrt(q));
            bodies[i].v[] -= d[] * (bodies[j].mass * mag);
            bodies[j].v[] += d[] * (bodies[i].mass * mag);
        }
    }
    foreach (ref b; bodies)
        b.p[] += dt * b.v[];
}
double energy() {
    double e = 0;
    foreach (i, ref b; bodies) {
        e += 0.5 * b.mass * dotProduct(b.v, b.v);
        foreach (j; i+1..bodies.length) {
            double[3] d;
            d[] = b.p[] - bodies[j].p[];
            e -= b.mass * bodies[j].mass / sqrt(dotProduct(d, d));
        }
    }
    return e;
}
void main(string[] args) {
    double[3] momentum = 0;
    foreach (b; bodies)
        momentum[] += b.v[] * b.mass;
    bodies[0].v[] = -momentum[] / solar;
    writefln("%.9f", energy());
    foreach (_; 0..to!int(args[1]))
        advance(0.01);
    writefln("%.9f", energy());
}
