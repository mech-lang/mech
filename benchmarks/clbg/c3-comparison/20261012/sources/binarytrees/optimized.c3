module clbg::binarytrees;
import libc;
import std::math;
import clbg::threads;

struct Node
{
    Node* left;
    Node* right;
}

struct Job
{
    int depth;
    long first;
    long last;
    long total;
}

fn Node* make(int depth)
{
    Node* node = libc::malloc(Node::size);
    if (!node)
    {
        libc::exit(2);
    }
    node.left = null;
    node.right = null;
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

fn int trees(void* argument)
{
    Job* job = argument;
    long total = 0;
    for (long iteration = job.first; iteration < job.last; iteration++)
    {
        Node* tree = make(job.depth);
        total += check(tree);
        destroy(tree);
    }
    job.total = total;
    return 0;
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
        int count = (int)math::min(8L, iterations);
        threads::Handle[8] workers;
        Job[8] jobs;
        for (int worker = 0; worker < count; worker++)
        {
            jobs[worker] = { current, iterations * worker / count, iterations * (worker + 1) / count, 0 };
            threads::start(&workers[worker], &trees, &jobs[worker]);
        }
        for (int worker = 0; worker < count; worker++)
        {
            threads::join(workers[worker]);
        }
        long total = 0;
        for (int worker = 0; worker < count; worker++)
        {
            total += jobs[worker].total;
        }
        libc::printf("%lld\t trees of depth %d\t check: %lld\n", iterations, current, total);
    }
    libc::printf("long lived tree of depth %d\t check: %lld\n", depth, check(long_lived));
    destroy(long_lived);
    return 0;
}
