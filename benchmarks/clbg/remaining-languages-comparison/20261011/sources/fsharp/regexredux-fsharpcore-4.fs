// The Computer Language Benchmarks Game
// https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
//
// regex-dna program contributed by David Grenier
// converted from regex-dna program

open System.Text.RegularExpressions
open System.Threading

let inline flip f a b = f b a
let inline curry f a b = f(a, b)

let regex s = Regex (s, RegexOptions.Compiled)
let replace rx = ((regex rx).Replace : string * string -> string) |> curry |> flip
let matchCount rx = (regex rx).Matches >> fun x -> x.Count

let input = stdin.ReadToEnd ()
let text = input |> replace ">.*\n" "" |> replace "\n" ""

module Array =
    module Parallel =
        let loop (count: int) f =
            let count = ref count
            let rec loop f =
                async {
                    match Interlocked.Decrement count with
                    | i when i < 0 -> ()
                    | i -> f i; Async.StartImmediate (loop f)
                }
            Array.init System.Environment.ProcessorCount (fun _ -> loop f)
            |> Async.Parallel
            |> Async.RunSynchronously
            |> ignore


        let init len f =
            let result = Array.zeroCreate len
            loop len (fun i -> result.[i] <- f i)
            result

        let map f arr = init (Array.length arr) (fun i -> f arr.[i])

[|
    "agggtaaa|tttaccct"
    "[cgt]gggtaaa|tttaccc[acg]"
    "a[act]ggtaaa|tttacc[agt]t"
    "ag[act]gtaaa|tttac[agt]ct"
    "agg[act]taaa|ttta[agt]cct"
    "aggg[acg]aaa|ttt[cgt]ccct"
    "agggt[cgt]aa|tt[acg]accct"
    "agggta[cgt]a|t[acg]taccct"
    "agggtaa[cgt]|[acg]ttaccct"
|]
|> Array.Parallel.map (fun s -> sprintf "%s %d" s (matchCount s text))
|> Array.iter (printfn "%s")

let chunkedMap windowCount f arr =
    let len = Array.length arr
    let size = len / (windowCount - 1)
    Array.Parallel.init windowCount (fun i ->
        if i + 1 = windowCount then len % (windowCount - 1) else size
        |> Array.sub arr (i * size)
        |> f
    )

let applyPatterns =
    [
        replace "tHa[Nt]" "<4>"
        replace "aND|caN|Ha[DS]|WaS" "<3>"
        replace "a[NSt]|BY" "<2>"
        replace "<[^>]*>" "|"
        replace "\\|[^|][^|]*\\|" "-"
    ]
    |> List.reduce (>>)

text.ToCharArray()
|> chunkedMap 16 (fun cs ->
    System.String cs
    |> applyPatterns
    |> String.length
)
|> Array.sum
|> printfn "\n%i\n%i\n%i" input.Length text.Length
