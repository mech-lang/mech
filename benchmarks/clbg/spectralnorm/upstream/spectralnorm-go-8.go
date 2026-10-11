/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   Naive transliteration from Sebastien Loisel's C program
   contributed by Isaac Gouy
*/

package main

import (
   "flag"
   "fmt"
   "math"   
   "strconv"   
)   

func eval_A(i, j int) float64 { return 1.0/ float64(((i+j)*(i+j+1)/2+i+1)) }

type Vec []float64

func eval_A_times_u(N int, u Vec, Au Vec) {
    for i := 0; i < N; i++ {
        Au[i]=0
        for j := 0; j < N; j++ { 
            Au[i] += float64(eval_A(i,j)) * u[j]             
        } 
    }
}

func eval_At_times_u(N int, u Vec, Au Vec) {
    for i := 0; i < N; i++ {
        Au[i]=0
        for j := 0; j < N; j++ { 
            Au[i] += float64(eval_A(j,i)) * u[j]   
        }
    }
}

func eval_AtA_times_u(N int, u, AtAu Vec) {
    v := make(Vec, N); eval_A_times_u(N,u,v); eval_At_times_u(N,v,AtAu) 
}

func main() {
    N := 100
    flag.Parse()
    if flag.NArg() > 0 { N, _ = strconv.Atoi(flag.Arg(0)) }

    u := make(Vec, N)
    for i := 0; i < N; i++ { u[i] = 1 }
    v := make(Vec, N)
    var vBv, vv float64    
    for i := 0; i < 10; i++ {    
        eval_AtA_times_u(N,u,v)
        eval_AtA_times_u(N,v,u)         
    }
    
    vBv = 0; vv = 0   
    for i := 0; i < N; i++ { vBv += u[i]*v[i]; vv += v[i]*v[i] }   
    fmt.Printf("%0.9f\n", math.Sqrt(vBv/vv))         
}
