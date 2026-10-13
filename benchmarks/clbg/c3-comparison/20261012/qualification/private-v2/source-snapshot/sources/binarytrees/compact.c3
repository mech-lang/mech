module clbg::binarytrees;
import libc;
import std::math;

struct Node
{
    Node* left;
    Node* right;
}

fn Node* make(int depth)
{
    Node* node = libc::calloc(1, Node::size);
    if (!node)
    {
        libc::exit(2);
    }
    if (depth > 0)
    {
        node.left = make(depth - 1);
        node.right = make(depth - 1);
    }
    return node;
}

fn long check(Node* node)
{
    long result = 1;
    if (node.left)
    {
        result += check(node.left) + check(node.right);
    }
    return result;
}

fn void destroy(Node* node)
{
    if (node.left)
    {
        destroy(node.left);
        destroy(node.right);
    }
    libc::free(node);
}

fn int main(int argc, char** argv)
{
    int depth = math::max(6, (int)libc::atoll(argv[1]));
    Node* stretch = make(depth + 1);
    libc::printf("stretch tree of depth %d\t check: %lld\n", depth + 1, check(stretch));
    destroy(stretch);
    Node* long_lived = make(depth);
    for (int current = 4; current <= depth; current += 2)
    {
        long iterations = 1L << (depth - current + 4);
        long total = 0;
        for (long iteration = 0; iteration < iterations; iteration++)
        {
            Node* tree = make(current);
            total += check(tree);
            destroy(tree);
        }
        libc::printf("%lld\t trees of depth %d\t check: %lld\n", iterations, current, total);
    }
    libc::printf("long lived tree of depth %d\t check: %lld\n", depth, check(long_lived));
    destroy(long_lived);
    return 0;
}
