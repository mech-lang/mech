/* The Computer Language Benchmarks Game
 * https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
 *
 * contributed by Diogo Lima
 * *reset*
*/

import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.TimeUnit;

public class binarytrees {

    /**
     * Internal class for structuring nodes and childs
     */
    private static class TreeNode {

        private TreeNode left;
        private TreeNode right;

        private TreeNode(TreeNode i_left, 
                         TreeNode i_right) {
            this.left = i_left;
            this.right = i_right;
        }

        public static TreeNode createNode(int i_depth) {
            // Create bottom node with empty child nodes => depth = 0
            TreeNode bottomTree = new TreeNode(null, null);
            return i_depth == 0 ? 
                       bottomTree : 
                       createNode(i_depth,
                                  bottomTree);
        }

        private static TreeNode createNode(int i_depth, 
                                           TreeNode i_accumulator) {
            TreeNode accumulator = i_accumulator;
            if (i_depth > 0) {
                final int depth = i_depth - 1;
                accumulator.left = createNode(depth);
                accumulator.right = createNode(depth);
            }
            return accumulator;
        }

        public final int checkNode() {
            return left == null ? 
                       1 : 
                       left.checkNode() + right.checkNode() + 1;
        }
    }


    public static void main(String[] args) {
        int n = args.length > 0 ? Integer.parseInt(args[0]) : 0;

        // Make the main process parallel to improve recursive createNode
        int proc = Runtime.getRuntime().availableProcessors() * 2;
        ExecutorService service = Executors.newFixedThreadPool(proc);
        service.execute(() -> runBenchmark(n));
        service.shutdown();
        try {
            service.awaitTermination(120L, TimeUnit.SECONDS);
        } catch (InterruptedException e) {
        }
    }

    // Do the real work with createNode
    private static void runBenchmark(int n) {
        final int minimumDepth = 4;
        final int maximumDepth = Math.max(minimumDepth + 2, n);
        final int stretchDepth = maximumDepth + 1;

        TreeNode node = TreeNode.createNode(stretchDepth);
        final int checkNode = node.checkNode();

        System.out.println("stretch tree of depth " 
                                + (maximumDepth + 1) 
                                + "\t check: " 
                                + checkNode);

        TreeNode longLivedTree = TreeNode.createNode(maximumDepth);
        for (int depth = minimumDepth; depth <= maximumDepth; depth += 2) {
            final int iterations = 1 << (maximumDepth - depth + minimumDepth);
            int check = 0;

            for (int i = 1; i <= iterations; i++) {
                check += TreeNode.createNode(depth).checkNode();
            }
            System.out.println(iterations
                                + "\t trees of depth " 
                                + depth 
                                + "\t check: "
                                + check);
        }
        System.out.println("long lived tree of depth " 
                           + maximumDepth 
                           + "\t check: " 
                           + longLivedTree.checkNode());
    }
}
