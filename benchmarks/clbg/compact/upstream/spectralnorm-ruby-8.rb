# The Computer Language Benchmarks Game
#   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
#
#   Naive transliteration from Sebastien Loisel's C program
#   contributed by Isaac Gouy
#

def eval_A(i, j) return 1.0/((i+j)*(i+j+1)/2+i+1) end

def eval_A_times_u(n, u, au)
   for i in 0...n  
      au[i]=0
      for j in 0...n do au[i]+=eval_A(i,j)*u[j] end
   end
end

def eval_At_times_u(n, u, au)
   for i in 0...n 
      au[i]=0
      for j in 0...n do au[i]+=eval_A(j,i)*u[j] end
   end      
end

def eval_AtA_times_u(n, u, atAu)
   v=[0]*n; eval_A_times_u(n,u,v); eval_At_times_u(n,v,atAu)
end

def main(n)
   u=[1]*n
   v=[0]*n    
   for i in 0...10 
      eval_AtA_times_u(n,u,v);
      eval_AtA_times_u(n,v,u);   
   end
   vBv=vv=0
   for i in 0...n do vBv+=u[i]*v[i]; vv+=v[i]*v[i] end
   puts "%.9f" % Math.sqrt(vBv/vv)    
end  

n = (ARGV[0] || 100).to_i 
main n
