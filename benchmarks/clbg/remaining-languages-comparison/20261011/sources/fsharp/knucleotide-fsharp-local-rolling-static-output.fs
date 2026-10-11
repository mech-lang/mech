// Locally authored full-work rolling-key implementation with seven ordinary maps.
open System
open System.Text
open System.Collections.Generic
open System.Threading.Tasks
let sequence = StringBuilder()
let mutable collecting = false
let mutable finished = false
let mutable line = Console.ReadLine()
while line <> null && not finished do
    if line.StartsWith(">") then
        if collecting then finished <- true
        elif line.StartsWith(">THREE") then collecting <- true
    elif collecting then sequence.Append(line.Trim()) |> ignore
    line <- Console.ReadLine()
let dna = Encoding.ASCII.GetBytes(sequence.ToString().ToUpperInvariant())
let code (b:byte) = uint64 ((int b &&& 6) >>> 1)
let pack (s:string) = Seq.fold (fun k c -> (k <<< 2) ||| code(byte c)) 0UL s
let decode key width =
    String(Array.init width (fun i -> "ACTG".[int((key >>> (2*(width-i-1))) &&& 3UL)]))
let counts width =
    let table = Dictionary<uint64,int>()
    let mask = (1UL <<< (2*width))-1UL
    let mutable key = 0UL
    for i in 0 .. dna.Length-1 do
        key <- ((key <<< 2) ||| code dna.[i]) &&& mask
        if i >= width-1 then
            let mutable previous = 0
            table.TryGetValue(key,&previous) |> ignore
            table.[key] <- previous+1
    table
let widths = [|1;2;3;4;6;12;18|]
let jobs = widths |> Array.map (fun width -> Task.Run(fun () -> counts width))
let tables = jobs |> Array.map (fun job -> job.Result)
for i in 0 .. 1 do
    let width = widths.[i]
    tables.[i] |> Seq.sortBy (fun p -> (-p.Value,decode p.Key width))
               |> Seq.iter (fun p -> System.Console.WriteLine("{0} {1:F3}", decode p.Key width, 100.0*float p.Value/float(dna.Length-width+1)))
    printfn ""
let queries = [|"GGT";"GGTA";"GGTATT";"GGTATTTTAATT";"GGTATTTTAATTTATAGT"|]
for i in 0 .. queries.Length-1 do
    let mutable total = 0
    tables.[i+2].TryGetValue(pack queries.[i],&total) |> ignore
    System.Console.WriteLine("{0}\t{1}", total, queries.[i])
