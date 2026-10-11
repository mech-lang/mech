// Independent strict D port of the canonical binarytrees work.
// Each node uses one equal-size allocation from Phobos Region!Mallocator.
import std;
import std.experimental.allocator.building_blocks.region : Region;
import std.experimental.allocator.mallocator : Mallocator;

alias Pool = Region!Mallocator;
struct Node
{
    Node* left, right;
}

Node* make(int depth, ref Pool pool)
{
    auto node = cast(Node*) pool.allocate(Node.sizeof).ptr;
    enforce(node !is null);
    node.left = depth > 0 ? make(depth - 1, pool) : null;
    node.right = depth > 0 ? make(depth - 1, pool) : null;
    return node;
}

int check(const Node* node)
{
    return node.left is null ? 1 : 1 + check(node.left) + check(node.right);
}

void main(string[] args)
{
    auto depth = max(6, to!int(args[1]));
    {
        auto stretch = Pool(((size_t(1) << (depth + 2)) - 1) * Node.sizeof);
        writeln("stretch tree of depth ", depth + 1, "\t check: ", check(make(depth + 1, stretch)));
    }
    auto live = Pool(((size_t(1) << (depth + 1)) - 1) * Node.sizeof);
    auto tree = make(depth, live);
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
            sum += check(make(d, pool));
            pool.deallocateAll();
        }
        sums[index] = sum;
    }
    foreach (i, d; depths)
        writeln(1 << (depth - d + 4), "\t trees of depth ", d, "\t check: ", sums[i]);
    writeln("long lived tree of depth ", depth, "\t check: ", check(tree));
}
