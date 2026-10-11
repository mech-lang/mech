# The Computer Language Benchmarks Game
# https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
#
# Contributed by Kristoffer Carlsson
# Updated by Adam Beckmeyer

const LINEWIDTH = 60

const IM = Int32(139968)
const IA = Int32(3877)
const IC = Int32(29573)
gen_random(seed) = ((seed * IA) + IC) % IM

# Vector{UInt8} faster indexing than Base.CodeUnits{UInt8,String} (julia 1.2)
const alu = Vector{UInt8}(string(
   "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGG",
   "GAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGA",
   "CCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAAT",
   "ACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCA",
   "GCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGG",
   "AGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCC",
   "AGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA"))

const iub1 = Vector{UInt8}("acgtBDHKMNRSVWY")
# Instead of the probability, iub2 is the integer upper-bounding the random
# integer to the equivalent index in iub1
const iub2 =
    floor.(Int32, IM .* cumsum([0.27, 0.12, 0.12, 0.27, 0.02,0.02, 0.02, 0.02,
                                0.02, 0.02,0.02, 0.02, 0.02, 0.02, 0.02]))
const homosapiens1 = Vector{UInt8}("acgt")
const homosapiens2 =
    floor.(Int32, IM .* cumsum([0.3029549426680, 0.1979883004921,
                                0.1975473066391, 0.3015094502008]))

# Defining a function with successive comparisons is faster than getindex lookup
for (op, symb, pr) in ((:iub, iub1, iub2),
                       (:homosapiens, homosapiens1, homosapiens2))
    # successively compares x with all values in pr and returns value from symb
    @eval function $op(x)
        $((:(x <= $p && return $s) for (p, s) in zip(pr, symb))...)
        'a' % UInt8
    end
end

function repeat_fasta(io, src, n)
    buffer = Vector{UInt8}(undef, LINEWIDTH + 1)
    buffer[end] = '\n' % UInt8

    col = count = 0
    @inbounds for c in Iterators.cycle(src)
        col += 1
        buffer[col] = c
        if col == LINEWIDTH
            write(io, buffer)
            col = 0
        end
        count += 1
        count == n && break
    end

    resize!(buffer, col + 1)
    buffer[end] = '\n' % UInt8
    write(io, buffer)
end

function random_fasta(io, flookup, n, seed=42%Int32)
    buffer = Vector{UInt8}(undef, LINEWIDTH + 1)

    for k=n:-LINEWIDTH:1
        m = min(k, LINEWIDTH)
        resize!(buffer, m + 1)

        for i=1:m
            seed = gen_random(seed)
            @inbounds buffer[i] = flookup(seed)
        end

        @inbounds buffer[end] = '\n' % UInt8
        write(io, buffer)
    end
    seed
end

function perf_fasta(n=25000000, io=stdout)
  write(io, ">ONE Homo sapiens alu\n")
  repeat_fasta(io, alu, 2n)

  write(io, ">TWO IUB ambiguity codes\n")
  seed = random_fasta(io, iub, 3n)
  write(io, ">THREE Homo sapiens frequency\n")
  random_fasta(io, homosapiens, 5n, seed)
end

isinteractive() || perf_fasta(parse(Int,ARGS[1]))
