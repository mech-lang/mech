// Compact D implementation of the canonical 10-step AtA power iteration.
import std: to, sqrt, writefln;
double a(int i, int j) {
    return 1.0 / ((i+j)*(i+j+1)/2+i+1);
}
void multiply(bool transpose)(double[] output, const(double)[] input) {
    foreach (int i; 0..cast(int)input.length) {
        double s = 0;
        foreach (int j; 0..cast(int)input.length)
            s += input[j] * (transpose ? a(j,i) : a(i,j));
        output[i] = s;
    }
}
void step(double[] output, const(double)[] input, double[] temporary) {
    multiply!false(temporary, input);
    multiply!true(output, temporary);
}
void main(string[] args) {
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
