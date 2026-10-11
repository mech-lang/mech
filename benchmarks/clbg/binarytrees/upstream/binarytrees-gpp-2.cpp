/* The Computer Language Benchmarks Game 
 * https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
 *
 * Contributed by Jon Harrop
 * Modified by Alex Mizrahi
 *  *reset*
 */

#include <stdio.h>
#include <stdlib.h>
#include <iostream>

struct Node {
  Node *l, *r;
  Node() : l(0), r(0) {}
  Node(Node *l2, Node *r2) : l(l2), r(r2) {}
  ~Node() { delete l; delete r; }
  int check() const {
	  if (l)
		return l->check() + 1 + r->check(); 
	  else return 1;
  }
};

Node *make(int d) {
  if (d == 0) return new Node();
  return new Node(make(d-1), make(d-1));
}

int main(int argc, char *argv[]) {
  int min_depth = 4,
    max_depth = std::max(min_depth+2,
			 (argc == 2 ? atoi(argv[1]) : 10)),
    stretch_depth = max_depth+1;

  {
    Node *c = make(stretch_depth);
    std::cout << "stretch tree of depth " << stretch_depth << "\t "
      << "check: " << c->check() << std::endl;
    delete c;
  }

  Node *long_lived_tree=make(max_depth);

  for (int d=min_depth; d<=max_depth; d+=2) {
    int iterations = 1 << (max_depth - d + min_depth), c=0;
    for (int i=1; i<=iterations; ++i) {
      Node *a = make(d);
      c += a->check();
      delete a;
    }
    std::cout << iterations << "\t trees of depth " << d << "\t "
	      << "check: " << c << std::endl;
  }

  std::cout << "long lived tree of depth " << max_depth << "\t "
	    << "check: " << (long_lived_tree->check()) << "\n";

  delete long_lived_tree;

  return 0;
}
