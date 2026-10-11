module binarytrees_gc_instrumented;
// Adapted from Hanabi1224's D binarytrees: default GC, no GC tuning.
import std;

ulong allocations;
ulong visits;
ulong leafAllocations;
class Node
{
    Node left, right;
    this(int depth)
    {
        ++allocations;
        if (depth == 0) ++leafAllocations;
        if (depth > 0)
        {
            left = new Node(depth - 1);
            right = new Node(depth - 1);
        }
    }
    int check()
    {
        ++visits;
        return left is null ? 1 : 1 + left.check() + right.check();
    }
}

void main(string[] args)
{
    auto depth = max(6, to!int(args[1]));
    writeln("stretch tree of depth ", depth + 1, "\t check: ", (new Node(depth + 1)).check());
    auto tree = new Node(depth);
    foreach (d; iota(4, depth + 1, 2))
    {
        auto n = 1 << (depth - d + 4);
        int sum;
        foreach (i; 0 .. n)
            sum += (new Node(d)).check();
        writeln(n, "\t trees of depth ", d, "\t check: ", sum);
    }
    writeln("long lived tree of depth ", depth, "\t check: ", tree.check());
    ulong expected = (1UL << (depth + 2)) - 1 + (1UL << (depth + 1)) - 1;
    ulong expectedLeaves = (1UL << (depth + 1)) + (1UL << depth);
    foreach (d; iota(4, depth + 1, 2))
    {
        expected += (1UL << (depth - d + 4)) * ((1UL << (d + 1)) - 1);
        expectedLeaves += (1UL << (depth - d + 4)) * (1UL << d);
    }
    enforce(allocations == expected);
    enforce(visits == expected);
    enforce(leafAllocations == expectedLeaves);
}
