# The Computer Language Benchmarks Game
# https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

# based on Oleg Mazurov's Java Implementation and Jeremy Zerfas' C implementation
# transliterated and modified by Hamza Yusuf Çakır.
# Updated for speed/succinctness by Adam Beckmeyer.

import Base: @propagate_inbounds
import .Threads: nthreads, threadid, @threads

const NUM_BLOCKS = 20

struct Perm
    # p holds the current permutation
    p::Vector{Int8}
    # p is copied to pp as a workspace for the actual flipping
    pp::Vector{Int8}
    # count is a tracker for determining the next permutation of p
    count::Vector{Int32}
end

# Perm is already ordered when passed to first_permutation!
Perm(n) = Perm(collect(Int8, 1:n), collect(Int8, 1:n), zeros(Int32, n))

@propagate_inbounds function first_permutation!(perm, idx)
    p = perm.p; pp = perm.pp

    for i=length(p):-1:2
        ifact = factorial(i - 1)
        d = idx ÷ ifact
        perm.count[i] = d
        idx = idx % ifact

        # Rotate the first i elements of p by d elements to the left
        # using pp as a temporary buffer
        unsafe_copyto!(pp, 1, p, 1, i)
        unsafe_copyto!(p, 1, pp, d + 1, i - d)
        unsafe_copyto!(p, i - d + 1, pp, 1, d)
    end
end

@propagate_inbounds function next_permutation!(perm)
    p = perm.p; count = perm.count

    p[1], p[2] = p[2], p[1]

    i = 2
    while count[i] >= i - 1
        count[i] = 0

        tmp = p[1]
        for j=1:i
            p[j] = p[j+1]
        end
        i += 1
        p[i] = tmp
    end
    count[i] += 1
end

@propagate_inbounds function count_flips(perm)
    p = perm.p; pp = perm.pp
    # count_flips is only called if the first element isn't already a
    # 1, so we know at least one flip is required.
    flips = 1

    first_value = p[1]
    if p[first_value] != 1
        # pp will be working copy. Don't have to copy first value as
        # it's stored in first_value var
        unsafe_copyto!(pp, 2, p, 2, length(p) - 1)

        # If the next flip would result in 0 being in first position,
        # iteration can stop without doing the flip
        while (new_first_value = pp[first_value]) != 1
            flips += 1
            # If only 2 or 3 elements flipped, a swap is all that's needed
            pp[first_value] = first_value
            # If first_value is greater than 3, more flips are needed
            if first_value > 3
                l = 2; r = first_value - 1
                # In total, first_value ÷ 2 swaps must occur, but 1 is
                # already finished. Use 12 explicit iterations here
                # instead of a while-loop to hint the compiler towards
                # unrolling. This means that this program is not
                # correct for n > 27.
                for _=1:12
                    pp[l], pp[r] = pp[r], pp[l]
                    l += 1
                    r -= 1
                    l < r || break
                end
            end
            first_value = new_first_value
        end
    end

    flips
end

@propagate_inbounds function run_task!(n, idxmin, idxmax)
    perm = Perm(n)
    first_permutation!(perm, idxmin)

    maxflips = chksum = 0

    for i=idxmin:idxmax
        if perm.p[1] != 1
            flips = count_flips(perm)
            maxflips = max(flips, maxflips)
            chksum += iseven(i) ? flips : -flips
        end
        next_permutation!(perm)
    end

    maxflips, chksum
end

function fannkuchredux(n)
    factn = factorial(n)
    blocksz = factn ÷ (factn < NUM_BLOCKS ? 1 : NUM_BLOCKS)

    maxflips = zeros(Int, nthreads())
    chksums = zeros(Int, nthreads())

    @threads for idxmin=0:blocksz:factn-1
        idxmax = idxmin + blocksz - 1
        task_maxflips, chksum = @inbounds run_task!(n, idxmin, idxmax)

        id = threadid()
        maxflips[id] = max(task_maxflips, maxflips[id])
        chksums[id] += chksum
    end

    # reduce results
    chk = sum(chksums)
    res = maximum(maxflips)

    println(chk, "\nPfannkuchen(", n, ") = ", res)
end

isinteractive() || fannkuchredux(parse(Int, ARGS[1]))
