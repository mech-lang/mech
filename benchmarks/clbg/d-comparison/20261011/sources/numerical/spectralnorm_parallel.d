// Canonical spectral-norm algorithm, with parallel rows and NEON division lanes.
import std: to, sqrt, writefln, iota;
import std.parallelism: taskPool, defaultPoolThreads;
import core.simd: double2;
void multiply(bool transpose)(double[] output, const(double)[] input) {
    int n = cast(int)input.length;
    foreach (i; taskPool.parallel(iota(0,n))) {
        double2 acc = 0;
        int j;
        for (; j+1<n; j+=2) {
            int p = i+j;
            int d = p*(p+1)/2;
            double[2] numerator = [input[j],input[j+1]];
            double[2] denominator = transpose ? [d+j+1.0,d+p+j+3.0] : [d+i+1.0,d+p+i+2.0];
            acc += cast(double2)numerator / cast(double2)denominator;
        }
        double s = acc.array[0]+acc.array[1];
        if (j<n) {
            int p = i+j;
            s += input[j] / (p*(p+1)/2+(transpose ? j : i)+1.0);
        }
        output[i] = s;
    }
}
void step(double[] output, const(double)[] input, double[] temporary) {
    multiply!false(temporary,input);
    multiply!true(output,temporary);
}
void main(string[] args) {
    defaultPoolThreads = 7;
    auto n = to!int(args[1]);
    auto u = new double[n];
    auto v = new double[n];
    auto w = new double[n];
    u[] = 1;
    foreach (_; 0..10) {
        step(v,u,w);
        step(u,v,w);
    }
    double uv = 0;
    double vv = 0;
    foreach (i; 0..n) {
        uv += u[i]*v[i];
        vv += v[i]*v[i];
    }
    writefln("%.9f",sqrt(uv/vv));
}
