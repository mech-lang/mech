(* The Computer Language Benchmarks Game
 * https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
 *
 * Contributed by Don Syme
 * Based on C# contribution by Isaac Gouy, Antti Lankila, A.Nahr, The Anh Tran
 * Uses one byte array rather than strings.
 *)

open System
open System.IO
open System.Collections.Generic
open System.Text
open System.Threading

let input = Console.In
let toBytes (s: string) = s.ToCharArray() |> Array.map byte
let toString (s: byte []) = String(s |> Array.map char)
let prefixes = [| "ggt"; "ggta"; "ggtatt"; "ggtattttaatt"; "ggtattttaatttatagt" |]

let prefixBytes = 
    [| for p in prefixes -> toBytes p |]

let prefixLengths = 
    [| for p in prefixBytes -> p.Length |]

let prefixOffsets = Array.scan (+) 0 prefixLengths
let dnaStart = prefixOffsets.[prefixLengths.Length]

let dna = 
    seq { while true do yield input.ReadLine() }
    |> Seq.takeWhile (fun x -> x <> null)
    |> Seq.skipWhile (fun x -> not (x.StartsWith(">THREE")))
    |> Seq.skip 1
    |> Seq.takeWhile (fun x -> not (x.StartsWith(">")))
    |> String.concat ""
    |> toBytes
    |> Array.append (Array.concat prefixBytes)

let keyHash (k, n) = 
    let mutable h = 0UL
    for i in 0..n - 1 do
        h <- (h <<< 2) ||| uint64 ((int dna.[k + i] &&& 6) >>> 1)
    h

let keyText (k, n) = toString(dna.[k..k + n - 1]).ToUpper()

type KeyData = 
    { mutable count: int
      key: int * int }

let generateFrequencies(frameSize) = 
    let freq = new Dictionary<uint64, KeyData>()
    let mutable v = Unchecked.defaultof<KeyData>
    for i in dnaStart..dna.Length - frameSize do
        let h = keyHash (i, frameSize)
        if freq.TryGetValue(h, &v) then v.count <- v.count + 1
        else freq.[h] <- { count = 1; key = (i, frameSize) }
    freq

let writeCount(n) = 
    let freq = generateFrequencies(prefixLengths.[n])
    let hash = keyHash(prefixOffsets.[n], prefixLengths.[n])
    
    let count = 
        if freq.ContainsKey(hash) then freq.[hash].count
        else 0
    String.Format("{0}\t{1}", count, prefixes.[n].ToUpper())

type Pair = KeyValuePair<uint64, KeyData>

let writeFrequencies(nucleotideLength) = 
    let freq = generateFrequencies(nucleotideLength)
    let items = new ResizeArray<Pair>(freq)
    items.Sort(fun (kv1: Pair) (kv2: Pair) -> 
        let c = kv2.Value.count - kv1.Value.count
        if c = 0 then keyText(kv1.Value.key).CompareTo(keyText(kv2.Value.key))
        else c)
    let buf = new StringBuilder()
    let sum = dna.Length - dnaStart - nucleotideLength + 1
    for element in items do
        let percent = double element.Value.count * 100.0 / double sum
        buf.AppendFormat("{0} {1:f3}\n", keyText(element.Value.key), percent) |> ignore
    buf.ToString()

Async.Parallel [ yield async { return writeFrequencies(1) }
                 yield async { return writeFrequencies(2) }
                 for i in 0..prefixes.Length - 1 do
                     yield async { return writeCount(i) } ]
|> Async.RunSynchronously
|> Array.iter Console.Out.WriteLine
