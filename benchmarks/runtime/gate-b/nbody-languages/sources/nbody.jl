# The Computer Language Benchmarks Game
# https://benchmarksgame-team.pages.debian.net/benchmarksgame/program/nbody-julia-5.html
# Contributed by Andrei Fomiga, Stefan Karpinski, Viral B. Shah, Jeff
# Bezanson, smallnamespaces, Adam Beckmeyer, and Vincent Yu.
# Adapted only to warm the JIT, time the advance loop, and emit CSV.

using Base.Cartesian

const SOLAR_MASS = 4 * pi * pi
const DAYS_PER_YEAR = 365.24
const PAIRS = Tuple((i, j) for i = 1:4 for j = i + 1:5)

struct Body
    x::NTuple{3,Float64}
    v::NTuple{3,Float64}
    m::Float64
end

function init_sun(bodies)
    p = (0.0, 0.0, 0.0)
    for b in bodies
        p = p .- b.v .* b.m
    end
    Body((0.0, 0.0, 0.0), p ./ SOLAR_MASS, SOLAR_MASS)
end

@inline function advance!(bodies)
    delta_x = @ntuple 10 k -> @inbounds bodies[PAIRS[k][1]].x .- bodies[PAIRS[k][2]].x
    distance_squared = @ntuple 10 k -> sum(delta_x[k] .* delta_x[k])
    reciprocal_distance = @ntuple 10 k -> @fastmath Float64(1 / sqrt(Float32(distance_squared[k])))
    reciprocal_distance = @ntuple 10 k -> 1.5reciprocal_distance[k] -
        0.5distance_squared[k] * reciprocal_distance[k] *
        (reciprocal_distance[k] * reciprocal_distance[k])
    magnitude = @ntuple 10 k -> 0.01reciprocal_distance[k] *
        (reciprocal_distance[k] * reciprocal_distance[k])

    k = 1
    @inbounds for i = 1:4
        body_i = bodies[i]
        velocity_i = body_i.v
        for j = i + 1:5
            velocity_i = velocity_i .- delta_x[k] .* (magnitude[k] * bodies[j].m)
            bodies[j] = Body(
                bodies[j].x,
                bodies[j].v .+ delta_x[k] .* (magnitude[k] * body_i.m),
                bodies[j].m,
            )
            k += 1
        end
        bodies[i] = Body(body_i.x, velocity_i, body_i.m)
    end

    @inbounds for i = 1:5
        body_i = bodies[i]
        bodies[i] = Body(body_i.x .+ body_i.v .* 0.01, body_i.v, body_i.m)
    end
end

function energy(bodies)
    total = 0.0
    @inbounds for body in bodies
        total += 0.5body.m * sum(body.v .* body.v)
    end
    @inbounds for (i, j) in PAIRS
        delta_x = bodies[i].x .- bodies[j].x
        total -= bodies[i].m * bodies[j].m / sqrt(sum(delta_x .* delta_x))
    end
    total
end

nbody!(bodies, turns) = for _ = 1:turns
    advance!(bodies)
end

const INITIAL_BODIES = [
    Body(
        (4.84143144246472090e0, -1.16032004402742839e0, -1.03622044471123109e-1),
        (
            1.66007664274403694e-3DAYS_PER_YEAR,
            7.69901118419740425e-3DAYS_PER_YEAR,
            -6.90460016972063023e-5DAYS_PER_YEAR,
        ),
        9.54791938424326609e-4SOLAR_MASS,
    ),
    Body(
        (8.34336671824457987e0, 4.12479856412430479e0, -4.03523417114321381e-1),
        (
            -2.76742510726862411e-3DAYS_PER_YEAR,
            4.99852801234917238e-3DAYS_PER_YEAR,
            2.30417297573763929e-5DAYS_PER_YEAR,
        ),
        2.85885980666130812e-4SOLAR_MASS,
    ),
    Body(
        (1.28943695621391310e1, -1.51111514016986312e1, -2.23307578892655734e-1),
        (
            2.96460137564761618e-3DAYS_PER_YEAR,
            2.37847173959480950e-3DAYS_PER_YEAR,
            -2.96589568540237556e-5DAYS_PER_YEAR,
        ),
        4.36624404335156298e-5SOLAR_MASS,
    ),
    Body(
        (1.53796971148509165e1, -2.59193146099879641e1, 1.79258772950371181e-1),
        (
            2.68067772490389322e-3DAYS_PER_YEAR,
            1.62824170038242295e-3DAYS_PER_YEAR,
            -9.51592254519715870e-5DAYS_PER_YEAR,
        ),
        5.15138902046611451e-5SOLAR_MASS,
    ),
]
push!(INITIAL_BODIES, init_sun(INITIAL_BODIES))

function benchmark(turns)
    warmup = copy(INITIAL_BODIES)
    nbody!(warmup, 1)

    bodies = copy(INITIAL_BODIES)
    initial_energy = energy(bodies)
    GC.gc()
    measured = @timed nbody!(bodies, turns)
    seconds = measured.time
    println(
        "julia-game-5,$turns,$(round(seconds, digits=9))," *
        "$(round(seconds * 1e9 / turns, digits=3))," *
        "$(round(turns / seconds, digits=3))," *
        "$(round(initial_energy, digits=12)),$(round(energy(bodies), digits=12))," *
        "$(round(measured.gctime, digits=9)),$(measured.bytes)",
    )
end

benchmark(isempty(ARGS) ? 1_000_000 : parse(Int, ARGS[1]))
