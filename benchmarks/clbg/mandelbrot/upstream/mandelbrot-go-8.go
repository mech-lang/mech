/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
 *
 * contributed by The Go Authors.
 * Based on mandelbrot.c contributed by Greg Buchholz
*/

package main

import (
   "bufio"
   "flag"
   "fmt"
   "os"
   "strconv"
)

var n = 0

func main() {
   flag.Parse()
   if flag.NArg() > 0 { n,_ = strconv.Atoi( flag.Arg(0) ) }

   out := bufio.NewWriter(os.Stdout)
   defer out.Flush()

   w := n
   h := n
   bit_num := 0
   byte_acc := byte(0)
   const Iter = 50
   const Zero float64 = 0
   const Limit = 2.0

   fmt.Fprintf(out, "P4\n%d %d\n", w, h)

   for y := 0; y < h; y++ {
      for x := 0; x<w; x++ {
         Zr, Zi, Tr, Ti := Zero, Zero, Zero, Zero
         Cr := (2*float64(x)/float64(w) - 1.5)
         Ci := (2*float64(y)/float64(h) - 1.0)

         for i := 0; i < Iter && (Tr+Ti <= Limit*Limit); i++ {
            Zi = 2*Zr*Zi + Ci
            Zr = Tr - Ti + Cr
            Tr = Zr * Zr
            Ti = Zi * Zi
         }

         byte_acc <<= 1
         if Tr+Ti <= Limit*Limit {
            byte_acc |= 0x01
         }

         bit_num++

         if bit_num == 8 {
            out.WriteByte(byte_acc)
            byte_acc = 0
            bit_num = 0
         } else if x == w-1 {
            byte_acc <<= uint(8-w%8)
            out.WriteByte(byte_acc)
            byte_acc = 0
            bit_num = 0
         }
      }
   }
}
