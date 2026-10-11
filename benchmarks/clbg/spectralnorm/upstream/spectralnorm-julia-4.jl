#=
The Computer Language Benchmarks Game
https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
contributed by Adam Beckmeyer with help from Vincent Yu
=#

A(i, j) = (i + j - 2) * (i + j - 1) / 2 + i
At(i, j) = A(j, i)

# Multiply vector v by the matrix represented by function f (f(i, j) returns the element in
# i-th row and j-th column) and store result in out
function mul_by_f!(f, v, out)
    n = length(v)
    # Threads.@threads has lower overhead than Threads.@spawn
    Threads.@threads :static for i=1:n
        x1 = x2 = 0.0
        # If we used the @simd macro instead of manually iterating by 2s, the compiler would
        # emit instructions using the ymm registers instead of xmm which appears to be
        # slower on ivybridge cpus
        @inbounds for j=1:2:n
            # Manually convert indices to Float64 so that arithmetic in function f can be
            # carried out using vectorized  floating point arithmetic
            x1 += v[j] / f(Float64(i), Float64(j))
            x2 += v[j+1] / f(Float64(i), Float64(j+1))
        end
        @inbounds out[i] = x1 + x2
    end
end

# Multiply vector v by matrix A and store result in out
mul_by_A!(v, out) = mul_by_f!(A, v, out)

# Multiply vector v by matrix A' and store result in out
mul_by_At!(v, out) = mul_by_f!(At, v, out)

# Multiply v by (A' * A) and store result in out using w as a temporary workspace
function mul_by_AtA!(v, out, w)
    mul_by_A!(v, w)
    mul_by_At!(w, out)
end

function main(n)
    # This program is not compatible with odd values of n
    isodd(n) && (n += 1)

    u = ones(Float64, n)
    v = Vector{Float64}(undef, n)
    # temporary working vector w
    w = Vector{Float64}(undef, n)

    for _=1:10
        mul_by_AtA!(u, v, w)
        mul_by_AtA!(v, u, w)
    end

    uv = vv = 0.0
    @inbounds for i=1:n
        uv += u[i] * v[i]
        vv += v[i] * v[i]
    end
    sqrt(uv / vv)
end

# The expanded form of Printf.@printf macro takes a significant time to compile, accounting
# for 25% of the total program runtime. Base.Ryu.writefixed(::Float64, ::Int) should already
# be compiled into the default system image.
isinteractive() || println(Base.Ryu.writefixed(main(parse(Int, ARGS[1])), 9))
