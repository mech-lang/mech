// The Computer Language Benchmarks Game
// https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
//
// contributed by Valentin Kraevskiy
// modified by Peter Kese

let pi = 3.141592653589793
let daysPerYear = 365.24
let solarMass = 4.0 * pi ** 2.0

type Body = {
    mutable X: float; mutable Y: float; mutable Z: float
    Mass: float
    mutable VX: float; mutable VY: float; mutable VZ: float
}

let planets = [|
    { // jupiter
        X = 4.84143144246472090e+00
        Y = -1.16032004402742839e+00
        Z = -1.03622044471123109e-01
        VX = 1.66007664274403694e-03 * daysPerYear
        VY = 7.69901118419740425e-03 * daysPerYear
        VZ = -6.90460016972063023e-05 * daysPerYear
        Mass = 9.54791938424326609e-04 * solarMass }
    { // saturn
        X = 8.34336671824457987e+00
        Y = 4.12479856412430479e+00
        Z = -4.03523417114321381e-01
        VX = -2.76742510726862411e-03 * daysPerYear
        VY = 4.99852801234917238e-03 * daysPerYear
        VZ = 2.30417297573763929e-05 * daysPerYear
        Mass = 2.85885980666130812e-04 * solarMass }
    { // uranus
        X = 1.28943695621391310e+01
        Y = -1.51111514016986312e+01
        Z = -2.23307578892655734e-01
        VX = 2.96460137564761618e-03 * daysPerYear
        VY = 2.37847173959480950e-03 * daysPerYear
        VZ = -2.96589568540237556e-05 * daysPerYear
        Mass = 4.36624404335156298e-05 * solarMass }
    { // neptune
        X = 1.53796971148509165e+01
        Y = -2.59193146099879641e+01
        Z = 1.79258772950371181e-01
        VX = 2.68067772490389322e-03 * daysPerYear
        VY = 1.62824170038242295e-03 * daysPerYear
        VZ = -9.51592254519715870e-05 * daysPerYear
        Mass = 5.15138902046611451e-05 * solarMass }
|]

let bodies = [
    let svx,svy,svz = // sun momentum
        planets |> Array.fold (fun (x, y, z) body ->
            let c = body.Mass / solarMass
            (x - c * body.VX, y - c * body.VY, z - c * body.VZ))
            (0.0, 0.0, 0.0)
    yield { X=0.0; Y=0.0; Z=0.0; VX=svx; VY=svy; VZ=svz; Mass=solarMass } // sun
    yield! planets
]

type System = {
    bodies: Body list
    pairs: struct (Body * Body) list
}

let system = {
    bodies=bodies
    pairs=[ for i in 0..bodies.Length-2 do
                for j in i+1..bodies.Length-1 do
                    struct (bodies.[i], bodies.[j]) ]
}

let simulate {bodies=bodies; pairs=pairs} repetitions dt =
    for _ in 1..repetitions do
        // calculate pairwise forces
        for b1,b2 in pairs do
            let dx,dy,dz = b1.X-b2.X, b1.Y-b2.Y, b1.Z-b2.Z
            let dist2 = dx*dx + dy*dy + dz*dz
            let mag = dt / (dist2 * sqrt dist2)
            let mag2 = b2.Mass * mag
            let mag1 = b1.Mass * mag
            b1.VX <- b1.VX - mag2 * dx
            b1.VY <- b1.VY - mag2 * dy
            b1.VZ <- b1.VZ - mag2 * dz
            b2.VX <- b2.VX + mag1 * dx
            b2.VY <- b2.VY + mag1 * dy
            b2.VZ <- b2.VZ + mag1 * dz
        // move bodies
        for body in bodies do
            body.X <- body.X + dt * body.VX
            body.Y <- body.Y + dt * body.VY
            body.Z <- body.Z + dt * body.VZ

let energy system =
    let ePlanets = system.bodies |> List.fold (fun e b ->
        e + 0.5 * b.Mass * (b.VX*b.VX + b.VY*b.VY + b.VZ*b.VZ)) 0.0
    let ePairs = system.pairs |> List.fold (fun e struct (b1,b2) ->
        let dx,dy,dz = b1.X-b2.X, b1.Y-b2.Y, b1.Z-b2.Z
        let dist = sqrt (dx*dx + dy*dy + dz*dz)
        e + b1.Mass * b2.Mass / dist) 0.0
    ePlanets - ePairs

[<EntryPoint>]
let main args =
    let repetitions = try int args.[0] with _ -> 20000000
    let e0 = energy system
    simulate system repetitions 0.01
    printf "%.9f\n%.9f\n" e0 (energy system)
    0
