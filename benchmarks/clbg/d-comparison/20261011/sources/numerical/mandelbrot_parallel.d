module mandelbrot_parallel;


import std.stdio: writefln, stdout;
import std.conv: to;
import std.range: iota;
import std.parallelism: taskPool, defaultPoolThreads;


static immutable VLEN = 8;

alias Vec = double[8];

pragma(inline, true):
ubyte mbrot8(Vec cr, double civ) {
    immutable Vec ci = civ;
    Vec zr = 0.0;
    Vec zi = 0.0;
    Vec tr = 0.0;
    Vec ti = 0.0;
    Vec absz = 0.0;
    foreach(l; 0 .. 10) {
        foreach(k; 0 .. 5) {
            zi[] = (zr[] + zr[]) * zi[] + ci[];
            zr[] = tr[] - ti[] + cr[];
            tr[] = zr[] * zr[];
            ti[] = zi[] * zi[];
        }
        absz[] = tr[] + ti[];
        bool terminate = true;
        foreach(i; 0 .. 8)
            if (absz[i] <= 4.0) {
                terminate = false;
                break;
            }
        if (terminate)
            return 0u;
    }
    ubyte accu;
    foreach(i; 0 .. VLEN)
        if (absz[i] <= 4.0) {
            accu |= 0x80 >> i;
        }
    return accu;
}

void main(string[] args) {
    defaultPoolThreads = 7;
    auto n = args.length > 1 ? args[1].to!int() : 200;
    auto size = n;
    auto chunk_size = (size + VLEN - 1) / VLEN;
    writefln("P4\n%d %d", size, size);
    auto inv = 2.0 / size;
    Vec[] xloc = new Vec[chunk_size];
    foreach(i, ref arr; xloc)
        foreach(j, ref el; arr)
            el = (i * 8 + j) * inv - 1.5;

    auto pixels = new ubyte[size * chunk_size];
    foreach(y; taskPool.parallel(iota(0,size))) {
        double ci = inv * y - 1.0;
        foreach(x; 0 .. chunk_size) {
            auto r = mbrot8(xloc[x], ci);
            pixels[y*chunk_size + x] = r;
        }
        if (size % VLEN)
            pixels[y*chunk_size + chunk_size - 1] &= cast(ubyte)(0xFF << (VLEN - size % VLEN));
    }
    stdout.rawWrite(pixels);
}
