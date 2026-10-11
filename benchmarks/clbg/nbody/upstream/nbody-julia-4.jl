# The Computer Language Benchmarks Game
# https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
#
# Contributed by Andrei Fomiga, Stefan Karpinski, Viral B. Shah, Jeff
# Bezanson, and Adam Beckmeyer.
# Based on Mark C. Lewis's Java version.

using Printf

# Constants
const SOLAR_MASS = 4 * pi * pi
const DAYS_PER_YEAR = 365.24

# Use a struct instead of mutable struct since a struct can be stored
# inline in an array avoiding the overhead of following a pointer
struct Body
    x::Float64
    y::Float64
    z::Float64
    vx::Float64
    vy::Float64
    vz::Float64
    m::Float64
end

function init_sun(bodies)
    px = py = pz = 0.0
    for b in bodies
        px -= b.vx * b.m
        py -= b.vy * b.m
        pz -= b.vz * b.m
    end
    Body(0.0, 0.0, 0.0, px / SOLAR_MASS, py / SOLAR_MASS, pz / SOLAR_MASS, SOLAR_MASS)
end

function advance!(bodies, dt)
    n = length(bodies)
    @inbounds for i=1:n-1
        bi = bodies[i]

        # Since the fields of bi aren't mutable, we track the changing
        # value of bi's velocity outside of the Body struct
        ivx = bi.vx
        ivy = bi.vy
        ivz = bi.vz

        for j=i+1:n
            bj = bodies[j]
            
            dx = bi.x - bj.x
            dy = bi.y - bj.y
            dz = bi.z - bj.z

            dsq = dx^2 + dy^2 + dz^2
            mag = dt / (dsq * √dsq)

            ivx -= dx * bj.m * mag
            ivy -= dy * bj.m * mag
            ivz -= dz * bj.m * mag

            bodies[j] = Body(
                bj.x, bj.y, bj.z,
                bj.vx + dx * bi.m * mag,
                bj.vy + dy * bi.m * mag,
                bj.vz + dz * bi.m * mag,
                bj.m
            )
        end

        bodies[i] = Body(
            bi.x, bi.y, bi.z,
            ivx, ivy, ivz,
            bi.m
        )
    end

    @inbounds for i=1:n
        bi = bodies[i]
        bodies[i] = Body(
            bi.x + dt * bi.vx, bi.y + dt * bi.vy, bi.z + dt * bi.vz,
            bi.vx, bi.vy, bi.vz,
            bi.m
        )
    end
end

function energy(bodies)
    n = length(bodies)
    e = 0.0
    @inbounds for i=1:n
        bi = bodies[i]

        e += 0.5 * bi.m * (bi.vx^2 + bi.vy^2 + bi.vz^2)
        for j=i+1:n
            bj = bodies[j]
            
            d = √((bi.x - bj.x)^2 + (bi.y - bj.y)^2 + (bi.z - bj.z)^2)
            e -= bi.m * bodies[j].m / d
        end
    end
    e
end


function nbody(n)
    jupiter = Body( 4.84143144246472090e+0,                   # x
                   -1.16032004402742839e+0,                   # y
                   -1.03622044471123109e-1,                   # z
                    1.66007664274403694e-3 * DAYS_PER_YEAR,   # vx
                    7.69901118419740425e-3 * DAYS_PER_YEAR,   # vy
                   -6.90460016972063023e-5 * DAYS_PER_YEAR,   # vz
                    9.54791938424326609e-4 * SOLAR_MASS)      # mass

    saturn = Body( 8.34336671824457987e+0,
                   4.12479856412430479e+0,
                  -4.03523417114321381e-1,
                  -2.76742510726862411e-3 * DAYS_PER_YEAR,
                   4.99852801234917238e-3 * DAYS_PER_YEAR,
                   2.30417297573763929e-5 * DAYS_PER_YEAR,
                   2.85885980666130812e-4 * SOLAR_MASS)

    uranus = Body( 1.28943695621391310e+1,
                  -1.51111514016986312e+1,
                  -2.23307578892655734e-1,
                   2.96460137564761618e-3 * DAYS_PER_YEAR,
                   2.37847173959480950e-3 * DAYS_PER_YEAR,
                  -2.96589568540237556e-5 * DAYS_PER_YEAR,
                   4.36624404335156298e-5 * SOLAR_MASS)

    neptune = Body( 1.53796971148509165e+1,
                   -2.59193146099879641e+1,
                    1.79258772950371181e-1,
                    2.68067772490389322e-3 * DAYS_PER_YEAR,
                    1.62824170038242295e-3 * DAYS_PER_YEAR,
                   -9.51592254519715870e-5 * DAYS_PER_YEAR,
                    5.15138902046611451e-5 * SOLAR_MASS)

    bodies = [jupiter, saturn, uranus, neptune]
    pushfirst!(bodies, init_sun(bodies))


    @printf("%.9f\n", energy(bodies))
    for i = 1:n
        advance!(bodies, 0.01)
    end
    @printf("%.9f\n", energy(bodies))
end

isinteractive() || nbody(parse(Int, ARGS[1]))
