/**
 * The Computer Language Benchmarks Game
 * https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
 *
 * Loosely based on Jarkko Miettinen's implementation. Requires Java 8.
 *
 * contributed by Heikki Salokanto.
 * modified by Chandra Sekar
 * modified by Mike Krüger
 * *reset*
 */

public class binarytrees {
    public static void main(String[] args) throws Exception {
        int n = args.length > 0 ? Integer.parseInt(args[0]) : 0;
        int minDepth = 4;
        int maxDepth = Math.max(minDepth + 2, n);
        int stretchDepth = maxDepth + 1;
        int check = (TreeNode.create(stretchDepth)).check();
        
        System.out.println("stretch tree of depth " + (maxDepth + 1) + "\t check: " + check);

        TreeNode longLivedTree = TreeNode.create(maxDepth);
        for (int depth = minDepth; depth <= maxDepth; depth += 2)
        {
           int iterations = 1 << (maxDepth - depth + minDepth);
           check = 0;

           for (int i = 1; i <= iterations; i++)
           {
                check += (TreeNode.create(depth)).check();
           }
           System.out.println(iterations + "\t trees of depth " + depth + "\t check: " + check);
        }

        System.out.println("long lived tree of depth " + maxDepth + "\t check: " + longLivedTree.check());
    }

    static class TreeNode {
        TreeNode left, right;

        static TreeNode create(int depth)
        {
            return ChildTreeNodes(depth);
        }
         
        static TreeNode ChildTreeNodes(int depth)
        {
            TreeNode node = new TreeNode();
            if (depth > 0)
            {
                node.left = ChildTreeNodes(depth - 1);
                node.right = ChildTreeNodes(depth - 1);
            }
            return node;
        }

        int check() {
            return left == null ? 1 : left.check() + right.check() + 1;
        }
    }
}
