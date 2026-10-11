// The Computer Language Benchmarks Game
// https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
//
// regex-dna program contributed by Valentin Kraevskiy
// converted from regex-dna program

open System.Text.RegularExpressions

let regex s = Regex (s, RegexOptions.Compiled)
let input = stdin.ReadToEnd ()
let text = (regex ">.*\n|\n").Replace (input, "")

["agggtaaa|tttaccct"
 "[cgt]gggtaaa|tttaccc[acg]"
 "a[act]ggtaaa|tttacc[agt]t"
 "ag[act]gtaaa|tttac[agt]ct"
 "agg[act]taaa|ttta[agt]cct"
 "aggg[acg]aaa|ttt[cgt]ccct"
 "agggt[cgt]aa|tt[acg]accct"
 "agggta[cgt]a|t[acg]taccct"
 "agggtaa[cgt]|[acg]ttaccct"]
|> List.iter (fun s ->
         System.Console.Write("{0} {1}\n", s, ((regex s).Matches text).Count))

let newText =
    ["tHa[Nt]", "<4>"
     "aND|caN|Ha[DS]|WaS", "<3>"
     "a[NSt]|BY", "<2>"
     "<[^>]*>", "|"
     "\\|[^|][^|]*\\|" , "-"]
     |> List.fold (fun s (code, alt) -> 
            (regex code).Replace (s, alt)) text

System.Console.Write("\n{0}\n{1}\n{2}\n", input.Length, text.Length, newText.Length)
