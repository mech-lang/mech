// Locally authored full-work Benchmark Game implementation.
open System
open System.Collections.Generic
let lines = seq { let mutable line = Console.ReadLine()
                  while line <> null do
                      yield line
                      line <- Console.ReadLine() }
let dna = lines |> Seq.skipWhile (fun s -> not (s.StartsWith(">THREE")))
                |> Seq.skip 1 |> Seq.takeWhile (fun s -> not (s.StartsWith(">")))
                |> Seq.map (fun s -> s.Trim().ToUpperInvariant()) |> String.concat ""
let counts width =
    let result = Dictionary<string,int>()
    for i in 0 .. dna.Length-width do
        let key = dna.Substring(i,width)
        let mutable previous = 0
        result.TryGetValue(key,&previous) |> ignore
        result.[key] <- previous+1
    result
for width in [1;2] do
    counts width |> Seq.sortBy (fun p -> (-p.Value,p.Key))
                 |> Seq.iter (fun p -> printfn "%s %.3f" p.Key (100.0*float p.Value/float(dna.Length-width+1)))
    printfn ""
for query in ["GGT";"GGTA";"GGTATT";"GGTATTTTAATT";"GGTATTTTAATTTATAGT"] do
    let mutable total = 0
    (counts query.Length).TryGetValue(query,&total) |> ignore
    printfn "%d\t%s" total query
