/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   Naive transliteration from Sebastien Loisel's C program
   contributed by Isaac Gouy
*/

import 'dart:math' as math;
import 'dart:typed_data';

double eval_A(int i, int j) { return 1.0/((i+j)*(i+j+1)/2+i+1); }  

void eval_A_times_u(int N, Float64List u, Float64List Au)
{
  int i,j;
  for(i=0;i<N;i++)
    {
      Au[i]=0;
      for(j=0;j<N;j++) Au[i]+=eval_A(i,j)*u[j];
    }
} 

void eval_At_times_u(int N, Float64List u, Float64List Au)
{
  int i,j;
  for(i=0;i<N;i++)
    {
      Au[i]=0;
      for(j=0;j<N;j++) Au[i]+=eval_A(j,i)*u[j];
    }
}  

void eval_AtA_times_u(int N, Float64List u, Float64List AtAu)
{ var v = new Float64List(N); eval_A_times_u(N,u,v); eval_At_times_u(N,v,AtAu); }  
  
void main(args)
{
  int i;
  int N = args.length > 0 ? int.parse(args[0]) : 100;
  var u = new Float64List(N)..fillRange(0, N, 1.0),
      v = new Float64List(N);        
  double vBv,vv;      
  for(i=0;i<10;i++)
    {
      eval_AtA_times_u(N,u,v);
      eval_AtA_times_u(N,v,u);
    }
  vBv=vv=0;
  for(i=0;i<N;i++) { vBv+=u[i]*v[i]; vv+=v[i]*v[i]; }
  print(math.sqrt(vBv/vv).toStringAsFixed(9));  
}
