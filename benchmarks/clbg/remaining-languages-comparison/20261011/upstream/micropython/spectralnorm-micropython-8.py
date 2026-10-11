# The Computer Language Benchmarks Game
#   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
#
#   Naive transliteration from Sebastien Loisel's C program
#   contributed by Isaac Gouy
#

from math import sqrt
import sys

def eval_A(i, j):
  return 1.0/((i+j)*(i+j+1)/2+i+1)

def eval_A_times_u(N, u, Au):
  for i in range(N):  
    Au[i]=0
    for j in range(N): Au[i]+=eval_A(i,j)*u[j]      

def eval_At_times_u(N, u, Au):
  for i in range(N):  
    Au[i]=0
    for j in range(N): Au[i]+=eval_A(j,i)*u[j]   

def eval_AtA_times_u(N, u, AtAu):
  v=[0]*N; eval_A_times_u(N,u,v); eval_At_times_u(N,v,AtAu)

def main(n):
  u=[1]*n
  v=[0]*n
  for i in range(10):
    eval_AtA_times_u(n,u,v)
    eval_AtA_times_u(n,v,u)
  vBv=vv=0
  for i in range(n): vBv+=u[i]*v[i]; vv+=v[i]*v[i]  
  print("%.9f" % sqrt(vBv/vv))
  
if __name__ == '__main__':
  main( int(sys.argv[1]) if len(sys.argv) > 1 else 100 )
