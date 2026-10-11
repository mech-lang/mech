// The Computer Language Benchmarks Game
// https://benchmarksgame-team.pages.debian.net/benchmarksgame/
//
// based on
// - Tristan Dupont Java #7
// - Anthony Lloyd C#
// - Isaac Guoy C# #7
// - Steve (hez2010) https://github.com/dotnet/runtime/issues/101006#issue-2241502984
// contributed by Arseniy Zlobintsev

not accepted kind-of interesting

using System;
using System.Linq;
using System.Threading.Tasks;

const int minDepth = 4;

var workers = Environment.ProcessorCount;
var maxDepth = args.Length is 0 ? 10
    : Math.Max(minDepth + 2, int.Parse(args[0]));

Console.WriteLine(
    $"stretch tree of depth {maxDepth + 1}\t check: {new TreeNode(maxDepth + 1).Check()}");

var longLivedTree = new TreeNode(maxDepth);

var results = new string[((maxDepth - minDepth) / 2) + 1];
for (var i = 0; i < results.Length; i++) {
    var depth = (i * 2) + minDepth;
    var n = (1 << (maxDepth - depth + minDepth)) / workers;
    var checks = new int[workers];
    Parallel.For(0, workers, w => {
        var check = 0;
        for (var i = n; i > 0; i--)
            check += new TreeNode(depth).Check();
        checks[w] = check;
    });
    results[i] = $"{n * workers}\t trees of depth {depth}\t check: {checks.Sum()}";
}

foreach (var r in results)
    Console.WriteLine(r);

Console.WriteLine(
    $"long lived tree of depth {maxDepth}\t check: {longLivedTree.Check()}");

readonly struct TreeNode(int depth) {
    record Next(TreeNode Left, TreeNode Right);
    readonly Next? next = depth > 0
        ? new(new(depth - 1), new(depth - 1))
        : null;

    public int Check() {
        return next is null ? 1 : 1 + next.Left.Check() + next.Right.Check();
    }
}
