/* The Computer Language Benchmarks game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   Contributed by Diogo
   + null safety 
*/

import 'dart:isolate';

void main(List<String> args) async {
  int n = int.parse(args.elementAt(0));
  var d = 4;

  /// Define min and max iterations
  var maxIter = (n <= 4)
      ? 4
      : (n % 2 == 0)
          ? n
          : n - 1;
  var maxIterId = (maxIter - 4) / 2;

  /// Variables for output control on flow
  var expectedId = 'stretch-tree';
  var iterResult = 0;

  /// Create map for storing data
  var results = Map<String, dynamic>();
  var status = Map<String, bool>();

  /// Set final status to finish loop of output
  status['finished'] = false;

  /// Create a port for dealing with results
  ReceivePort port = new ReceivePort();
  port.listen((dataReceived) {
    /// Data received will be a ResultTreeMessage
    /// Store data received from message
    ResultTreeMessage data = dataReceived as ResultTreeMessage;
    results[data.id] = data;
    status[data.id] = true;

    /// We will use this main isolate to print results
    /// Create event loop to check if ordered result has arrived

    var nextIdStatus = true;
    while (nextIdStatus == true) {
      if (status[expectedId] == true) {
        if (expectedId == 'stretch-tree') {
          /// Get stretch tree result and output
          var result = results['stretch-tree'] as ResultTreeMessage;
          int checkVal = result.check;
          print("stretch tree of depth ${n + 1}\t check: ${checkVal}");

          /// Change expectedId to first iter tree: 0-tree
          expectedId = "${iterResult}-tree";
        } else if (expectedId == 'long-lived-tree') {
          /// Get long lived tree result and output
          var result = results['long-lived-tree'] as ResultTreeMessage;
          int checkVal = result.check;
          print("long lived tree of depth ${n}\t check: ${checkVal}");

          /// Change expectedId to 'finished' to avoid errors
          expectedId = 'finished';
          nextIdStatus = false;

          /// Close port because we finished job
          port.close();
        } else {
          /// Get current iter result we are looking for
          var iterId = "${iterResult}-tree";
          if (status[iterId] == true) {
            /// If it is done
            /// Get iter trees result and output
            var result = results[iterId] as ResultTreeMessage;
            int niter = result.niter;
            int depth = result.depth;
            int check = result.check;
            print("${niter}\t trees of depth ${depth}\t check: ${check}");

            /// If we achieved maxIter than we go to long lived tree
            if (iterResult == maxIterId) {
              /// Reached end of iter go to long lived tree
              expectedId = "long-lived-tree";
            } else {
              /// Keep iterating trees
              /// Set expectedId to next integer
              iterResult += 1;
              expectedId = "${iterResult}-tree";
            }
          }
        }
      }

      /// Don't continue unless next result is already available
      nextIdStatus = false;
      if (status[expectedId] == true) {
        nextIdStatus = true;
      }
    }
  });

  /// Keep track of all isolates
  /// And start making isolates to create trees
  final isolates = <Isolate>[];

  /// Stretch Tree
  /// Uses depth of n + 1
  /// Keep id as 'stretch-tree'
  StartMessage stretchStartMessage =
      new StartMessage(n + 1, 'stretch-tree', false, port.sendPort, 0);
  status['stretch-tree'] = false;
  isolates.add(await Isolate.spawn(runIsoCheckTree, stretchStartMessage));

  /// Long Lived Tree
  /// Uses depth of n
  /// Keep id as 'long-lived-tree'
  StartMessage longLivedStartMessage =
      new StartMessage(n, 'long-lived-tree', true, port.sendPort, 0);
  status['long-lived-tree'] = false;
  isolates.add(await Isolate.spawn(runIsoCheckTree, longLivedStartMessage));

  /// Start iterating by steps of 2
  /// Begin at d (which is 4) and goes up to maxIter
  /// Id of each tree will be '${niter}-tree'
  d = 4;
  var iterId = 0;
  while (d <= n) {
    var idIter = "${iterId}-tree";
    StartMessage message = new StartMessage(d, idIter, false, port.sendPort, n);
    status[idIter] = false;
    isolates.add(await Isolate.spawn(runIsoIterationFor, message));
    iterId += 1;
    d += 2;
  }

  /// Port will receive messages as soon as they get ready and print them
}

/// Define a message for sending isolate data
class StartMessage {
  int depth;
  var id;
  bool keepTree;
  SendPort port;
  int n;
  StartMessage(this.depth, this.id, this.keepTree, this.port, this.n);
}

/// Define a message for receiving data
class ResultTreeMessage {
  var id;
  List? tree;
  int check;
  int depth;
  int niter;
  ResultTreeMessage(this.id, this.tree, this.check, this.depth, this.niter);
}

void runIsoCheckTree(StartMessage message) {
  /// Creating tree of depth n
  List tree = make(message.depth);

  /// Making check on that tree
  int checkVal = check(tree);
  if (message.keepTree == true) {
    /// Return value and tree if tree is kept - use Isolate Port for it
    message.port.send(
        new ResultTreeMessage(message.id, tree, checkVal, message.depth, 0));
  } else {
    /// Return only value if tree is not kept - use Isolate Port for it
    message.port.send(
        new ResultTreeMessage(message.id, null, checkVal, message.depth, 0));
  }
}

/// Get message containing depth and define number of trees to iterate over
void runIsoIterationFor(StartMessage message) {
  int niter = 1 << (message.n - message.depth + 4);
  int checkVal = 0;
  for (int i = 1; i <= niter; i++) {
    /// Create tree and check number of nodes
    final List tree = make(message.depth);
    final int c = check(tree);

    /// Add number of nodes to master value
    checkVal += c;
  }
  message.port.send(
      new ResultTreeMessage(message.id, null, checkVal, message.depth, niter));
}

/// Trees will be defined by Lists of Lists containing always 2 elements
/// make => Create recursive lists of 2 elements
List make(int depth) {
  List tree = List.filled(2, null);
  if (depth != 0) {
    tree[0] = make(depth - 1);
    tree[1] = make(depth - 1);
  }
  return tree;
}

/// check => Runs through lists of list and get number of items
int check(List node) {
  return (node[0] == null) ? 1 : 1 + check(node[0]) + check(node[1]);
}
