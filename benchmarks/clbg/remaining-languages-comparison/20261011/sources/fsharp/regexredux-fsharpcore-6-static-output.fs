// The Computer Language Benchmarks Game
// https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
//
// regex-dna program contributed by David Grenier
// converted from regex-dna program
// parallelization by Peter Kese
// order variants by execution time by Anthony Lloyd

open System.Text.RegularExpressions

let inline regex s = Regex(s, RegexOptions.Compiled)
let input = stdin.ReadToEnd()
let text = (regex ">.*\n|\n").Replace (input, "")

let inline regexCount pattern = async {
    let rec loop c (m:Match) =
        if not m.Success then c
        else loop (c+1) (m.NextMatch())
    let c = loop 0 ((regex pattern).Match text)
    return pattern + " " + string c
}

let replaceTask = async {
    let l =
        [
            "tHa[Nt]", "<4>"
            "aND|caN|Ha[DS]|WaS", "<3>"
            "a[NSt]|BY", "<2>"
            "<[^>]*>", "|"
            "\\|[^|][^|]*\\|", "-"
        ]
        |> List.fold (fun s (pattern, replace) ->
            (regex pattern).Replace (s, replace)) text
        |> String.length |> string
    return "\n" + string input.Length + "\n" + string text.Length + "\n" + l
}

let results =
    [
        replaceTask
        regexCount "[cgt]gggtaaa|tttaccc[acg]"
        regexCount "a[act]ggtaaa|tttacc[agt]t"
        regexCount "agggt[cgt]aa|tt[acg]accct"
        regexCount "aggg[acg]aaa|ttt[cgt]ccct"
        regexCount "ag[act]gtaaa|tttac[agt]ct"
        regexCount "agg[act]taaa|ttta[agt]cct"
        regexCount "agggtaaa|tttaccct"
        regexCount "agggtaa[cgt]|[acg]ttaccct"
        regexCount "agggta[cgt]a|t[acg]taccct"
    ]
    |> fun jobs -> Async.Parallel(jobs, maxDegreeOfParallelism=8)
    |> Async.RunSynchronously

stdout.WriteLine results.[7]
stdout.WriteLine results.[1]
stdout.WriteLine results.[2]
stdout.WriteLine results.[5]
stdout.WriteLine results.[6]
stdout.WriteLine results.[4]
stdout.WriteLine results.[3]
stdout.WriteLine results.[9]
stdout.WriteLine results.[8]
stdout.WriteLine results.[0]
