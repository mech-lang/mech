/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   Naive transliteration from Sebastien Loisel's C program
   contributed by Isaac Gouy
*/

import Foundation

func eval_A(_ i: Int, _ j: Int) -> Double 
{ 
   return 1.0 / Double((i+j)*(i+j+1)/2+i+1)   
}

func eval_A_times_u(_ N: Int, _ u: [Double], _ Au: inout [Double])
{
   for i in 0..<N   
   {
      Au[i]=0
      for j in 0..<N { Au[i]+=eval_A(i,j)*u[j] }
   }
} 

func eval_At_times_u(_ N: Int, _ u: [Double], _ Au: inout [Double])
{
   for i in 0..<N   
   {
      Au[i]=0
      for j in 0..<N { Au[i]+=eval_A(j,i)*u[j] }
   }
} 

func eval_AtA_times_u(_ N: Int, _ u: [Double], _ AtAu: inout [Double])
{ 
   var v = Array(repeating: 0.0, count: N) 
   eval_A_times_u(N,u,&v); eval_At_times_u(N,v,&AtAu) 
} 


func main(_ N: Int) 
{ 
   var u = Array(repeating: 1.0, count: N)
   var v = Array(repeating: 0.0, count: N)
   for _ in 0..<10
   {
      eval_AtA_times_u(N,u,&v)
      eval_AtA_times_u(N,v,&u)      
   }
   var vBv=0.0, vv=0.0   
   for i in 0..<N { vBv+=u[i]*v[i]; vv+=v[i]*v[i] }
   print( String(format: "%.9f", sqrt(vBv/vv)))    
}

main( 
   (CommandLine.argc > 1) 
      ? Int(CommandLine.arguments[1])! 
      : 100 )
