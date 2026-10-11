# The Computer Language Benchmarks Game
# https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
#
# contributed by David Campbell
# modified by Jarret Revels, Alex Arslan

const ITER = 50

function ismandel(z::Complex{Float64})
    c = z
    for n = 1:ITER
        if abs2(z) > 4
            return false
        end
        z = z^2 + c
    end
    return true
end

function draw_mandel(M::Array{UInt8, 2}, n::Int)
    for y = 0:n-1
        ci = 2y/n - 1
        for x = 0:n-1
            c = complex(2x/n - 1.5, ci)
            if ismandel(c)
                M[div(x, 8) + 1, y + 1] |= 1 << UInt8(7 - x%8)
            end
        end
    end
end

function perf_mandelbrot(n::Int=200)
    if n%8 != 0
        error("Error: n of $n is not divisible by 8")
    end

    M = zeros(UInt8, div(n, 8), n)
    draw_mandel(M, n)
    write(stdout, "P4\n$n $n\n")
    write(stdout, M)
end

n = parse(Int,ARGS[1])
perf_mandelbrot(n)
