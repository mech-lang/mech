(* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   Adapted by Antti Lankila from the earlier Isaac Gouy's implementation
   Add multithread & tweaks from C++ by The Anh Tran
   Ported to F# by Don Syme
   Add memory reduction and performance tweaks by Jomo Fisher
*)

open System
open System.Threading
open System.IO

let out = Console.OpenStandardOutput()

// Helper type to accumulate a bit array
type BitAccumulator(n) = 
    let numBytes = (n+7)/8
    let numBits = n &&& 7
    let mutable byteCount = 0
    let mutable bitNum = 0
    let mutable byteAcc = 0
    let mutable pdata : byte array = Array.zeroCreate numBytes

    member x.Reinit() = 
        byteCount <- 0
        bitNum <- 0
        byteAcc <- 0       

    member x.Add b =     
        byteAcc <- (byteAcc <<< 1) ||| (if b then 1 else 0);

        bitNum <- bitNum + 1
        if bitNum = 8 then 
           pdata.[byteCount] <- byte byteAcc;
           byteCount <- byteCount + 1;
           byteAcc <- 0;
           bitNum <- 0;

    member x.Close() =     
        // write left over bits
        if bitNum <> 0 then
            byteAcc <- byteAcc <<< (8 - numBits)
            pdata.[byteCount] <- byte byteAcc
            byteCount <- byteCount+1

    member x.Data = pdata
    member x.ByteCount = byteCount

type ThreadState(n) = 
    let ba = BitAccumulator(n)
    let mutable finishedLine = -1
    member ts.BitAccumulator = ba
    member ts.IsReadyToWork = finishedLine = -1
    member ts.SetFinishedLine(lineNumber) =
        finishedLine <- lineNumber
    member ts.TryWriteLine(nextLine) = 
        if finishedLine = nextLine then
            out.Write( ba.Data, 0, ba.ByteCount);
            finishedLine <- -1
            ba.Reinit()
            true
        else false

// Initialize an array by a parallel init using all available processors
let parallelArrayInit n f = 
    let currentLine = ref -1
    let lastWritten = ref -1
    let rec loop (ts:ThreadState) = 
        if ts.IsReadyToWork then 
            let y = Interlocked.Increment(&currentLine.contents)
            if y < n then 
                f ts.BitAccumulator y
                ts.SetFinishedLine(y)
                loop(ts)
        elif ts.TryWriteLine(!lastWritten + 1) then
            Interlocked.Increment(&lastWritten.contents) |> ignore
            loop(ts)
        else loop(ts)

    Async.Parallel [ for i in 1 .. Environment.ProcessorCount -> async {do loop(ThreadState(n)) } ] 
       |> Async.Ignore 
       |> Async.RunSynchronously

[<EntryPoint>]
let main args =
    let start = System.DateTime.Now
    let numLines = if (args.Length > 0) then int args.[0] else 200 
    Console.Out.Write("P4\n{0} {0}\n", numLines);
    let inversen = 2.0 / float numLines;

    parallelArrayInit numLines (fun ba y -> 

        let civ = float y * inversen - 1.0;

        for x = 0 to numLines - 1 do 
            let crv = float x * inversen - 1.5;

            let rec innerLoop i zrv ziv trv tiv = 
               let ziv = (zrv*ziv) + (zrv*ziv) + civ;
               let zrv = trv - tiv + crv;

               let trv = zrv * zrv;
               let tiv = ziv * ziv;
               if ((trv + tiv) <= 4.0) && (i > 0) then 
                   innerLoop (i-1) zrv ziv trv tiv
               else
                   i

            let i = innerLoop 49 crv civ (crv * crv) (civ * civ)
            ba.Add ((i = 0))
        ba.Close())

    0
