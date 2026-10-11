/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   Naive transliteration from Drake Diedrich's C program
   contributed by Isaac Gouy 
*/

package main

import (
    "bufio"
    "flag"
    "os"
    "strconv" 
    "strings"
)

const IM = 139968
const IA = 3877
const IC = 29573
const SEED = 42

var seed = SEED
func fastaRand(max float64) float64 {
   seed = (seed * IA + IC) % IM
   return max * float64(seed) / IM
}

var ALU = 
  "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGG" +
  "GAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGA" +
  "CCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAAT" +
  "ACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCA" +
  "GCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGG" +
  "AGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCC" +
  "AGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA" 

var iub = "acgtBDHKMNRSVWY"
var iubP = []float64 {
   0.27, 0.12, 0.12, 0.27, 
   0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 
}

var homosapiens = "acgt"
var homosapiensP = []float64 {
   0.3029549426680,
   0.1979883004921,
   0.1975473066391,
   0.3015094502008,
}  

const LINELEN = 60

// slowest character-at-a-time output
func repeatFasta(seq string, n int) {
   _len := len(seq)
   i := 0
   var b strings.Builder
   for i=0; i<n; i++ {
      b.WriteString( string(seq[i % _len]) )
      if (i % LINELEN == LINELEN - 1) { 
         b.WriteString("\n")
         out.WriteString(b.String())
         b.Reset()
      }
   }
   if (i % LINELEN != 0) { 
      b.WriteString("\n")
      out.WriteString(b.String()) 
   }
}

func randomFasta(seq string, probability []float64, n int) {
   _len := len(seq)
   i, j := 0, 0
   var b strings.Builder   
   for i=0; i<n; i++ {
      v := fastaRand(1.0)       
      /* slowest idiomatic linear lookup.  Fast if len is short though. */
      for j=0; j<_len-1; j++ {  
         v -= probability[j]
         if (v<0) { break }    
      }
      b.WriteString( string(seq[j]) )      
      if (i % LINELEN == LINELEN - 1) { 
         b.WriteString("\n")
         out.WriteString(b.String())
         b.Reset()
      }    
   }
   if (i % LINELEN != 0) { 
      b.WriteString("\n")
      out.WriteString(b.String()) 
   }
}

var out = bufio.NewWriter(os.Stdout)

func main() {
   flag.Parse()
   n := 1000   
   if flag.NArg() > 0 { n,_ = strconv.Atoi(flag.Arg(0)) } 
   
   out.WriteString(">ONE Homo sapiens alu\n")    
   repeatFasta(ALU, n*2)
   
   out.WriteString(">TWO IUB ambiguity codes\n")    
   randomFasta(iub, iubP, n*3)   
   
   out.WriteString(">THREE Homo sapiens frequency\n")    
   randomFasta(homosapiens, homosapiensP, n*5)    
   
   out.Flush()   
}
