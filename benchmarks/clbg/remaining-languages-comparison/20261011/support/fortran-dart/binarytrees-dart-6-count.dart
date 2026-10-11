/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   transliterated from Andrey Filatkin's Node #6 program by Isaac Gouy
*/

void originalMain(List<String> args) {
  final maxDepth = (args.length > 0) ? int.parse(args[0]) : 6;

  final stretchDepth = maxDepth + 1;
  final check = itemCheck(bottomUpTree(stretchDepth));
  print("stretch tree of depth $stretchDepth\t check: $check");

  final longLivedTree = bottomUpTree(maxDepth);

  for (var depth = 4; depth <= maxDepth; depth += 2) {
    final iterations = 1 << maxDepth - depth + 4;
    work(iterations, depth);
  }

  print(
      "long lived tree of depth $maxDepth\t check: ${itemCheck(longLivedTree)}");
}

void work(int iterations, int depth) {
  var check = 0;
  for (int i = 0; i < iterations; i++) {
    check += itemCheck(bottomUpTree(depth));
  }
  print("${iterations}\t trees of depth $depth\t check: $check");
}

class TreeNode {
  TreeNode? left, right;
  TreeNode(this.left, this.right) {auditAllocations++;}
}

int itemCheck(TreeNode? node) {
  if (node == null || node.left == null) {
    return 1;
  } else {
    return 1 + itemCheck(node.left) + itemCheck(node.right);
  }
}

TreeNode bottomUpTree(int depth) {
  return depth > 0
      ? TreeNode(bottomUpTree(depth - 1), bottomUpTree(depth - 1))
      : TreeNode(null, null);
}

int auditAllocations=0;
int verifyUnique(TreeNode root) {
  final seen=Set<TreeNode>.identity();
  int visit(TreeNode node) {
    if (!seen.add(node)) throw StateError('Shared tree node');
    if ((node.left==null)!=(node.right==null)) throw StateError('Incomplete binary tree');
    return node.left==null ? 1 : 1+visit(node.left!)+visit(node.right!);
  }
  return visit(root);
}
void main(List<String> args) {
  const n=10;
  if(verifyUnique(bottomUpTree(n+1))!=4095) throw StateError('stretch');
  final longRoot=bottomUpTree(n);
  if(verifyUnique(longRoot)!=2047) throw StateError('long root');
  var checks=0;
  for(var depth=4;depth<=n;depth+=2) {
    final count=1 << (n-depth+4);
    for(var i=0;i<count;i++) {
      final tree=bottomUpTree(depth);
      if(verifyUnique(tree)!=(1 << (depth+1))-1) throw StateError('node count');
      checks+=itemCheck(tree);
    }
  }
  if(itemCheck(longRoot)!=2047) throw StateError('long root lost');
  if(auditAllocations!=135854 || checks!=129712) throw StateError('total allocation work');
  print('AUDIT_ALLOCATIONS $auditAllocations');
}
