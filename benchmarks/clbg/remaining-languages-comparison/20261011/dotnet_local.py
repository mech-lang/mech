"""Additional F# full-map nucleotide candidates; every application helper counts."""
import json
from pathlib import Path
from dotnet_fetch import D,sha
from dotnet_prepare import build
from metrics import normalize
from probe import save

COMPACT='''// Locally authored full-work Benchmark Game implementation.
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
    printfn "%d\\t%s" total query
'''

OPTIMIZED='''// Locally authored full-work rolling-key implementation with seven ordinary maps.
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
               |> Seq.iter (fun p -> printfn "%s %.3f" (decode p.Key width) (100.0*float p.Value/float(dna.Length-width+1)))
    printfn ""
let queries = [|"GGT";"GGTA";"GGTATT";"GGTATTTTAATT";"GGTATTTTAATTTATAGT"|]
for i in 0 .. queries.Length-1 do
    let mutable total = 0
    tables.[i+2].TryGetValue(pack queries.[i],&total) |> ignore
    printfn "%d\\t%s" total queries.[i]
'''

def main():
    report=json.loads((D/'dotnet_jit_builds.json').read_text())
    by_key={r['key']:r for r in report['programs']}
    for form,text in [('compact',COMPACT),('rolling',OPTIMIZED)]:
        ident='knucleotide-fsharp-local-'+form
        path=D/'sources/fsharp'/(ident+'.fs');path.write_text(text)
        row=dict(id=ident,key='knucleotide:'+ident,benchmark='knucleotide',language='fsharp',
            source=str(path),source_sha256=sha(path),source_url=None,local_adaptation=True,
            adaptations=['Locally authored F# application. All seven widths build complete growable maps; packed keys retain all 36 bits.'],
            **normalize(text,'fsharp'))
        built=build(row)
        by_key[built['key']]=built
    report['programs']=list(by_key.values());save(D/'dotnet_jit_builds.json',report)

if __name__=='__main__':main()
