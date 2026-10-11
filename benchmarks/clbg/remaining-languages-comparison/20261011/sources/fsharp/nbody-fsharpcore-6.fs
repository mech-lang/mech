// The Computer Language Benchmarks Game
// https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
//
// ported from C# version with local variable changes by Anthony Lloyd
// Refactored for Span<> + Intrinsics, with optimizations from C# code by François-David Collin
#nowarn "9"
open System
open System.Runtime.Intrinsics
open System.Runtime.Intrinsics.X86
open FSharp.NativeInterop

// Vector Intrinsics
type Vec = Vector256
type Vf = Vector256<float>

let inline (.*) (v1 : Vf) (v2 : Vf) = Avx.Multiply(v1,v2)
let inline (.+) (v1 : Vf) (v2 : Vf) = Avx.Add(v1,v2)
let inline (.-) (v1 : Vf) (v2 : Vf) = Avx.Subtract(v1,v2)
let inline (./) (v1 : Vf) (v2 : Vf) = Avx.Divide(v1,v2)
let inline (?*) (v1 : Vf) (f : float) = v1 .* Vec.Create(f)

// Order of elements matters (1,0,3,2) gives best codegen
let inline Sum (v : Vf) = v.GetElement(1) + v.GetElement(0)  + v.GetElement(3)
let inline Sqr (v : Vf) = Sum (v .* v)

let vecInit(x,y,z) = Vec.Create(x,y,0.,z)

// Global Variables
let N = 5
let DT = 0.01
let DaysPeryear = 365.24
let Solarmass = 39.4784176043574

let DaysPeryearV = Vec.Create(DaysPeryear)
let SolarmassV = Vec.Create(Solarmass)

// Main functions
let initSystem (P: Span<Vf>) (V: Span<Vf>) (M: Span<Vf>) =
    // Sun
    M.[0] <- Vec.Create(Solarmass)

    // Jupiter
    P.[1] <- vecInit(
            4.84143144246472090e+00,
            -1.16032004402742839e+00,
            -1.03622044471123109e-01
    )
    V.[1] <- vecInit(
            1.66007664274403694e-03,
            7.69901118419740425e-03,
            -6.90460016972063023e-05
    )
    M.[1] <- Vec.Create(Solarmass * 9.54791938424326609e-04)
    
    // Saturn
    P.[2] <- vecInit(
            8.34336671824457987e+00,
            4.12479856412430479e+00,
            -4.03523417114321381e-01
    )
    V.[2] <- vecInit(
            -2.76742510726862411e-03,
            4.99852801234917238e-03,
            2.30417297573763929e-05
    )
    M.[2] <- Vec.Create(Solarmass * 2.85885980666130812e-04)       

    // Uranus
    P.[3] <- vecInit(
            1.28943695621391310e+01,
            -1.51111514016986312e+01,
            -2.23307578892655734e-01
    )
    V.[3] <- vecInit(
            2.96460137564761618e-03,
            2.37847173959480950e-03,
            -2.96589568540237556e-05
    )
    M.[3] <- Vec.Create(Solarmass * 4.36624404335156298e-05)

    // Neptune
    P.[4] <- vecInit(
            1.53796971148509165e+01,
            -2.59193146099879641e+01,
            1.79258772950371181e-01
    )
    V.[4] <- vecInit(
            2.68067772490389322e-03,
            1.62824170038242295e-03,
            -9.51592254519715870e-05
    )
    M.[4] <- Vec.Create(Solarmass * 5.15138902046611451e-05)

    for i in 1..N-1 do
        V.[0] <- V.[0] .- (V.[i] .* M.[i])
        V.[i] <- V.[i] .* DaysPeryearV
    V.[0] <- (V.[0] ./ SolarmassV) .* DaysPeryearV

let inline energy (P: Span<Vf>) (V: Span<Vf>) (M: Span<Vf>) =
    let mutable e = 0.0
    for i in 0..N-1 do
        e <- e + 0.5 * M.[i].ToScalar() *  (Sqr V.[i])
    for i in 0..N-2 do
        for j in (i+1)..N-1 do
            let dstv = P.[i] .- P.[j]
            e <- e - M.[i].ToScalar() * M.[j].ToScalar() / (sqrt (Sqr dstv))
    e

let inline advance (P: Span<Vf>) (V: Span<Vf>) (M: Span<Vf>) (DT: float) repetitions =
    let N = P.Length
    let DTV = Vec.Create(DT)

    for _ in 1..repetitions do
        for i in 0..N-2 do
            let mutable iv = V.[i] 
            let bim = M.[i]
            for j in (i+1)..N-1 do
                let Δx = P.[i] .- P.[j]
                let dsq = Sqr Δx
                let mag = Δx ?* (DT / ((sqrt dsq)*dsq))
                iv <- iv .- (M.[j].*mag)
                V.[j] <- V.[j] .+ (bim.*mag)
            V.[i] <- iv
        for i in 0..4 do
            P.[i] <- P.[i] .+ (V.[i] .* DTV)

let inline vecUnsafeAllocate n =
    let mem = NativePtr.stackalloc<byte>(sizeof<Vf> * n)
    let memvoid = mem |> NativePtr.toVoidPtr 
    Span<Vf>(memvoid,n)

[<EntryPoint>]
let main args =
    let repetitions = try int args.[0] with _ -> 1000
    let P = vecUnsafeAllocate N
    let V = vecUnsafeAllocate N
    let M = vecUnsafeAllocate N
    initSystem P V M
    energy P V M |> printfn "%.9f"
    advance P V M DT repetitions
    energy P V M |> printfn "%.9f"
    0
