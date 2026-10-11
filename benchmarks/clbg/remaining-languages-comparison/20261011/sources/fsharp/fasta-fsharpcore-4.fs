// The Computer Language Benchmarks Game
// https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
//
// Contributed by Valentin Kraevskiy
// Multithreaded by Anthony Lloyd
[<Literal>]
let Width = 60
[<Literal>]
let Width1 = 61
[<Literal>]
let LinesPerBlock = 2048

open System

[<EntryPoint>]
let main (args:string []) =
    let n = if Array.isEmpty args then 1000 else Int32.Parse args.[0]
    let out = Console.OpenStandardOutput()
    let blocks = Array.zeroCreate ((8*n-2)/(Width*LinesPerBlock)+3)
    let bytePool = Buffers.ArrayPool.Shared
    let intPool = Buffers.ArrayPool.Shared

    Threading.ThreadPool.QueueUserWorkItem(fun _ ->
        let writeRandom n offset seed (vs:byte[]) (ps:float[]) =
            // cumulative probability
            let mutable total = ps.[0]
            for i = 1 to ps.Length-1 do
                total <- total + ps.[i]
                ps.[i] <- total

            let mutable seed = seed
            let inline rnds l j =
                let a = intPool.Rent (Width*LinesPerBlock+1)
                for i = 0 to l-1 do
                    seed <- (seed * 3877 + 29573) % 139968
                    a.[i] <- seed
                a.[l] <- j
                a

            let inline bytes l (rnds:int[]) =
                let a = bytePool.Rent (Width1*LinesPerBlock)
                let inline lookup probability =
                    let rec search i =
                        if ps.[i]>=probability then i else search (i+1)
                    vs.[search 0]
                for i = 0 to l-1 do
                    a.[1+i+i/Width] <- lookup (1.0/139968.0 * float rnds.[i])
                intPool.Return rnds
                for i = 0 to (l-1)/Width do
                    a.[i*Width1] <- '\n'B
                a

            for i = offset to offset+(n-1)/(Width*LinesPerBlock)-1 do
                Threading.ThreadPool.QueueUserWorkItem(fun o ->
                    let rnds = o :?> int[]
                    blocks.[rnds.[Width*LinesPerBlock]] <-
                    box(bytes (Width*LinesPerBlock) rnds, Width1*LinesPerBlock)
                , rnds (Width*LinesPerBlock) i) |> ignore

            let remaining = (n-1)%(Width*LinesPerBlock)+1
            Threading.ThreadPool.QueueUserWorkItem(fun o ->
                let rnds = o :?> int[]
                blocks.[rnds.[remaining]] <-
                    box(bytes remaining rnds, remaining+(remaining-1)/Width+1)
            , rnds remaining (offset+(n-1)/(Width*LinesPerBlock))) |> ignore
            seed

        let seed = writeRandom (3*n) 0 42 "acgtBDHKMNRSVWY"B
                    [|0.27;0.12;0.12;0.27;0.02;0.02;0.02;
                      0.02;0.02;0.02;0.02;0.02;0.02;0.02;0.02|]

        writeRandom (5*n) ((3*n-1)/(Width*LinesPerBlock)+2) seed "acgt"B
            [|0.3029549426680;0.1979883004921;0.1975473066391;0.3015094502008|]
        |> ignore
    , null) |> ignore

    out.Write(">ONE Homo sapiens alu"B,0,21)
    let table =
        "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGG\
         GAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGA\
         CCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAAT\
         ACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCA\
         GCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGG\
         AGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCC\
         AGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA"B
    let linesPerBlock = (LinesPerBlock/287+1) * 287
    let repeatedBytes = bytePool.Rent (Width1*linesPerBlock)
    for i = 0 to linesPerBlock*Width-1 do
        repeatedBytes.[1+i+i/Width] <- table.[i%287]
    for i = 0 to (Width*linesPerBlock-1)/Width do
        repeatedBytes.[i*Width1] <- '\n'B
    for _ = 1 to (2*n-1)/(Width*linesPerBlock) do
        out.Write(repeatedBytes, 0, Width1*linesPerBlock)
    let remaining = (2*n-1)%(Width*linesPerBlock)+1
    out.Write(repeatedBytes, 0, remaining+(remaining-1)/Width+1)
    bytePool.Return repeatedBytes
    out.Write("\n>TWO IUB ambiguity codes"B,0,25)

    blocks.[(3*n-1)/(Width*LinesPerBlock)+1] <-
        box("\n>THREE Homo sapiens frequency"B, 30)

    for i = 0 to blocks.Length-1 do
        let mutable t = blocks.[i]
        while isNull t do
            Threading.Thread.Sleep 0
            t <- blocks.[i]
        let bs,l = t :?> byte[] * int
        out.Write(bs,0,l)
        if l=Width1*LinesPerBlock then bytePool.Return bs

    out.WriteByte '\n'B
    0
