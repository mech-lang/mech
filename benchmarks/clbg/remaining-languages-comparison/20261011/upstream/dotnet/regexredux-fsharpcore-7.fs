// The Computer Language Benchmarks Game
// https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
//
// regex-dna program contributed by David Grenier
// converted from regex-dna program
// parallelization by Peter Kese
// Use PCRE.NET by Anthony Lloyd

open PCRE

let input = stdin.ReadToEnd()
let inline regex s = PcreRegex(s,PcreOptions.Compiled|||PcreOptions.NoUtfCheck)
let text = (regex ">.*\n|\n").Replace(input, "")

let settings = PcreMatchSettings(AdditionalOptions=PcreMatchOptions.NoUtfCheck)
let inline regexCount pattern = async {
    let c = (regex pattern).Matches(text,settings) |> Seq.length
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
    |> Async.Parallel
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
