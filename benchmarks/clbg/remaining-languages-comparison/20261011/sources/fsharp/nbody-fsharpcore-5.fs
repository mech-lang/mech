// The Computer Language Benchmarks Game
// https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
//
// ported from C# version with local variable changes by Anthony Lloyd
// Refactored for Span<> and Intrinsics, with optimizations from C# code by François-David Collin
#nowarn "9"
open System
open System.Runtime.Intrinsics
open System.Runtime.Intrinsics.X86

// Vector code
let ArraytoVec (v : float array) =
    Vector256.Create(v.[0],v.[1],v.[2],0.)

[<Struct>]
type Body =
    val mutable P: Vector256<float> // position
    val mutable V: Vector256<float> // speed
    val Mass: Vector256<float>
    new(p: float[],v: float[],mass:float) = { P = ArraytoVec p
                                              V = ArraytoVec v 
                                              Mass = Vector256.Create(mass) }


let inline (.*) (v1 : Vector256<float>) (v2 : Vector256<float>) = Avx.Multiply(v1,v2)
let inline (.+) (v1 : Vector256<float>) (v2 : Vector256<float>) = Avx.Add(v1,v2)
let inline (.-) (v1 : Vector256<float>) (v2 : Vector256<float>) = Avx.Subtract(v1,v2)
let inline (./) (v1 : Vector256<float>) (v2 : Vector256<float>) = Avx.Divide(v1,v2)
let inline (?*) (v1 : Vector256<float>) (f : float) = v1 .* Vector256.Create(f)

let inline Sum (v : Vector256<float>) = v.GetElement(1) + v.GetElement(0)  + v.GetElement(2)
let inline Sqr (v : Vector256<float>) = Sum (v .* v)

// Global Variables
let N = 5
let DT = 0.01
let DaysPeryear = 365.24
let Solarmass = 39.4784176043574

let DaysPeryearV = Vector256.Create(DaysPeryear)
let SolarmassV = Vector256.Create(Solarmass)
let DTV = Vector256.Create(DT)

// Main functions
let initSystem (bodies: Span<Body>) =
    for i in 1..N-1 do
        bodies.[0].V <- bodies.[0].V .- bodies.[i].V .* bodies.[i].Mass
    bodies.[0].V <- bodies.[0].V ./ SolarmassV
    for i in 0..N-1 do
        bodies.[i].V <- bodies.[i].V .* DaysPeryearV

let inline energy (bodies : Span<Body>)=
    let mutable e = 0.0
    for b in bodies do
        e <- e + 0.5 * b.Mass.ToScalar() *  (Sqr b.V)
    for i in 0..N-2 do
        for j in (i+1)..N-1 do
            let dstv = bodies.[i].P .- bodies.[j].P
            e <- e - bodies.[i].Mass.ToScalar() * bodies.[j].Mass.ToScalar() / (sqrt (Sqr dstv))
    e

let inline advance (bodies : Span<Body>) repetitions =
    for _ in 1..repetitions do
        for i in 0..N-2 do
            let mutable iv = bodies.[i].V 
            let bim = bodies.[i].Mass
            for j in (i+1)..N-1 do
                let Δx = bodies.[i].P .- bodies.[j].P
                let dsq = Sum (Δx .* Δx)
                let mag = Δx ?* (DT / ((sqrt dsq)*dsq))
                iv <- iv .- (bodies.[j].Mass.*mag)
                bodies.[j].V <- bodies.[j].V .+ (bim.*mag)
            bodies.[i].V <- iv
        for i in 0..4 do
            bodies.[i].P <- bodies.[i].P .+ (bodies.[i].V .* DTV)

// Initial conditions
let bodiesOrig = [|
    Body( // Sun
        [|0.0;0.0;0.0|],
        [|0.0;0.0;0.0|],
        Solarmass
    )
    Body( // Jupiter
        [|
            4.84143144246472090e+00
            -1.16032004402742839e+00
            -1.03622044471123109e-01
        |],
        [|
            1.66007664274403694e-03
            7.69901118419740425e-03
            -6.90460016972063023e-05
        |],
        Solarmass * 9.54791938424326609e-04
    )   
    Body( // Saturn
        [|
            8.34336671824457987e+00
            4.12479856412430479e+00
            -4.03523417114321381e-01
        |],
        [|
            -2.76742510726862411e-03
            4.99852801234917238e-03
            2.30417297573763929e-05
        |],
        Solarmass * 2.85885980666130812e-04        
    )   
    Body( // Uranus
        [|
            1.28943695621391310e+01
            -1.51111514016986312e+01
            -2.23307578892655734e-01
        |],
        [|
            2.96460137564761618e-03
            2.37847173959480950e-03
            -2.96589568540237556e-05
        |],
        Solarmass * 4.36624404335156298e-05        
    )   
    Body( // Neptune
        [|
            1.53796971148509165e+01
            -2.59193146099879641e+01
            1.79258772950371181e-01
        |],
        [|
            2.68067772490389322e-03
            1.62824170038242295e-03
            -9.51592254519715870e-05
        |],
        Solarmass * 5.15138902046611451e-05       
    )   
|]

[<EntryPoint>]
let main args =
    let repetitions = try int args.[0] with _ -> 1000
    let bodies = Span<Body>(bodiesOrig)
    initSystem bodies
    energy bodies |> printfn "%.9f"
    advance bodies repetitions
    energy bodies |> printfn "%.9f"
    0
