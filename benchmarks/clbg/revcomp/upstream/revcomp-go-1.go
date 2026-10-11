/* The Computer Language Benchmarks Game
 * https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
 *
 * contributed by Krzysztof Kowalczyk
 */

package main

import (
   "bytes"
   "fmt"
   "io"
   "log"
   "os"
   "runtime"
)

var comptbl = [256]uint8{}

func build_comptbl() {
   l1 := []byte("UACBDKRWSN")
   l2 := []byte("ATGVHMYWSN")
   l1_lower := bytes.ToLower(l1)
   l2_lower := bytes.ToLower(l2)
   for i, c1 := range l1 {
      c2 := l2[i]
      comptbl[c1] = c2
      comptbl[c2] = c1
      comptbl[l1_lower[i]] = c2
      comptbl[l2_lower[i]] = c1
   }
   comptbl['\n'] = '\n'
}

const CHUNK_SIZE = 1024 * 128
const BUF_SIZE = 1024 * 1024 * 243
const (
   START   = 0
   IN_HDR  = 1
   IN_DATA = 2
)

type ChunkInfo struct {
   i, end int
   last   bool
}

type PrintJob struct {
   data []byte
   last bool
}

var buf []byte
var start_fasta_hdr int
var start_fasta_data int
var state int = START
var chunker_chan chan ChunkInfo
var printer_chan chan PrintJob
var done_chan chan bool

func printer(jobs chan PrintJob, done chan bool) {
   for {
      job := <-jobs
      if nil != job.data {
         os.Stdout.Write(job.data)
      }
      if job.last {
         done <- true
         return
      }
   }
}

func add_printer_job(data []byte, last bool) {
   var j PrintJob
   j.data = data
   j.last = last
   printer_chan <- j
}

// TODO: this one can print a bit earlier
func reverse(strand []byte) {
   i := 0
   end := len(strand) - 1
   for i < end {
      c := strand[i]
      if c == '\n' {
         i += 1
         c = strand[i]
      }
      cend := strand[end]
      if cend == '\n' {
         end -= 1
         cend = strand[end]
      }
      strand[i] = comptbl[cend]
      strand[end] = comptbl[c]
      i += 1
      end -= 1
   }
}

func print_fasta(start, end int) {
   reverse(buf[start:end])
   add_printer_job(buf[start:end], false)
}

func process_chunk_data(i, end int) int {
   pos := bytes.IndexByte(buf[i:end], '>')
   if -1 == pos {
      i = end
      return IN_DATA
   }
   i = i + pos
   print_fasta(start_fasta_data, i)
   start_fasta_hdr = i
   return process_chunk_start(i, end)
}

func process_chunk_hdr(i, end int) int {
   for ; i < end; i++ {
      if buf[i] == '\n' {
         add_printer_job(buf[start_fasta_hdr:i+1], false)
         start_fasta_data = i + 1
         return process_chunk_data(i+1, end)
      }
   }
   return IN_HDR
}

func process_chunk_start(i, end int) int {
   if buf[i] != '>' {
      panic("Unexpected")
   }
   start_fasta_hdr = i
   return process_chunk_hdr(i+1, end)
}

func process_chunk(i, end int) {
   if state == START {
      state = process_chunk_start(i, end)
   } else if state == IN_HDR {
      state = process_chunk_hdr(i, end)
   } else {
      state = process_chunk_data(i, end)
   }
}

func chunker(chunks chan ChunkInfo, done chan bool) {
   for {
      ci := <-chunks
      if ci.last {
         if state != IN_DATA {
            panic("unexpected state")
         }
         print_fasta(start_fasta_data, ci.end)
         add_printer_job(nil, true)
         done <- true
         return
      } else {
         process_chunk(ci.i, ci.end)
      }
   }
}

func main() {
   runtime.GOMAXPROCS(4)
   build_comptbl()
   buf = make([]byte, BUF_SIZE, BUF_SIZE)
   chunker_chan = make(chan ChunkInfo, 128)
   printer_chan = make(chan PrintJob, 16)
   done_chan = make(chan bool, 2)
   go chunker(chunker_chan, done_chan)
   go printer(printer_chan, done_chan)
   i := 0
   for {
      n, err := os.Stdin.Read(buf[i : i+CHUNK_SIZE])
      if n > 0 {
         var ci ChunkInfo
         ci.i = i
         ci.end = i + n
         ci.last = false
         chunker_chan <- ci
         i += n
      } else {
         if err == io.EOF {
            var ci ChunkInfo
            ci.end = i
            ci.last = true
            chunker_chan <- ci
            break
         }
         log.Fatalf(fmt.Sprintf("Unexpected error: %s\n"), err.Error())
      }
   }
   // wait for both chunker and printer
   <-done_chan
   <-done_chan
}
