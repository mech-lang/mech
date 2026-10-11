/* The Computer Language Benchmarks Game
* https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
*
* Contributed by Jon Harrop
* Modified by Alex Mizrahi
* Modified by Andreas Schäfer
* Modified by aardsplat-guest
*  *reset*
*/

#include <iostream>
#include <algorithm>
#include <future>
#include <vector>

#include <boost/pool/object_pool.hpp>

constexpr int threads{16};

struct Node {
   Node *l,*r;
   Node() : l(0),r(0) {}
   Node(Node *l,Node *r) : l(l),r(r) {}
   int check() const
   {
      if (l)
         return l->check() + 1 + r->check();
      else return 1;
   }
};

using Node_pool = boost::object_pool<Node>;

Node *make(int d,Node_pool& pool)
{
   if (d==0)
      return pool.construct();
   return pool.construct(make(d-1,pool),make(d-1,pool));
}

int make_iteration(int from,int to,int d,bool thread)
{
   int c{0};
   if (thread) {
      std::vector<std::future<int>>futures{};
      for (int j=0; j<threads; ++j) {
         int span{(to-from+1)/threads};
         futures.emplace_back(std::async(std::launch::async,
            make_iteration,from+span*j,span+span*j,d,false));
      }
      for (auto& fti : futures) {
         c += fti.get();
      }
   }
   else {
      Node_pool pool;
      for (int i=from; i<=to; ++i) {
         Node *a = make(d,pool);
         c += a->check();
      }
   }
   return c;
}

int main(int argc,char *argv[])
{
   int min_depth = 4,
      max_depth = std::max(min_depth+2,
         (argc == 2 ? atoi(argv[1]) : 10)),
      stretch_depth = max_depth+1;

   {
      Node_pool pool;
      Node *c = make(stretch_depth,pool);
      std::cout << "stretch tree of depth " << stretch_depth << "\t "
         << "check: " << c->check() << std::endl;
   }

   Node_pool long_lived_pool;
   Node *long_lived_tree=make(max_depth,long_lived_pool);

   for (int d=min_depth; d<=max_depth; d+=2) {
      int iterations = 1 << (max_depth - d + min_depth);
      int   c=0;
      c = make_iteration(1,iterations,d,true);
      std::cout << iterations << "\t trees of depth " << d << "\t "
         << "check: " << c << std::endl;
   }

   std::cout << "long lived tree of depth " << max_depth << "\t "
      << "check: " << (long_lived_tree->check()) << "\n";

   return 0;
}
