// The Computer Language Benchmarks Game
// https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
//
// Based on C# version by Isaac Gouy, The Anh Tran, Alan McGovern
// Contributed by Don Syme
// Small optimisations by Anthony Lloyd

#nowarn "9"

open Microsoft.FSharp.NativeInterop

let approximate n1 u tmp v rbegin rend (barrier:System.Threading.Barrier) =

    let inline multiplyAv v Av A =
        for i = rbegin to rend do
            let mutable sum = A i 0 * NativePtr.read v
            for j = 1 to n1 do
                sum <- sum + A i j * NativePtr.get<float> v j
            NativePtr.set Av i sum

    let inline multiplyatAv v tmp atAv =
        let inline A i j = 1.0 / float((i + j) * (i + j + 1) / 2 + i + 1)
        multiplyAv v tmp A
        barrier.SignalAndWait()
        let inline At i j = A j i
        multiplyAv tmp atAv At
        barrier.SignalAndWait()

    for __ = 0 to 9 do
        multiplyatAv u tmp v
        multiplyatAv v tmp u

    let vbegin = NativePtr.get v rbegin
    let mutable vv = vbegin * vbegin
    let mutable vBv = vbegin * NativePtr.get u rbegin
    for i = rbegin+1 to rend do
        let vi = NativePtr.get v i
        vv <- vv + vi * vi
        vBv <- vBv + vi * NativePtr.get u i
    vBv, vv

[<EntryPoint>]
let main args =
    let n = try int args.[0] with _ -> 2500
    let u = fixed &(Array.create n 1.0).[0]
    let tmp = NativePtr.stackalloc n
    let v = NativePtr.stackalloc n
    let nthread = System.Environment.ProcessorCount
    let barrier = new System.Threading.Barrier(nthread)
    let chunk = n / nthread
    let aps =
        [ for i = 0 to nthread-1 do
            let r1 = i * chunk
            let r2 = if (i < (nthread - 1)) then r1 + chunk - 1 else n-1
            yield async { return approximate (n-1) u tmp v r1 r2 barrier } ]
        |> Async.Parallel
        |> Async.RunSynchronously
    sqrt(Array.sumBy fst aps/Array.sumBy snd aps).ToString("F9")
    |> stdout.WriteLine
    exit 0
