# The Computer Language Benchmarks Game
# https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
#
# Contributed by Adam Beckmeyer. Based on code by Jarret Revels, Alex
# Arslan, Michal Stransky, Jens Adam.

using Distributed

Distributed.addprocs()

@everywhere struct Node
    leaf::Bool
    l::Node
    r::Node
    Node(l, r) = new(false, l, r)
    Node() = new(true)
end

@everywhere make(n) = n === 0 ? Node() : Node(make(n-1), make(n-1))

@everywhere check(node) =
    node.leaf ? 1 : 1 + check(node.l) + check(node.r)

function binary_trees(io, n)
    write(io, "stretch tree of depth $(n+1)\t check: $(check(make(n+1)))\n")

    long_tree = make(n)

    d = 4
    while d <= n
        niter = 1 << (n - d + 4)
        c = @distributed (+) for _ in 1:niter
            check(make(d))
        end
        write(io, "$niter\t trees of depth $d\t check: $c\n")
        d += 2
    end

    write(io, "long lived tree of depth $n\t check: $(check(long_tree))\n")
end

isinteractive() || binary_trees(stdout, parse(Int, ARGS[1]))
