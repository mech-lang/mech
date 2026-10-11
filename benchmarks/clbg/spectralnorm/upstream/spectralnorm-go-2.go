/* The Computer Language Benchmarks Game
 * https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
 *
 * contributed by chaishushan
 * Based on spectral-norm.go by The Go Authors.
 */

package main

import (
   "flag"
   "fmt"
   "math"
   "runtime"
   "strconv"
   "sync"
)

var NumCPU = runtime.NumCPU()*2
var Num int

func init() {
   flag.Parse()
   if flag.NArg() > 0 {
      Num, _ = strconv.Atoi(flag.Arg(0))
   }
   runtime.GOMAXPROCS(NumCPU)
}

func main() {
   fmt.Printf("%0.9f\n", SpectralNorm(Num))
}

func SpectralNorm(n int) float64 {
   u := make([]float64, n)
   v := make([]float64, n)
   t := make([]float64, n)

   for i := 0; i < n; i++ {
      u[i] = 1
   }
   for i := 0; i < 10; i++ {
      mult_AtAv(v, u, t)
      mult_AtAv(u, v, t)
   }

   dot_uv := Dot(u, v, n)
   dot_vv := Dot(v, v, n)

   return math.Sqrt(dot_uv / dot_vv)
}

func mult_AtAv(v, u, x []float64) {
   mult_Av(x, u)
   mult_Atv(v, x)
}

func mult_Av(v, u []float64) {
   if NumCPU <= 1 {
      ul := len(u)
      for i := 0; i < len(v); i++ {
         var vi float64
         for j := 0; j < ul; j++ {
            vi += u[j] / float64(A(i, j))
         }
         v[i] = vi
      }
   } else {
      var wg sync.WaitGroup
      for k := 0; k < NumCPU; k++ {
         wg.Add(1)
         go func(kk int) {
            size := (len(v) + NumCPU - 1) / NumCPU
            start := size * kk
            end := size * (kk + 1)
            if end > len(v) {
               end = len(v)
            }
            ul := len(u)
            for i := start; i < end; i++ {
               var vi float64
               for j := 0; j < ul; j++ {
                  vi += u[j] / float64(A(i, j))
               }
               v[i] = vi
            }
            wg.Done()
         }(k)
      }
      wg.Wait()
   }
}

func mult_Atv(v, u []float64) {
   if NumCPU <= 1 {
      ul := len(u)
      for i := 0; i < len(v); i++ {
         var vi float64
         for j := 0; j < ul; j++ {
            vi += u[j] / float64(A(j, i))
         }
         v[i] = vi
      }
   } else {
      var wg sync.WaitGroup
      for k := 0; k < NumCPU; k++ {
         wg.Add(1)
         go func(kk int) {
            size := (len(v) + NumCPU - 1) / NumCPU
            start := size * kk
            end := size * (kk + 1)
            if end > len(v) {
               end = len(v)
            }
            ul := len(u)
            for i := start; i < end; i++ {
               var vi float64
               for j := 0; j < ul; j++ {
                  vi += u[j] / float64(A(j, i))
               }
               v[i] = vi
            }
            wg.Done()
         }(k)
      }
      wg.Wait()
   }
}

func Dot(v, u []float64, n int) float64 {
   var sum float64
   for i := 0; i < n; i++ {
      sum += v[i] * u[i]
   }
   return sum
}

func A(i, j int) int {
   return ((i+j)*(i+j+1)/2 + i + 1)
}
