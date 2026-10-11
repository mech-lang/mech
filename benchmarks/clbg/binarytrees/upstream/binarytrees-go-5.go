/* The Computer Language Benchmarks Game
 * https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
 *
 * contributed by The Go Authors.
 * based on C program by Kevin Carson
 * flag.Arg hack by Isaac Gouy
 * goroutines by Atom
 * *reset*
 */

package main

import (
   "flag"
   "fmt"
   "runtime"
   "strconv"
)

const LOG2_N_CPU = 2
const N_CPU = (1 << LOG2_N_CPU)
const LOG2_WORK_UNIT = 8

type Node struct {
   left, right *Node
}

func bottomUpTree(depth int) *Node {
   if depth <= 0 {
      return &Node{}
   }
   return &Node{bottomUpTree(depth-1), bottomUpTree(depth-1)}
}

func go_bottomUpTree(depth int, goroutine_depth int) *Node {
   if depth <= 0 {
      return &Node{}
   }

   var left, right *Node
   if goroutine_depth <= 0 {
      left = bottomUpTree(depth-1)
      right = bottomUpTree(depth-1)
   } else {
      left_chan := make(chan *Node)
      right_chan := make(chan *Node)
      go func() {
         left_chan <- go_bottomUpTree(depth-1, goroutine_depth-1)
      }()
      go func() {
         right_chan <- go_bottomUpTree(depth-1, goroutine_depth-1)
      }()
      left, right = <-left_chan, <-right_chan
   }

   return &Node{left, right}
}

func Go_bottomUpTree(depth int) *Node {
   // Not enough work per goroutine to amortize goroutine creation
   if depth < LOG2_N_CPU+LOG2_WORK_UNIT {
      return bottomUpTree(depth)
   }

   return go_bottomUpTree(depth, LOG2_N_CPU)
}

func (n *Node) itemCheck() int {
   if n.left == nil {
      return 1
   }
   return 1 + n.left.itemCheck() + n.right.itemCheck()
}

func (n *Node) go_itemCheck(goroutine_depth int) int {
   if n.left == nil {
      return 1
   }

   var left, right int
   if goroutine_depth <= 0 {
      left = n.left.itemCheck()
      right = n.right.itemCheck()
   } else {
      left_chan := make(chan int)
      right_chan := make(chan int)
      go func() {
         left_chan <- n.left.go_itemCheck(goroutine_depth - 1)
      }()
      go func() {
         right_chan <- n.right.go_itemCheck(goroutine_depth - 1)
      }()
      left, right = <-left_chan, <-right_chan
   }
   return 1 + left + right
}

func (n *Node) Go_itemCheck() int {
   return n.go_itemCheck(LOG2_N_CPU)
}

var total_goroutines uint32 = 0

const minDepth = 4

func main() {
   runtime.GOMAXPROCS(N_CPU)

   n := 0
   flag.Parse()
   if flag.NArg() > 0 {
      n, _ = strconv.Atoi(flag.Arg(0))
   }

   maxDepth := n
   if minDepth+2 > n {
      maxDepth = minDepth + 2
   }
   stretchDepth := maxDepth + 1

   {
      check := Go_bottomUpTree(stretchDepth).Go_itemCheck()
      fmt.Printf("stretch tree of depth %d\t check: %d\n", stretchDepth, check)
   }

   longLivedTree := Go_bottomUpTree(maxDepth)

   outputs := make(map[int]chan string)
   control := make(chan byte, N_CPU) // This 'control' also puts a cap on memory usage
   for _depth := minDepth; _depth <= maxDepth; _depth += 2 {
      outputs[_depth] = make(chan string, 1)
      go func(depth int) {
         control <- 0

         iterations := 1 << uint(maxDepth-depth+minDepth)
         check := 0

         // Avoid creating a lot of short-lived goroutines
         if depth <= LOG2_N_CPU+LOG2_WORK_UNIT {
            // No goroutines
            for i := 1; i <= iterations; i++ {
               check += bottomUpTree(depth).itemCheck()
            }
         } else {
            // Use goroutines
            for i := 1; i <= iterations; i++ {
               check += Go_bottomUpTree(depth).Go_itemCheck()
            }
         }
         outputs[depth] <- fmt.Sprintf("%d\t trees of depth %d\t check: %d\n",
            iterations, depth, check)

         <-control
      }(_depth)
   }
   for depth := minDepth; depth <= maxDepth; depth += 2 {
      fmt.Print(<-outputs[depth])
   }

   fmt.Printf("long lived tree of depth %d\t check: %d\n", maxDepth, longLivedTree.Go_itemCheck())
}
