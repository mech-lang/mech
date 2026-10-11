/* The Computer Language Benchmarks Game
 * https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
 *
 * contributed by Jon Harrop
 * modified by Alex Mizrahi
 * modified by Andreas Schäfer
 * very minor omp tweak by The Anh Tran
 * modified by Maxime Coste
 * "tree nodes all the way down-to depth 0" by Isaac Gouy
 *  *reset*
 */

#include <iostream>
#include <stdlib.h>
#include <stdio.h>
#include <omp.h>

#include <boost/pool/object_pool.hpp>


const size_t   LINE_SIZE = 64;


struct Node 
{
    Node *l, *r;

    int check() const { return l ? l->check() + 1 + r->check() : 1; }
};

typedef boost::object_pool<Node> NodePool;


Node *make(int d, NodePool &store) 
{
//    return (d == 0) ? nullptr : store.construct(Node{make(d-1, store), make(d-1, store)});

    return (d == 0) 
        ? store.construct(Node{nullptr, nullptr})
        : store.construct(Node{make(d-1, store), make(d-1, store)});
}

int GetThreadCount()
{
   cpu_set_t cs;
   CPU_ZERO(&cs);
   sched_getaffinity(0, sizeof(cs), &cs);

   int count = 0;
   for (int i = 0; i < 8; i++)
   {
      if (CPU_ISSET(i, &cs))
         count++;
   }
   return count;
}

int main(int argc, char *argv[]) 
{
    int min_depth = 4;
    int max_depth = std::max(min_depth+2,
                             (argc == 2 ? atoi(argv[1]) : 10));
    int stretch_depth = max_depth+1;

   // Alloc then dealloc stretchdepth tree
    {
        NodePool store;
        Node *c = make(stretch_depth, store);
        std::cout << "stretch tree of depth " << stretch_depth << "\t "
                  << "check: " << c->check() << std::endl;
    }

    NodePool long_lived_store;
    Node *long_lived_tree = make(max_depth, long_lived_store);

   // buffer to store output of each thread
   char *outputstr = (char*)malloc(LINE_SIZE * (max_depth +1) * sizeof(char));

   #pragma omp parallel for default(shared) num_threads(GetThreadCount()) schedule(dynamic, 1)
    for (int d = min_depth; d <= max_depth; d += 2) 
    {
        int iterations = 1 << (max_depth - d + min_depth);
        int c = 0;

        for (int i = 1; i <= iterations; ++i) 
        {
            NodePool store;
            Node *a = make(d, store);
            c += a->check();
        }

      // each thread write to separate location
      sprintf(outputstr + LINE_SIZE * d, "%d\t trees of depth %d\t check: %d\n", iterations, d, c);
   }

   // print all results
   for (int d = min_depth; d <= max_depth; d += 2) 
      printf("%s", outputstr + (d * LINE_SIZE) );
   free(outputstr);

    std::cout << "long lived tree of depth " << max_depth << "\t "
              << "check: " << (long_lived_tree->check()) << "\n";

    return 0;
}
