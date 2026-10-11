/* The Computer Language Benchmarks Game
 * https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
 *
 * Contributed by Martin Koistinen
 * Based on mandelbrot.c contributed by Greg Buchholz and The Go Authors
 * flag.Arg hack by Isaac Gouy
 *
 * Large changes by Bill Broadley, including:
 * 1) Switching the one goroutine per line to one per CPU
 * 2) Replacing gorouting calls with channels
 * 3) Handling out of order results in the file writer.

 * modified by Sean Lake
 *
 * modified by Rodrigo Corsi
 * 1)two goroutines per cpu
 * 2)each goroutine generate one line and increment counter (atomic int32)


 * modified by Anton Yuzhaninov
 *
 */

package main

import (
    "bufio"
    "flag"
    "fmt"
    "os"
    "runtime"
    "strconv"
    "sync"
    "sync/atomic"
)

const limit = 4.0        // abs(z) < 2
const maxIter = 50       // number of iterations
const defaultSize = 4000 // bitmap size if not given as command-line argument

var rows [][]byte
var bytesPerRow int
var initial_r []float64
var initial_i []float64

// renderRow returns rendered row of pixels
// pixels packed in one byte for PBM image
func renderRow(y0 *int32) []byte {
    var i, j, x int
    var res, b byte
    var Zr1, Zr2, Zi1, Zi2, Tr1, Tr2, Ti1, Ti2 float64

    row := make([]byte, bytesPerRow)

    for xByte := range row {
        res = 0
        Ci := initial_i[*y0]
        for i = 0; i < 8; i += 2 {
            x = xByte << 3 // * 8
            Cr1 := initial_r[x+i]
            Cr2 := initial_r[x+i+1]

            Zr1 = Cr1
            Zi1 = Ci

            Zr2 = Cr2
            Zi2 = Ci

            b = 0

            for j = 0; j < maxIter; j++ {
                Tr1 = Zr1 * Zr1
                Ti1 = Zi1 * Zi1
                Zi1 = 2*Zr1*Zi1 + Ci
                Zr1 = Tr1 - Ti1 + Cr1

                if Tr1+Ti1 > limit {
                    b |= 2
                    if b == 3 {
                        break
                    }
                }

                Tr2 = Zr2 * Zr2
                Ti2 = Zi2 * Zi2
                Zi2 = 2*Zr2*Zi2 + Ci
                Zr2 = Tr2 - Ti2 + Cr2

                if Tr2+Ti2 > limit {
                    b |= 1
                    if b == 3 {
                        break
                    }
                }
            }
            res = (res << 2) | b
        }
        row[xByte] = ^res
    }
    return row
}

var yAt int32 = -1

func renderRows(wg *sync.WaitGroup, s32 int32) {
    var y int32
    for y = atomic.AddInt32(&yAt, 1); y < s32; y = atomic.AddInt32(&yAt, 1) {
        rows[y] = renderRow(&y)
    }
    wg.Done()
}

func main() {
    pool := runtime.NumCPU() * 2
    runtime.GOMAXPROCS(pool)

    // Get input, if any...
    size := defaultSize
    flag.Parse()
    if flag.NArg() > 0 {
        size, _ = strconv.Atoi(flag.Arg(0))
    }

    bytesPerRow = size >> 3

    // Precompute the initial real and imaginary values for each x and y
    // coordinate in the image.
    initial_r = make([]float64, size)
    initial_i = make([]float64, size)
    inv := 2.0 / float64(size)
    for xy := 0; xy < size; xy++ {
        i := inv * float64(xy)
        initial_r[xy] = i - 1.5
        initial_i[xy] = i - 1.0
    }

    rows = make([][]byte, size)

    /* Wait group for finish */
    wg := new(sync.WaitGroup)
    wg.Add(pool)

    // start pool workers, and assign all work
    for i := 0; i < pool; i++ {
        go renderRows(wg, int32(size))
    }

    /* wait for the file workers to finish, then write */
    wg.Wait()

    out := bufio.NewWriter(os.Stdout)
    defer out.Flush()
    fmt.Fprintf(out, "P4\n%d %d\n", size, size)

    for y := 0; y < size; y++ {
        out.Write(rows[y])
    }
}
