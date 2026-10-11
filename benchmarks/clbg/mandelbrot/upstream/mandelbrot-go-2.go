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
)

/* targeting a q6600 system, two cpu workers per core */
const pool = 8
const log2pool = 3

const LIMIT = 2.0
const ITER = 50 // Benchmark parameter
const SIZE = 16000

var bytesPerRow int

// This func is responsible for rendering a row of pixels,
// and when complete writing it out to the file.

func renderRow(w, h, y0, maxiter int, wg *sync.WaitGroup, fieldChan chan<- []byte) {

   var Zr, Zi, Tr, Ti, Cr float64
   var x, i int

   //All fields have at least floor( h / pool ) rows
   //numRows := h / pool //Uncomment if pool is not a power of 2
   numRows := h >> log2pool //Comment out if pool is not a power of 2

   //Add one more row if this renderer needs to cover the extra row
   /*if y0 < h % pool { //Uncomment if pool is not a power of 2
      numRows++
   }*/
   if y0 < h&int(pool-1) { //Comment out if pool is not a power of 2
      numRows++
   }

   field := make([]byte, numRows*bytesPerRow)

   for y := 0; y < numRows; y++ {

      offset := bytesPerRow * y
      //uncomment if pool is not a power of 2
      //Ci := (float64((y * pool + y0) << 1)/float64(h) - 1.0)
      //comment out if pool is not a power of 2
      Ci := (float64((y<<log2pool+y0)<<1)/float64(h) - 1.0)

      for x = 0; x < w; x++ {
         Zr, Zi, Tr, Ti = 0, 0, 0, 0
         Cr = (float64(x<<1)/float64(w) - 1.5)

         for i = 0; i < maxiter && Tr+Ti <= LIMIT*LIMIT; i++ {
            Zr, Zi = Tr-Ti+Cr, 2*Zr*Zi+Ci
            Tr, Ti = Zr*Zr, Zi*Zi
         }

         // Store the value in the array of ints
         if Tr+Ti <= LIMIT*LIMIT {
            field[offset+(x>>3)] |= (byte(1) << uint(7-(x&int(7))))
         }
      }
   }
   //Signal finish
   wg.Done()
   fieldChan <- field
}

func main() {
   runtime.GOMAXPROCS(pool)

   size := SIZE // Contest settings
   maxiter := ITER

   // Get input, if any...
   flag.Parse()
   if flag.NArg() > 0 {
      size, _ = strconv.Atoi(flag.Arg(0))
   }
   w, h := size, size
   bytesPerRow = w / 8

   out := bufio.NewWriter(os.Stdout)
   defer out.Flush()
   fmt.Fprintf(out, "P4\n%d %d\n", w, h)

   fieldChans := make([]chan []byte, pool)

   /* Wait group for finish */
   wg := new(sync.WaitGroup)
   // start pool workers, and assign all work
   for y := 0; y < pool; y++ {
      wg.Add(1)
      fc := make(chan []byte)
      fieldChans[y] = fc
      go renderRow(w, h, y, maxiter, wg, fc)
   }

   fields := make([][]byte, pool)

   /* wait for the file workers to finish, then write */
   wg.Wait()
   for y := 0; y < pool; y++ {
      fields[y] = <-fieldChans[y]
   }

   //Interlace the fields for write out
   var rowEnd int
   for rowStart := 0; rowStart < len(fields[0]); rowStart = rowEnd {
      rowEnd = rowStart + bytesPerRow
      for fieldNum := 0; fieldNum < pool &&
         rowStart < len(fields[fieldNum]); fieldNum++ {
         out.Write(fields[fieldNum][rowStart:rowEnd])
      }
   }
}
