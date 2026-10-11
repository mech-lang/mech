/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
  
   contributed by Isaac Gouy
   TreeNode borrowed from Andrey Filatkin's JavaScript program
*/

import 'dart:io';
import 'dart:isolate';

final nIsolates = Platform.numberOfProcessors;

void main(List<String> args) {
  final mainIsolate = ReceivePort();
  final n = (args.length > 0) ? int.parse(args[0]) : 6;

  final stretchDepth = n + 1;
  final check = itemCheck(bottomUpTree(stretchDepth));
  print('stretch tree of depth $stretchDepth\t check: $check');
  final longLivedTree = bottomUpTree(n);

  var i = nIsolates;
  while (i-- > 0) {
    Isolate.spawn(other, mainIsolate.sendPort);
  }
  final requests = <RequestReply>[];
  final replies = <RequestReply>[];
  for (var depth = 4; depth <= n; depth += 2) {
    final iterations = 1 << n - depth + 4;
    requests.add(RequestReply(iterations, depth));
  }
  var awaited = requests.length;

  mainIsolate.listen((dynamic reply) {
    if (reply is SendPort) {
      if (requests.isNotEmpty) {
        reply.send(requests.removeLast());
      }
    } else if (reply is RequestReply) {
      replies.add(reply);
      if (--awaited == 0) {
        replies.sort((a, b) => a.depth.compareTo(b.depth));
        for (var reply in replies) {
          print('${reply.iterations}\t ' +
              'trees of depth ${reply.depth}\t ' +
              'check: ${reply.check}');
        }
        print('long lived tree of depth $n\t ' +
            'check: ${itemCheck(longLivedTree)}');

        mainIsolate.close();
      }
    }
  });
}

void other(SendPort p) {
  final otherIsolate = ReceivePort();
  p.send(otherIsolate.sendPort);

  otherIsolate.listen((dynamic request) {
    if (request is RequestReply) {
      for (int i = 0; i < request.iterations; i++) {
        request.check += itemCheck(bottomUpTree(request.depth));
      }
    }
    p.send(request);
    p.send(otherIsolate.sendPort);
  });
}

class RequestReply {
  var iterations = 0, depth = 0, check = 0;
  RequestReply(this.iterations, this.depth);
}

class TreeNode {
  TreeNode? left, right;
  TreeNode(this.left, this.right);
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
