/* The Great Computer Language Shootout
   http://shootout.alioth.debian.org/

   converted to D by Dave Fladebo
   compile: dmd -O -inline -release spectralnorm.d
*/

import std.math, std.stdio, std.string;

void main(char[][] args)
{
    int      N = args.length > 1 ? cast(int)atoi(args[1]) : 2000;
    double[] u = new double[N], v = new double[N], w = new double[N];
    double   vBv = 0, vv = 0;

    u[] = 1.0;
    for(int i = 0; i < 10; i++)
    {
        eval_AtA_times_u(u,v,w);
        eval_AtA_times_u(v,u,w);
    }

    foreach(int i, double vi; v)
    {
        vBv += u[i] * vi;
        vv  += vi * vi;
    }
    writefln("%0.9f",sqrt(vBv/vv));
}

void eval_AtA_times_u(double[] u, double[] v, double[] w)
{
    eval_A_times_u(u,w);
    eval_At_times_u(w,v);
}

void eval_A_times_u(double[] u, double[] Au)
{
    foreach(int i, inout double Aui; Au)
    {
        Aui = 0.0;
        foreach(int j, double uj; u)
        {
            Aui += eval_A(i,j) * u[j];
        }
    }
}

void eval_At_times_u(double[] u, double[] Au)
{
    foreach(int i, inout double Aui; Au)
    {
        Aui = 0.0;
        foreach(int j, double uj; u)
        {
            Aui += eval_A(j,i) * uj;
        }
    }
}

double eval_A(int i, int j)
{
    return 1.0/(((i+j)*(i+j+1)/2)+i+1);
}
