/* The Computer Language Benchmarks game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   Contributed by Aldo Román 
*/

import "dart:async" show Completer;
import "dart:io" show exit;
import "dart:isolate" show Isolate, SendPort, RawReceivePort;

const int numberOfProcessors = 4;
const int minDepth = 4;

class TreeNode {
  final TreeNode? left, right;
  const TreeNode(this.left, this.right);
}

TreeNode buildTree(int depth) => 0 < depth
    ? TreeNode(buildTree(depth - 1), buildTree(depth - 1))
    : TreeNode(null, null);

int itemCheck(TreeNode node) {
  if (null == node.left) {
    return 1;
  } else {
    return 1 + itemCheck(node.left!) + itemCheck(node.right!);
  }
}

void _isolate(SendPort parentPort) {
  final port = RawReceivePort((List params) {
    final int start = params[0];
    final int end = params[1];
    final int maxDepth = params[2];
    final SendPort resultPort = params[3];

    final String result =
        Iterable.generate(end - start, (i) => i + start).map((i) {
      final int depth = i * 2 + minDepth;
      final int iterations = 1 << (maxDepth - depth + minDepth);
      final int check = Iterable.generate(iterations)
          .fold(0, (acc, _) => acc + itemCheck(buildTree(depth)));
      return "${iterations}\t trees of depth $depth\t check: $check";
    }).join("\n");

    resultPort.send(result);
  });

  parentPort.send(port.sendPort);
}

Future<String> spawnWorker(int start, int end, int maxDepth) {
  final Completer<String> c = Completer.sync();
  final resultPort = RawReceivePort(c.complete);
  final initPort = RawReceivePort((SendPort sendPort) {
    sendPort.send([start, end, maxDepth, resultPort.sendPort]);
  });
  Isolate.spawn(_isolate, initPort.sendPort);
  return c.future;
}

void main(List<String> args) {
  final int n = args.length > 0 ? int.parse(args[0], radix: 10) : 0;

  final int maxDepth = (minDepth + 2 > n) ? minDepth + 2 : n;

  final int stretchDepth = maxDepth + 1;
  final int check = itemCheck(buildTree(stretchDepth));
  print("stretch tree of depth $stretchDepth\t check: $check");

  final TreeNode longLivedTree = buildTree(maxDepth);

  final int totalTasks = (maxDepth - minDepth) ~/ 2 + 1;
  final int numberOfWorkers =
      totalTasks > numberOfProcessors ? numberOfProcessors : totalTasks;
  final int tasksPerWorker = totalTasks ~/ numberOfWorkers;
  final int offset = totalTasks % numberOfWorkers;

  final tasks = Iterable.generate(numberOfWorkers, (i) {
    final int start = i * tasksPerWorker + ((i == 0) ? 0 : offset);
    final int end = start + tasksPerWorker + ((i == 0) ? offset : 0);
    return spawnWorker(start, end, maxDepth);
  });

  Future.wait(tasks)
      .then((resultsPerWorker) => resultsPerWorker.join("\n"))
      .then(print)
      .then((_) {
    print(
        "long lived tree of depth $maxDepth\t check: ${itemCheck(longLivedTree)}");
    exit(0);
  });
}
