/* The Computer Language Benchmarks Game
 * https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
 *
 * regex-dna program contributed by The Go Authors.
 * converted from regex-dna program
 */

package main

import (
   "fmt"
   "io/ioutil"
   "os"
   "runtime"
   "regexp"
)

var variants = []string{
   "agggtaaa|tttaccct",
   "[cgt]gggtaaa|tttaccc[acg]",
   "a[act]ggtaaa|tttacc[agt]t",
   "ag[act]gtaaa|tttac[agt]ct",
   "agg[act]taaa|ttta[agt]cct",
   "aggg[acg]aaa|ttt[cgt]ccct",
   "agggt[cgt]aa|tt[acg]accct",
   "agggta[cgt]a|t[acg]taccct",
   "agggtaa[cgt]|[acg]ttaccct",
}

type Subst struct {
   pat, repl string
}

var substs = []Subst{
   Subst{"tHa[Nt]", "<4>"},
   Subst{"aND|caN|Ha[DS]|WaS", "<3>"},
   Subst{"a[NSt]|BY", "<2>"},
   Subst{"<[^>]*>", "|"},
   Subst{"\\|[^|][^|]*\\|", "-"},
}

func countMatches(pat string, bytes []byte) int {
   re := regexp.MustCompile(pat)
   n := 0
   for {
      e := re.FindIndex(bytes)
      if e == nil {
         break
      }
      n++
      bytes = bytes[e[1]:]
   }
   return n
}

func main() {
   runtime.GOMAXPROCS(4)
   bytes, err := ioutil.ReadFile("/dev/stdin")
   if err != nil {
      fmt.Fprintf(os.Stderr, "can't read input: %s\n", err)
      os.Exit(2)
   }
   ilen := len(bytes)
   // Delete the comment lines and newlines
   bytes = regexp.MustCompile("(>[^\n]+)?\n").ReplaceAll(bytes, []byte{})
   clen := len(bytes)

   mresults := make([]chan int, len(variants))
   for i, s := range variants {
      ch := make(chan int)
      mresults[i] = ch
      go func(ss string) {
         ch <- countMatches(ss, bytes)
      }(s)
   }

   lenresult := make(chan int)
   bb := bytes
   go func() {
      for _, sub := range substs {
         bb = regexp.MustCompile(sub.pat).ReplaceAll(bb, []byte(sub.repl))
      }
      lenresult <- len(bb)
   }()

   for i, s := range variants {
      fmt.Printf("%s %d\n", s, <-mresults[i])
   }
   fmt.Printf("\n%d\n%d\n%d\n", ilen, clen, <-lenresult)
}
