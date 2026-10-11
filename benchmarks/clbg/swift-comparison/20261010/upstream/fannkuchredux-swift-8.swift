/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   Naive transliteration from Rex Kerr's Scala program
   contributed by Isaac Gouy 
*/

func fannkuch(_ n: Int) -> Int {
   var perm = Array(repeating: 0, count: n),
      count = Array(repeating: 0, count: n),      
      perm1 = Array(repeating: 0, count: n)
      
   for j in 0...n-1 { perm1[j] = j }
   var f = 0, i = 0, k = 0, r = 0, flips = 0, nperm = 0, checksum = 0

   r = n
   while r > 0 {
      i = 0
      while r != 1 { count[r-1] = r; r -= 1 }
      while i < n { perm[i] = perm1[i]; i += 1 }

      // Count flips and update max  and checksum
      f = 0
      k = perm[0]
      while k != 0 {
         i = 0
         while 2*i < k {
            let t = perm[i]; perm[i] = perm[k-i]; perm[k-i] = t
            i += 1
         }
         k = perm[0]
         f += 1
      }
      if f>flips { flips = f }
      if (nperm & 0x1) == 0 { checksum += f } else { checksum -= f }

      // Use incremental change to generate another permutation
      var more = true
      while more {
         if r == n {
            print(checksum)
            return flips
         }
         let p0 = perm1[0]
         i = 0
         while i < r {
            let j = i+1
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

let n = (CommandLine.argc > 1) 
   ? Int(CommandLine.arguments[1])! 
   : 7

print("Pfannkuchen(\(n)) = \(fannkuch(n))")
