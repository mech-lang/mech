(* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   Contributed by Don Syme
   Port of C# version by by Marek Safar and optimized by kasthack
   *reset*
*)

open System

[<AllowNullLiteral>]
type TreeNode(left:TreeNode,right:TreeNode) = 
    member __.CheckSum =
        match right with 
        | null -> 1 
        | _ -> 1 + left.CheckSum + right.CheckSum

let rec mkTree(depth) =
    if depth = 0 then TreeNode(null, null)
    else TreeNode(mkTree (depth-1), mkTree(depth-1))

let bottomUpTree (depth) = mkTree(depth)

let minDepth = 4
[<EntryPoint>]
let main argv = 
    let n = if argv.Length > 0 then Int32.Parse(argv.[0]) else 0
    let maxDepth = Math.Max(minDepth + 2, n)
    let stretchDepth = maxDepth + 1
    let mutable check = bottomUpTree(stretchDepth).CheckSum
    Console.WriteLine("stretch tree of depth {0}\t check: {1}", stretchDepth, check)
    let longLivedTree = bottomUpTree(maxDepth)
    for depth in minDepth .. 2 .. maxDepth do
         let iterations = 1 <<< ( maxDepth - depth + minDepth )
         check <- 0
         for i in 1 .. iterations do 
            check <- check + bottomUpTree(depth).CheckSum
         Console.WriteLine("{0}\t trees of depth {1}\t check: {2}",iterations, depth, check)
    Console.WriteLine("long lived tree of depth {0}\t check: {1}",maxDepth, longLivedTree.CheckSum)
    0
