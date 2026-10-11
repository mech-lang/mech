// The Computer Language Benchmarks Game
// https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
//
// Based on Java version by Jarkko Miettinen's
// Contributed by Vasily Kirichenko

[<Sealed; AllowNullLiteral>]
type TreeNode(left: TreeNode, right: TreeNode) =
    member this.itemCheck() = 
        if isNull left then 1 else 1 + left.itemCheck() + right.itemCheck()


let rec bottomUpTree(depth: int) : TreeNode =
    if depth > 0 then
        TreeNode(bottomUpTree(depth - 1), bottomUpTree(depth - 1))
    else
        TreeNode(null, null)

[<EntryPoint>]
let main args =
    let n = match args with [|n|] -> int n | _ -> 0
    let minDepth = 4
    let maxDepth = max (minDepth + 2) n
    let stretchDepth = maxDepth + 1

    printfn "stretch tree of depth %d\t check: %d" stretchDepth (bottomUpTree(stretchDepth).itemCheck())

    let longLivedTree = bottomUpTree maxDepth

    [| for depth in minDepth..2..maxDepth do
         yield async {
            let iterations = 1 <<< (maxDepth - depth + minDepth)
            let check = Array.init iterations (fun _ -> bottomUpTree(depth).itemCheck()) |> Array.sum
            return sprintf "%d\t trees of depth %d\t check: %d" iterations depth check
         } |]
    |> Async.Parallel
    |> Async.RunSynchronously
    |> Array.iter (printfn "%s")

    printfn "long lived tree of depth %d\t check: %d" maxDepth (longLivedTree.itemCheck())
    0
