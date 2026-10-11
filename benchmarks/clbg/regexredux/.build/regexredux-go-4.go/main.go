/* The Computer Language Benchmarks Game
 * https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
 *
 * regex-dna program contributed by The Go Authors.
 * modified by Tylor Arndt.
 * modified by Chandra Sekar S to use optimized PCRE binding.
 * modified by Matt Dellandrea.
 * converted from regex-dna program
 */
/* The Computer Language Benchmarks Game
 * https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
 *
 * regex-dna program contributed by The Go Authors.
 * modified by Tylor Arndt.
 * modified by Chandra Sekar S to use optimized PCRE binding.
 * modified by Matt Dellandrea.
 * converted from regex-dna program
 */
package main

import (
    "fmt"
    "io/ioutil"
    "os"
    "runtime"

    "github.com/mdellandrea/golang-pkg-pcre/src/pkg/pcre"
)

type Subst struct {
    pat, repl string
}

var (
    variants = [9]string{
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
    substs = [5]Subst{
        {"tHa[Nt]", "<4>"},
        {"aND|caN|Ha[DS]|WaS", "<3>"},
        {"a[NSt]|BY", "<2>"},
        {"<[^>]*>", "|"},
        {"\\|[^|][^|]*\\|", "-"},
    }
)

func countMatches(index int, bytes []byte) int {
    m := pcre.MustCompile(variants[index], 0).Matcher(bytes, 0)
    var n, a int
    for {
        a = m.RIndex(bytes, 0)
        if a < 0 {
            break
        }
        n++
        bytes = bytes[a:]
    }
    return n
}

func pProcess(index int, bytes []byte) chan int {
    ch := make(chan int)
    go func() {
        ch <- countMatches(index, bytes)
    }()
    return ch
}

func main() {
    runtime.GOMAXPROCS(runtime.NumCPU())
    bytes, err := ioutil.ReadAll(os.Stdin)
    if err != nil {
        fmt.Fprintf(os.Stderr, "can't read input: %s\n", err)
        os.Exit(2)
    }

    ilen := len(bytes)
    // Delete the comment lines and newlines
    bytes = pcre.MustCompile("(>[^\n]+)?\n", 0).
        ReplaceAll(bytes, []byte{}, 0)
    clen := len(bytes)

    mresults := make([]chan int, 9)
    for i := 0; i < len(variants); i++ {
        mresults[i] = pProcess(i, bytes)
    }

    lenresult := make(chan int, 1)
    bb := make([]byte, clen)
    copy(bb, bytes)
    go func() {
        for i := 0; i < len(substs); i++ {
            bb = pcre.MustCompile(substs[i].pat, 0).
                ReplaceAll(bb, []byte(substs[i].repl), 0)
        }
        lenresult <- len(bb)
    }()
    for i := 0; i < len(variants); i++ {
        fmt.Printf("%s %d\n", variants[i], <-mresults[i])
    }

    fmt.Printf("\n%d\n%d\n%d\n", ilen, clen, <-lenresult)
}
