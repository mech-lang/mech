// The Computer Language Benchmarks Game
// https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
//
// Original C contributed by Sebastien Loisel
// Conversion to C++ by Jon Harrop
// Compile: g++ -O3 -o spectralnorm spectralnorm.cpp


#include <stdio.h>
#include <stdlib.h>
#include <cmath>
#include <vector>
#include <iostream>
#include <iomanip>

using namespace std;

double eval_A(int i, int j) { return 1.0 / ((i+j)*(i+j+1)/2 + i + 1); }

void eval_A_times_u(const vector<double> &u, vector<double> &Au)
{
  for(int i=0; i<u.size(); i++)
    for(int j=0; j<u.size(); j++) Au[i] += eval_A(i,j) * u[j];
}

void eval_At_times_u(const vector<double> &u, vector<double> &Au)
{
  for(int i=0; i<u.size(); i++)
    for(int j=0; j<u.size(); j++) Au[i] += eval_A(j,i) * u[j];
}

void eval_AtA_times_u(const vector<double> &u, vector<double> &AtAu)
{ vector<double> v(u.size()); eval_A_times_u(u, v); eval_At_times_u(v, AtAu); }

int main(int argc, char *argv[])
{
  int N = ((argc == 2) ? atoi(argv[1]) : 2000);
  vector<double> u(N), v(N);

  fill(u.begin(), u.end(), 1);

  for(int i=0; i<10; i++) {
    eval_AtA_times_u(u, v);
    fill(u.begin(), u.end(), 0);
    eval_AtA_times_u(v, u);
  }

  double vBv=0, vv=0;
  for(int i=0; i<N; i++) { vBv += u[i]*v[i]; vv += v[i]*v[i]; }

  cout << setprecision(10) << sqrt(vBv/vv) << endl;

  return 0;
}
