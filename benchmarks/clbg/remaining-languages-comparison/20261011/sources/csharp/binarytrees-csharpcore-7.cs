// The Computer Language Benchmarks Game
// https://benchmarksgame-team.pages.debian.net/benchmarksgame/
//
// based Tristan Dupont Java #7 and Anthony Lloyd C# 
// contributed by Isaac Gouy

using System;
using System.Threading.Tasks;

class BinaryTrees
{
    const int MinDepth = 4;
    const int NoTasks = 4;

    public static void Main(string[] args)
    {
        int maxDepth = args.Length == 0 ? 10
            : Math.Max(MinDepth + 2, int.Parse(args[0]));

        Console.WriteLine(string.Concat("stretch tree of depth ", maxDepth + 1,
            "\t check: ", (bottomUpTree(maxDepth + 1)).itemCheck()));

        var longLivedTree = bottomUpTree(maxDepth);

        var results = new string[(maxDepth - MinDepth) / 2 + 1];

        for (int i = 0; i < results.Length; i++)
        {
            int depth = i * 2 + MinDepth;
            int n = (1 << maxDepth - depth + MinDepth) / NoTasks;
            var tasks = new Task<int>[NoTasks];
            for (int t = 0; t < tasks.Length; t++)
            {
                tasks[t] = Task.Run(() =>
                {
                    var check = 0;
                    for (int i = n; i > 0; i--)
                        check += (bottomUpTree(depth)).itemCheck();
                    return check;
                });
            }
            var check = tasks[0].Result;
            for (int t = 1; t < tasks.Length; t++)
                check += tasks[t].Result;
            results[i] = string.Concat(n * NoTasks, "\t trees of depth ",
                depth, "\t check: ", check);
        }
        for (int i = 0; i < results.Length; i++)
            Console.WriteLine(results[i]);

        Console.WriteLine(string.Concat("long lived tree of depth ", maxDepth,
            "\t check: ", longLivedTree.itemCheck()));
    }
    
    static TreeNode bottomUpTree(int depth) 
    {
        if (0 < depth) {
            return new TreeNode(bottomUpTree(depth - 1), bottomUpTree(depth - 1));
        }
        return new TreeNode(null,null);
    }

    private class TreeNode
    {
        readonly TreeNode left, right;
        
        internal TreeNode(TreeNode left, TreeNode right) 
        {
            this.left = left;
            this.right = right;
        }     

        internal int itemCheck()
        {
            if (null == left) {
                return 1;
            }
            return 1 + left.itemCheck() + right.itemCheck();
        }
    }   
}
