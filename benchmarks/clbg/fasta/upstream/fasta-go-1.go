/* The Computer Language Benchmarks Game
 * https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
 *
 * contributed by The Go Authors.
 * Based on C program by Joern Inge Vestgaarden
 * and Jorge Peixoto de Morais Neto.
 * flag.Arg hack by Isaac Gouy
 */

package main

import (
   "bufio"
   "flag"
   "os"
   "strconv"
)

var out *bufio.Writer

var n = 0

const WIDTH = 60 // Fold lines after WIDTH bytes

func min(a, b int) int {
   if a < b {
      return a
   }
   return b
}

type AminoAcid struct {
   p float64
   c byte
}

func AccumulateProbabilities(genelist []AminoAcid) {
   for i := 1; i < len(genelist); i++ {
      genelist[i].p += genelist[i-1].p
   }
}

// RepeatFasta prints the characters of the byte slice s. When it
// reaches the end of the slice, it goes back to the beginning.
// It stops after generating count characters.
// After each WIDTH characters it prints a newline.
// It assumes that WIDTH <= len(s) + 1.
func RepeatFasta(s []byte, count int) {
   pos := 0
   s2 := make([]byte, len(s)+WIDTH)
   copy(s2, s)
   copy(s2[len(s):], s)
   for count > 0 {
      line := min(WIDTH, count)
      out.Write(s2[pos : pos+line])
      out.WriteByte('\n')
      pos += line
      if pos >= len(s) {
         pos -= len(s)
      }
      count -= line
   }
}

var lastrandom uint32 = 42

const (
   IM = 139968
   IA = 3877
   IC = 29573
)

// Each element of genelist is a struct with a character and
// a floating point number p between 0 and 1.
// RandomFasta generates a random float r and
// finds the first element such that p >= r.
// This is a weighted random selection.
// RandomFasta then prints the character of the array element.
// This sequence is repeated count times.
// Between each WIDTH consecutive characters, the function prints a newline.
func RandomFasta(genelist []AminoAcid, count int) {
   buf := make([]byte, WIDTH+1)
   for count > 0 {
      line := min(WIDTH, count)
      for pos := 0; pos < line; pos++ {
         lastrandom = (lastrandom*IA + IC) % IM
         // Integer to float conversions are faster if the integer is signed.
         r := float64(int(lastrandom)) / IM
         for _, v := range genelist {
            if v.p >= r {
               buf[pos] = v.c
               break
            }
         }
      }
      buf[line] = '\n'
      out.Write(buf[0 : line+1])
      count -= line
   }
}

func main() {
   out = bufio.NewWriter(os.Stdout)
   defer out.Flush()

   flag.Parse()
   if flag.NArg() > 0 { n,_ = strconv.Atoi( flag.Arg(0) ) }

   iub := []AminoAcid{
      AminoAcid{0.27, 'a'},
      AminoAcid{0.12, 'c'},
      AminoAcid{0.12, 'g'},
      AminoAcid{0.27, 't'},
      AminoAcid{0.02, 'B'},
      AminoAcid{0.02, 'D'},
      AminoAcid{0.02, 'H'},
      AminoAcid{0.02, 'K'},
      AminoAcid{0.02, 'M'},
      AminoAcid{0.02, 'N'},
      AminoAcid{0.02, 'R'},
      AminoAcid{0.02, 'S'},
      AminoAcid{0.02, 'V'},
      AminoAcid{0.02, 'W'},
      AminoAcid{0.02, 'Y'},
   }

   homosapiens := []AminoAcid{
      AminoAcid{0.3029549426680, 'a'},
      AminoAcid{0.1979883004921, 'c'},
      AminoAcid{0.1975473066391, 'g'},
      AminoAcid{0.3015094502008, 't'},
   }

   AccumulateProbabilities(iub)
   AccumulateProbabilities(homosapiens)

   alu := []byte(
      "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGG" +
         "GAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGA" +
         "CCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAAT" +
         "ACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCA" +
         "GCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGG" +
         "AGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCC" +
         "AGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA")

   out.WriteString(">ONE Homo sapiens alu\n")
   RepeatFasta(alu, 2*n)
   out.WriteString(">TWO IUB ambiguity codes\n")
   RandomFasta(iub, 3*n)
   out.WriteString(">THREE Homo sapiens frequency\n")
   RandomFasta(homosapiens, 5*n)
}
