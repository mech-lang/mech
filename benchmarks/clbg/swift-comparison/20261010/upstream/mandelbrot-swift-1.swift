/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   direct transliteration of Greg Buchholz's C program
   contributed by Isaac Gouy, fix by David Turnbull
*/

import Glibc

let w: Int = Int(CommandLine.arguments[1])!
let h = w

var bit_num = 0, i = 0, byte_acc: Int32 = 0
let iter = 50, limit = 2.0
var Zr, Zi, Cr, Ci, Tr, Ti: Double

print("P4\n\(w) \(h)")

for y in 0..<h {
   for x in 0..<w {

      Zr = 0.0; Zi = 0.0; Tr = 0.0; Ti = 0.0
      Cr = 2.0*Double(x)/Double(w) - 1.5; 
      Ci = 2.0*Double(y)/Double(h) - 1.0

      i = 0
      while i < iter && (Tr+Ti <= limit*limit) {
         i += 1
         Zi = 2.0*Zr*Zi + Ci
         Zr = Tr - Ti + Cr
         Tr = Zr * Zr
         Ti = Zi * Zi
      }

      byte_acc <<= 1
      if Tr+Ti <= limit*limit { byte_acc |= 0x01 }

      bit_num += 1

      if bit_num == 8 {
         putc(byte_acc,stdout) // Glibc
         byte_acc = 0
         bit_num = 0
      }
      else if x == w-1 {
         byte_acc <<= (8-w%8)
         putc(byte_acc,stdout) // Glibc
         byte_acc = 0
         bit_num = 0
      }
   }
}
