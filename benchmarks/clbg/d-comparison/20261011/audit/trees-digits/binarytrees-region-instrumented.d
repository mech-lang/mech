module binarytrees_region_instrumented;
// Independent strict D port of the canonical binarytrees work.
// Each node uses one equal-size allocation from Phobos Region!Mallocator.
import std;
import std.experimental.allocator.building_blocks.region : Region;
import std.experimental.allocator.mallocator : Mallocator;

ulong allocations;
ulong visits;
ulong leafAllocations;
Node*[void*] treeAddresses;
alias Pool = Region!Mallocator;
struct Node
{
    Node* left, right;
}

Node* make(int depth, ref Pool pool)
{
    auto node = cast(Node*) pool.allocate(Node.sizeof).ptr;
    enforce(node !is null);
    enforce(node !in treeAddresses);
    treeAddresses[node] = node;
    ++allocations;
    if (depth == 0) ++leafAllocations;
    node.left = depth > 0 ? make(depth - 1, pool) : null;
    node.right = depth > 0 ? make(depth - 1, pool) : null;
    return node;
}

int check(const Node* node)
{
    ++visits;
    return node.left is null ? 1 : 1 + check(node.left) + check(node.right);
}

void main(string[] args)
{
    auto depth = max(6, to!int(args[1]));
    {
        auto stretch = Pool(((size_t(1) << (depth + 2)) - 1) * Node.sizeof);
        writeln("stretch tree of depth ", depth + 1, "\t check: ", check(make(depth + 1, stretch)));
    }
    enforce(allocations == (1UL << (depth + 2)) - 1);
    enforce(leafAllocations == (1UL << (depth + 1)));
    enforce(visits == allocations);
    visits = 0;
    treeAddresses = null;
    allocations = 0;
    leafAllocations = 0;
    auto live = Pool(((size_t(1) << (depth + 1)) - 1) * Node.sizeof);
    auto tree = make(depth, live);
    enforce(allocations == (1UL << (depth + 1)) - 1);
    enforce(leafAllocations == (1UL << depth));
    treeAddresses = null;
    auto depths = iota(4, depth + 1, 2).array;
    auto sums = new int[depths.length];
    auto workers = new TaskPool(7);
    scope (exit) workers.finish();
    foreach (index, d; workers.parallel(depths, 1))
    {
        auto pool = Pool(((size_t(1) << (d + 1)) - 1) * Node.sizeof);
        auto n = 1 << (depth - d + 4);
        int sum;
        foreach (i; 0 .. n)
        {
            visits = 0;
            allocations = 0;
            leafAllocations = 0;
            sum += check(make(d, pool));
            enforce(allocations == (1UL << (d + 1)) - 1);
            enforce(leafAllocations == (1UL << d));
            enforce(visits == allocations);
            treeAddresses = null;
            pool.deallocateAll();
        }
        sums[index] = sum;
    }
    foreach (i, d; depths)
        writeln(1 << (depth - d + 4), "\t trees of depth ", d, "\t check: ", sums[i]);
    visits = 0;
    writeln("long lived tree of depth ", depth, "\t check: ", check(tree));
    enforce(visits == (1UL << (depth + 1)) - 1);
}
