/* The Computer Language Benchmarks Game
 * https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
 *
 * contributed by Anthony Perez-Sanz.
 * based on Java program by Jarkko Miettinen
 * *reset*
 */

package main

import (
   "flag"
   "fmt"
   "strconv"
)

type Node struct {
   left, right *Node
}

const minDepth = 4

func trees(maxDepth int) {
   longLastingNode := createTree(maxDepth)
   depth := 4

   for depth <= maxDepth {
      iterations := 1 << uint(maxDepth-depth+minDepth) // 16 << (maxDepth - depth)

      loops(iterations, depth)
      depth += 2
   }
   fmt.Printf("long lived tree of depth %d\t check: %d\n", maxDepth,
      checkTree(longLastingNode))
}

func loops(iterations, depth int) {
   check := 0
   item := 0
   for item < iterations {
      check += checkTree(createTree(depth))
      item++
   }
   fmt.Printf("%d\t trees of depth %d\t check: %d\n",
      iterations, depth, check)
}

func checkTree(n *Node) int {
   if n.left == nil {
      return 1
   }
   return checkTree(n.left) + checkTree(n.right) + 1
}

func createTree(depth int) *Node {
   node := &Node{}
   if depth > 0 {
      depth--
      node.left = createTree(depth)
      node.right = createTree(depth)
   }
   return node
}

func main() {
   n := 0
   flag.Parse()
   if flag.NArg() > 0 {
      n, _ = strconv.Atoi(flag.Arg(0))
   }

   maxDepth := n
   if minDepth+2 > n {
      maxDepth = minDepth + 2
   }

   {
      stretchDepth := maxDepth + 1
      check := checkTree(createTree(stretchDepth))
      fmt.Printf("stretch tree of depth %d\t check: %d\n", stretchDepth, check)
   }
   trees(maxDepth)
}
