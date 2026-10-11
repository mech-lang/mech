// The Computer Language Benchmarks Game
// https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
//
// Based on spectral-norm Rust #6 program
// Contributed by Arseniy Zlobintsev

open System
open System.Runtime.Intrinsics
open System.Runtime.Intrinsics.X86
open System.Threading.Tasks

type F64x2 = Vector128<float>

let inline hadd (x: F64x2) y =
    if Sse41.IsSupported then Sse41.HorizontalAdd(x, y)
    else Vector128.Create(Vector128.Sum x, Vector128.Sum y)

let inline a i j =
    let i0, i1 = i
    let j0, j1 = j
    Vector128.Create(
        (i0 + j0) * (i0 + j0 + 1.0) / 2.0 + i0 + 1.0,
        (i1 + j1) * (i1 + j1 + 1.0) / 2.0 + i1 + 1.0
    )

let inline mult (v: F64x2 array) (out: F64x2 array) ([<InlineIfLambda>] f) =
    Parallel.For(0, v.Length, fun i ->
        let fi = float (2 * i)
        let i0, i1 = (fi, fi), (fi + 1.0, fi + 1.0)
        
        let mutable sum0, sum1 = F64x2.Zero, F64x2.Zero
        for j in 0..v.Length-1 do
            let x = v[j]
            let j = float (2 * j), float (2 * j + 1)
            sum0 <- sum0 + x / f i0 j
            sum1 <- sum1 + x / f i1 j
        
        out[i] <- hadd sum0 sum1
    ) |> ignore

let multAtAv v out tmp =
    mult v tmp a
    mult tmp out (fun i j -> a j i)

let dot (v: F64x2 array) (u: F64x2 array) =
    let mutable acc = F64x2.Zero
    for i in 0..v.Length-1 do
        acc <- acc + v[i] * u[i]
    Vector128.Sum acc

[<EntryPoint>]
let main argv =
    let n = (try int argv[0] with _ -> 500) / 2
    let u = Array.create n (Vector128.Create 1.0)
    let v = Array.zeroCreate n
    let tmp = Array.zeroCreate n

    for _ in 0..9 do
        multAtAv u v tmp
        multAtAv v u tmp
    
    let answer = sqrt (dot u v / dot v v)

    Console.WriteLine(answer.ToString "F9")
    0
