# The Computer Language Benchmarks Game
# https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

# based on the Javascript program
# optimizations by Kristoffer Carlsson

using Printf

A(i,j) = 1.0 / ( (((i+j)*(i+j+1)) >> 1) + i+1)

function Au!(w, u)
    n = length(u)
    Threads.@threads for i = 1:n
        w[i] = 0
        z = 0.0
        @simd for j = 1:n
           @inbounds z += A(i-1, j-1) * u[j]
        end
        w[i] = z
    end
end

function Atu!(v, w)
    n = length(w)
    Threads.@threads for i = 1:n
        z = 0.0
        @simd for j = 1:n
           @inbounds z += A(j-1,i-1) * w[j]
        end
        v[i] = z
    end
end

function AtAu!(w, v, u)
    Au!(w, u)
    Atu!(v, w)
end

function perf_spectralnorm(n::Int=100)
    u = ones(Float64, n)
    v = zeros(Float64 ,n)
    w = zeros(Float64, n)
    vv = vBv = 0
    for i = 1:10
        AtAu!(w, v, u)
        AtAu!(w, u, v)
    end
    for i = 1:n
        vBv += u[i]*v[i]
        vv += v[i]*v[i]
    end
    return sqrt(vBv/vv)
end

n = parse(Int,ARGS[1])
@printf("%.9f\n", perf_spectralnorm(n))
