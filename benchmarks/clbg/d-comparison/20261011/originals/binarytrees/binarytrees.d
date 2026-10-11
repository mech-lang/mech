// Original author: Akira1364
// Year: 2020
// License: MIT

import std.conv, std.parallelism, std.range, std.stdio, pooledmm;

alias TNodePool = TNonFreePooledMemManager!(TNode);

struct TDataRec {
  ubyte depth;
  int iterations, check;
}

struct TNode {
  TNode* left, right;

  pragma(inline, true) pure nothrow @nogc static int checkNode(const TNode* node) {
    if (node.right != null && node.left != null)
      return 1 + checkNode(node.right) + checkNode(node.left);
    return 1;
  }

  pragma(inline, true) nothrow @nogc static TNode* makeTree(const int depth, TNodePool* mp) {
    TNode* result = mp.newItem();
    if (depth > 0) {
      result.right = makeTree(depth - 1, mp);
      result.left = makeTree(depth - 1, mp);
    }
    return result;
  }
}

static immutable ubyte mindepth = 4;

static TDataRec[9] data;

void main(in string[] args) {
  immutable ubyte maxdepth = args.length > 1 ? to!(ubyte)(args[1]) : 10;

  // Create and destroy a tree of depth `maxdepth + 1`.
  TNodePool pool;
  writeln("stretch tree of depth ", maxdepth + 1, "\t check: ",
          TNode.checkNode(TNode.makeTree(maxdepth + 1, &pool)));
  pool.clear();

  // Create a "long lived" tree of depth `maxdepth`.
  TNode* tree = TNode.makeTree(maxdepth, &pool);

  // While the tree stays live, create multiple trees. Local data is stored in
  // the `data` variable.
  immutable ubyte highindex = cast(ubyte)((maxdepth - mindepth) / 2 + 1);
  auto slice = data[0 .. highindex];
  foreach (i, ref item; taskPool().parallel(slice, 1)) {
    item.depth = cast(ubyte)(mindepth + i * 2);
    item.iterations = 1 << (maxdepth - i * 2);
    item.check = 0;
    TNodePool ipool;
    for (int j = 1; j <= item.iterations; ++j) {
      item.check += TNode.checkNode(TNode.makeTree(item.depth, &ipool));
      ipool.clear();
    }
  }

  // Display the results.
  foreach (i, ref item; slice) {
    writeln(item.iterations, "\t trees of depth ", item.depth, "\t check: ", item.check);
  }

  // Check and destroy the long lived tree.
  writeln("long lived tree of depth ", maxdepth, "\t check: ", TNode.checkNode(tree));
  pool.clear();
}
