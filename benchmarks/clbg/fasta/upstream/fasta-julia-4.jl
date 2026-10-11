# The Computer Language Benchmarks Game
# https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
#
# Contributed by Jens Adam
# using ideas from Julia#2, Julia#3, Go#3

struct Aminoacids
    c::UInt8
    p::Int32
end

const OUT = stdout
const LINE_LENGTH = 60

const IM = Int32(139968)
const IA = Int32(3877)
const IC = Int32(29573)
const last_rnd = Ref(Int32(42))
gen_random() = (last_rnd[] = (last_rnd[] * IA + IC) % IM)

function random_char(genelist)
    r = gen_random()
    for aminoacid in genelist
        aminoacid.p >= r && return aminoacid.c
    end
    return genelist[end].c
end

function fillrand!(line, genelist, n)
    for i in 1:n
        @inbounds line[i] = random_char(genelist)
    end
end

function random_fasta(genelist, n)
    line = Vector{UInt8}(undef, LINE_LENGTH+1)
    line[end] = UInt8('\n')
    while n > LINE_LENGTH
        fillrand!(line, genelist, LINE_LENGTH)
        write(OUT, line)
        n -= LINE_LENGTH
    end
    fillrand!(line, genelist, n)
    line[n+1] = UInt8('\n')
    write(OUT, @view line[1:n+1])
end

function repeat_fasta(str, n)
    len = length(str)
    # create a string with the beginning repeated at the end
    # so we don't have to wrap around
    src = Vector{UInt8}(undef, len + LINE_LENGTH)
    for i in 1:len
        @inbounds src[i] = str[i]
    end
    for i in 1:LINE_LENGTH
        @inbounds src[i+len] = str[i]
    end

    i = 1
    lines, rest = divrem(n, LINE_LENGTH)
    for _ in 1:lines
        write(OUT, @inbounds @view src[i:i+LINE_LENGTH-1])
        write(OUT, '\n')
        i += LINE_LENGTH
        i > len && (i -= len)
    end
    write(OUT, @inbounds @view src[i:i+rest-1])
    write(OUT, '\n')
end

function make_Aminoacids(cs, ps)
    cum_p = 0.0
    tmp = Aminoacids[]
    for (c, p) in zip(cs, ps)
        cum_p += p * IM
        # the comparison is with Int32, so use it here as well
        push!(tmp, Aminoacids(c, floor(Int32, cum_p)))
    end
    return (tmp...,)
end

# create Aminoacids with accumulated probabilities and make
# the result a constant
const IUB = let
    iub_c = b"acgtBDHKMNRSVWY"
    iub_p = [0.27, 0.12, 0.12, 0.27, 0.02, 0.02, 0.02,
             0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02]
    make_Aminoacids(iub_c, iub_p)
end
const HOMOSAPIENS = let
    homosapiens_c = b"acgt"
    homosapiens_p = [0.3029549426680, 0.1979883004921,
                     0.1975473066391, 0.3015094502008]
    make_Aminoacids(homosapiens_c, homosapiens_p)
end
const ALU = codeunits(
    "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGG" *
    "GAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGA" *
    "CCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAAT" *
    "ACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCA" *
    "GCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGG" *
    "AGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCC" *
    "AGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA")

function main(n)
    write(OUT, ">ONE Homo sapiens alu\n")
    repeat_fasta(ALU, 2n)
    write(OUT, ">TWO IUB ambiguity codes\n")
    random_fasta(IUB, 3n)
    write(OUT, ">THREE Homo sapiens frequency\n")
    random_fasta(HOMOSAPIENS, 5n)
end

main(parse(Int, ARGS[1]))
