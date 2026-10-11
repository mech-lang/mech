/* The Computer Language Benchmarks Game
 * https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
 *
 * Contributed by Tyler Kropp,
 * based on the C program by Jeremy Zerfas (C gcc #2),
 * which was based on the C++ program from
 * Jon Harrop, Alex Mizrahi, and Bruno Coutinho.
 */

package main

import (
    "fmt"
    "os"
    "strconv"
    "sync"
)

// Node is a binary tree node.
type Node struct {
    Left  *Node
    Right *Node
}

// CreateTree returns the root of a new binary tree with the given depth.
func CreateTree(depth int) *Node {
    if depth <= 0 {
        return &Node{}
    }
    return &Node{
        Left:  CreateTree(depth - 1),
        Right: CreateTree(depth - 1),
    }
}

// CalculateChecksum returns the total number of nodes in the binary tree.
func (n Node) CalculateChecksum() int {
    if n.Left == nil {
        return 1
    }
    return n.Left.CalculateChecksum() + n.Right.CalculateChecksum() + 1
}

// LongLivedTree is an exported variable used to prevent the long lived tree
// from being garbage collected.
var LongLivedTree *Node

func main() {
    // Set minDepth to 4 and maxDepth to the maximum of what was specified as
    // the argument to the program and minDepth+2.
    const minDepth = 4

    if len(os.Args) < 2 {
        panic("not enough arguments")
    }

    maxDepth, err := strconv.Atoi(os.Args[1])
    if err != nil {
        panic("could not parse first argument")
    }

    if maxDepth < minDepth+2 {
        maxDepth = minDepth + 2
    }

    // Create a stretch tree of depth maxDepth+1, compute its checksum, and
    // print its statistics.
    stretch := CreateTree(maxDepth + 1)
    check := stretch.CalculateChecksum()
    fmt.Printf("stretch tree of depth %v\t check: %v\n", maxDepth+1, check)

    // The LongLivedTree will be created while the rest of the trees are also
    // being processed. LongLivedTree will remain valid until the end of the
    // program, because it is exported from the package.
    longLivedTreeChecksum := make(chan int, 1)
    go func() {
        // Have one thread create the LongLivedTree of depth maxDepth, compute
        // the longLivedTreeChecksum, and then just leave the LongLivedTree
        // alone for a while while the rest of the binary trees finish
        // processing (which should have simultaneously been started to be
        // processed by any other available threads).
        LongLivedTree = CreateTree(maxDepth)
        longLivedTreeChecksum <- LongLivedTree.CalculateChecksum()
    }()

    // These will be used to store outputs for the various trees so the
    // statistics can be printed in the correct order later.
    outputBuffer := make([]string, (maxDepth-minDepth)/2+1)
    type depthOutput struct {
        depth int
        s     string
    }

    out := make(chan depthOutput, (maxDepth-minDepth)/2+1)
    done := make(chan struct{})
    wg := &sync.WaitGroup{}

    go func() {
        for o := range out {
            // Record the output for the trees at the given depth.
            i := (o.depth - minDepth) / 2
            outputBuffer[i] = o.s
        }
        done <- struct{}{}
    }()

    for depth := minDepth; depth <= maxDepth; depth += 2 {
        wg.Add(1)
        go func(depth int) {
            iterations := 1 << (maxDepth - depth + minDepth)
            totalChecksum := 0

            for i := 1; i <= iterations; i++ {
                // Create a bunch of binary trees of depth depth, compute each
                // of their checksums, and add them to the totalChecksum.
                tree := CreateTree(depth)
                totalChecksum += tree.CalculateChecksum()
            }

            out <- depthOutput{
                depth,
                fmt.Sprintf("%v\t trees of depth %v\t check: %v",
                    iterations, depth, totalChecksum),
            }
            wg.Done()
        }(depth)
    }

    wg.Wait()
    close(out)
    <-done

    // Print the statistics for all of the various tree depths.
    for _, s := range outputBuffer {
        fmt.Println(s)
    }

    // Print the statistics for the LongLivedTree that was processed earlier.
    // Note that although the LongLivedTree variable isn't used here, it still
    // is in scope and valid to use until the end of the program.
    fmt.Printf("long lived tree of depth %v\t check: %v\n",
        maxDepth, <-longLivedTreeChecksum)
}
