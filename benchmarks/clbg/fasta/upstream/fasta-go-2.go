/* The Computer Language Benchmarks Game
 * https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
 *
 * contributed by The Go Authors.
 * Based on C program by Joern Inge Vestgaarden
 * and Jorge Peixoto de Morais Neto.
 * flag.Arg hack by Isaac Gouy
 * parallel hack by INADA Naoki
 */

package main

import (
   "bufio"
   "flag"
   "os"
   "runtime"
   "strconv"
)

var out *bufio.Writer

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

const (
   IM = 139968
   IA = 3877
   IC = 29573
)

var lastrandom uint32 = 42

func generateRandom(buf []float64) {
   for i := 0; i < len(buf); i++ {
      lastrandom = (lastrandom*IA + IC) % IM
      buf[i] = float64(lastrandom) / IM
   }
}

// generateDna generates DNA text from random sequence.
// Each element of genelist is a struct with a character and
// a floating point number p between 0 and 1.
// generateDna takes a random float r and
// finds the first element such that p >= r.
// This is a weighted random selection.
func generateDna(genelist []AminoAcid, rb []float64, wb []byte) int {
   count := len(rb)
   i := 0
   o := 0
   for count > 0 {
      line := min(WIDTH, count)
      count -= line
      for j := 0; j < line; j++ {
         r := rb[i]
         for _, v := range genelist {
            if v.p >= r {
               wb[o] = v.c
               break
            }
         }
         i++
         o++
      }
      wb[o] = '\n'
      o++
   }
   return o
}

const (
   RANDOM_BUF_SIZE = WIDTH * 1000
   OUT_BUF_SIZE    = (WIDTH + 1) * 1000

   // 1 for output, 4 for generateDna, 1 for generateRandom and 2 spaces
   SLOT = 8
)

// RandomFasta then prints the character of the array element.
// This sequence is repeated count times.
// Between each WIDTH consecutive characters, the function prints a newline.
func RandomFasta(genelist []AminoAcid, count int) {
   rbufs := make([][]float64, SLOT)
   wbufs := make([][]byte, SLOT)
   for i := 0; i < SLOT; i++ {
      rbufs[i] = make([]float64, RANDOM_BUF_SIZE)
      wbufs[i] = make([]byte, OUT_BUF_SIZE)
   }

   // Use `chan []byte` as future object. och is queue of future.
   och := make(chan chan []byte, 4)
   done := make(chan bool)
   go func() {
      for bc := range och {
         buf := <-bc
         out.Write(buf)
      }
      done <- true
   }()

   for i := 0; count > 0; i++ {
      chunk := min(count, RANDOM_BUF_SIZE)
      count -= chunk
      rb := rbufs[i%SLOT][:chunk]
      wb := wbufs[i%SLOT]
      generateRandom(rb)

      c := make(chan []byte)
      och <- c
      go func(rb []float64, wb []byte, c chan []byte) {
         o := generateDna(genelist, rb, wb)
         c <- wb[:o]
      }(rb, wb, c)
   }
   close(och)
   <-done
}

func main() {
   runtime.GOMAXPROCS(runtime.NumCPU())
   out = bufio.NewWriter(os.Stdout)
   defer out.Flush()

   n := 0
   flag.Parse()
   if flag.NArg() > 0 {
      n, _ = strconv.Atoi(flag.Arg(0))
   }

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
