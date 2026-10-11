/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   Naive transliteration from Rex Kerr's Scala program
   contributed by Isaac Gouy 
*/

package main

import (
    "flag"
    "fmt"
    "strconv"    
)

func fannkuch(n int) int {
   perm1 := make([]int, n)
   for i := 0; i<n; i++ { perm1[i] = i }   
   perm := make([]int, n)
   count := make([]int, n)      
   f, i, k, r, flips, nperm, checksum := 0, 0, 0, 0, 0, 0, 0
      
   r = n
   for r > 0 {
      i = 0
      for r != 1 { count[r-1] = r; r -= 1 }
      for i < n { perm[i] = perm1[i]; i += 1 }

      // Count flips and update max  and checksum
      f = 0
      k = perm[0]
      for k != 0 {
         i = 0
         for 2*i < k {
            t := perm[i]; perm[i] = perm[k-i]; perm[k-i] = t
            i += 1
         }
         k = perm[0]
         f += 1
      }
      if f>flips { flips = f }
      if (nperm & 0x1) == 0 { checksum += f } else { checksum -= f }

      // Use incremental change to generate another permutation
      var more = true
      for more {
         if r == n {   
            fmt.Println(checksum)             
            return flips
         }
         p0 := perm1[0]
         i = 0
         for i < r {
            j := i+1
            perm1[i] = perm1[j]
            i = j
         }
         perm1[r] = p0

         count[r] -= 1
         if count[r] > 0 { more = false } else { r += 1 }
      }
      nperm += 1
   }
   return flips
}

func main() {
   flag.Parse()
   n := 7   
   if flag.NArg() > 0 { n,_ = strconv.Atoi(flag.Arg(0)) }   
   
   fmt.Printf("Pfannkuchen(%d) = %d\n", n, fannkuch(n))    
}
