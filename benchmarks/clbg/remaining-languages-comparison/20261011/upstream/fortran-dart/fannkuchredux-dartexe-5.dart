/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   transliterated from Andrey Filatkin's Node #5 program by Isaac Gouy
*/

import 'dart:io';
import 'dart:isolate';
import 'dart:math';
import 'dart:typed_data';

var fact = <int>[];

void fillFact(int n) {
  fact.add(1);
  for (var i = 1; i <= n; i++) {
    fact.add(fact[i - 1] * i);
  }
}

Int32List fannkuch(int n, int idxMin, int idxMax) {
  if (fact.isEmpty) {
    fillFact(n);
  }
  final p = Int32List(n);
  final pp = Int32List(n);
  final count = Int32List(n);

  // first permutation
  for (var i = 0; i < n; i++) {
    p[i] = i;
  }
  int idx = idxMin;
  for (var i = n - 1; i > 0; i--) {
    final d = div(idx, fact[i]);
    count[i] = d;
    idx = idx % fact[i];

    for (var j = 0; j < n; j++) {
      pp[j] = p[j];
    }

    for (var j = 0; j <= i; j++) {
      if (j + d <= i) {
        p[j] = pp[j + d];
      } else {
        p[j] = pp[j + d - i - 1];
      }
    }
  }

  var maxFlips = 1;
  var checkSum = 0;

  idx = idxMin;
  for (var sign = true;; sign = !sign) {
    var first = p[0];
    if (first != 0) {
      var flips = 1;
      if (p[first] != 0) {
        for (var j = 1; j < n; j++) {
          pp[j] = p[j];
        }
        var p0 = first;
        while (true) {
          flips++;
          var i = 1;
          var j = p0 - 1;
          while (i < j) {
            final t = pp[i];
            pp[i] = pp[j];
            pp[j] = t;
            i++;
            j--;
          }
          final t = pp[p0];
          pp[p0] = p0;
          p0 = t;
          if (pp[p0] == 0) {
            break;
          }
        }
      }

      if (maxFlips < flips) {
        maxFlips = flips;
      }
      if (sign) {
        checkSum += flips;
      } else {
        checkSum -= flips;
      }
    }

    idx++;
    if (idx == idxMax) {
      break;
    }

    // next permutation
    if (sign) {
      p[0] = p[1];
      p[1] = first;
    } else {
      final t = p[1];
      p[1] = p[2];
      p[2] = t;
      for (var k = 2;; k++) {
        count[k]++;
        if (count[k] <= k) {
          break;
        }
        count[k] = 0;
        for (var j = 0; j <= k; j++) {
          p[j] = p[j + 1];
        }
        p[k + 1] = first;
        first = p[0];
      }
    }
  }
  // Fixed-length with type.
  return Int32List.fromList([maxFlips, checkSum]);
}

int div(int val, int by) {
  return (val - val % by) ~/ by;
}

void main(List<String> args) {
  final mainIsolate = ReceivePort();
  var i = Platform.numberOfProcessors;
  while (i-- > 0) {
    Isolate.spawn(other, mainIsolate.sendPort);
  }

  final n = (args.length > 0) ? int.parse(args[0]) : 7;
  fillFact(n);

  const nchunks = 720;
  var chunkSize = div((fact[n] + nchunks - 1), nchunks);
  chunkSize += chunkSize % 2;

  final requests = <Request>[];
  final len = div((fact[n] + chunkSize - 1), chunkSize);
  for (var i = 0; i < len; i++) {
    final idxMin = chunkSize * i;
    final idxMax = min(fact[n], idxMin + chunkSize);
    requests.add(Request(n, idxMin, idxMax));
  }
  var awaited = requests.length;
  var flips = 0, chk = 0;

  mainIsolate.listen((dynamic p) {
    if (p is SendPort) {
      if (requests.isNotEmpty) {
        p.send(requests.removeLast());
      }
    } else if (p is Int32List) {
      if (flips < p.first) {
        flips = p.first;
      }
      chk += p.last;

      if (--awaited == 0) {
        print("$chk\nPfannkuchen($n) = $flips");
        mainIsolate.close();
      }
    }
  });
}

void other(SendPort p) {
  final otherIsolate = ReceivePort();
  p.send(otherIsolate.sendPort);

  otherIsolate.listen((dynamic ini) {
    if (ini is Request) {
      p.send(fannkuch(ini.n, ini.idxMin, ini.idxMax));
      p.send(otherIsolate.sendPort);
    }
  });
}

class Request {
  int n = 0, idxMin = 0, idxMax = 0;
  Request(this.n, this.idxMin, this.idxMax);
}
