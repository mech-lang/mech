#=
The Computer Language Benchmarks Game
 https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

 direct transliteration of the swift#3 program by Ralph Ganszky and Daniel Muellenborn

 modified for Julia 1.0 by Simon Danisch
 tweaked for performance by maltezfaria and Adam Beckmeyer
=#

const masks = (0b01111111, 0b10111111, 0b11011111, 0b11101111, 0b11110111,
               0b11111011, 0b11111101, 0b11111110)

# Calculate the byte to print for a given vector of real numbers cr
# and a given imaginary component ci. This function should give the
# same result whether prune is true or false but may be faster or
# slower depending on the input.
function mand8(cr, ci, prune)
    r = cr
    t = i = ntuple(_-> ci, 8)

    # In cases where the last call to mand8 resulted in 0x00, the next
    # call is much more likely to result in 0x00, so it's worth it to
    # check several times if the calculation can be aborted
    # early. Otherwise, the relatively costly check can be eliminated.
    if prune
        for _=1:10
            for _=1:5
                r, i, t = calc_sum(r, i, cr, ci)
            end
            all(>(4.0), t) && return 0x00
        end
    else
        for _=1:50
            r, i, t = calc_sum(r, i, cr, ci)
        end
    end

    byte = 0xff
    for i=1:8
        t[i] <= 4.0 || (byte &= masks[i])
    end
    byte
end

# Single iteration of mandelbrot calculation for vector r of real
# components and vector i or imaginary components.
@inline function calc_sum(r, i, cr, ci)
    r2 = r .* r
    i2 = i .* i
    ri = r .* i

    t = r2 .+ i2
    r = r2 .- i2 .+ cr
    i = ri .+ ri .+ ci
    r, i, t
end

# Write n by n portable bitmap image of mandelbrot set to io
function mandelbrot(io, n)
    inv_ = 2.0 / n
    xvals = Float64[i * inv_ - 1.5 for i=0:n-1]
    yvals = Float64[i * inv_ - 1.0 for i=0:n-1]

    rows = Vector{UInt8}(undef, n * n ÷ 8)
    Threads.@threads for y=1:n
        ci = @inbounds yvals[y]
        start = (y - 1) * n ÷ 8
        # The first iteration within a row will generally return 0x00
        prune = true
        @inbounds for x=1:8:n
            cr = ntuple(i-> xvals[x+i-1], 8)
            res = mand8(cr, ci, prune)

            rows[start + x÷8 + 1] = res
            prune = res == 0x00
        end
    end

    write(io, "P4\n$n $n\n")
    write(io, rows)
end

isinteractive() || mandelbrot(stdout, parse(Int, ARGS[1]))
