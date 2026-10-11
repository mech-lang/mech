// The Computer Language Benchmarks Game
// https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
//
// regex-dna program modified version of Valentin Kraevskiy
// contributed by Vassil Keremidchiev
// converted from regex-dna program

open System.Text.RegularExpressions
open System.Threading

let regex s = Regex (s, RegexOptions.Compiled)
let input = stdin.ReadToEnd ()
let withoutComments = (regex ">.*\n").Replace (input, "")
let text = (regex "\n").Replace (withoutComments, "")

let rec onblocks res s =
    let size = 1024*4096
    match s with
    | "" -> res
    | s when (s.Length < size) -> res @ [s]
    | s -> onblocks (res @ [s.Substring(0, size)]) (s.Substring(size)) 

["agggtaaa|tttaccct"
 "[cgt]gggtaaa|tttaccc[acg]"
 "a[act]ggtaaa|tttacc[agt]t"
 "ag[act]gtaaa|tttac[agt]ct"
 "agg[act]taaa|ttta[agt]cct"
 "aggg[acg]aaa|ttt[cgt]ccct"
 "agggt[cgt]aa|tt[acg]accct"
 "agggta[cgt]a|t[acg]taccct"
 "agggtaa[cgt]|[acg]ttaccct"]
|> List.map (fun s -> async { 
        return System.String.Format( "{0} {1}", s, 
                                        ((regex s).Matches text).Count) } ) 
|> Async.Parallel |> Async.RunSynchronously 
|> Array.iter (printfn "%s")

let newTextLength t =
    ["tHa[Nt]", "<4>"
     "aND|caN|Ha[DS]|WaS", "<3>"
     "a[NSt]|BY", "<2>"
     "<[^>]*>", "|"
     "\\|[^|][^|]*\\|" , "-"]
     |> List.fold (fun s (code, alt) -> (regex code).Replace (s, alt)) t 
     |> String.length

let newText = 
    text |> onblocks [] 
    |> Seq.map (fun s -> async { return newTextLength s } ) 
    |> Async.Parallel |> Async.RunSynchronously
    |> Array.sum

printf "\n%i\n%i\n%i\n" input.Length text.Length newText
