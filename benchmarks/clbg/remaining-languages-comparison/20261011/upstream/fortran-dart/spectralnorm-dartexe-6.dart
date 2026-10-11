/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   contributed by Isaac Gouy
   (Span.initialize from Andrey Filatkin's JavaScript #6 program)
*/

import 'dart:math';
import 'dart:typed_data';
import 'dart:isolate';
import 'dart:async';
import 'dart:io';

double a(int i, int j) {
  return 1.0 / (((i + j) * (i + j + 1) >> 1) + i + 1);
}

Float64List auFromTo(int start, int end, Float64List u) {
  final n = u.length;
  final w = new Float64List(n);
  for (int i = start; i < end; ++i) {
    double t = 0.0;
    for (int j = 0; j < n; ++j) {
      t += u[j] * a(i, j);
    }
    w[i] = t;
  }
  return w;
}

Float64List atuFromTo(int start, int end, Float64List u) {
  final n = u.length;
  final w = new Float64List(n);
  for (int i = start; i < end; ++i) {
    double t = 0.0;
    for (int j = 0; j < n; ++j) {
      t += u[j] * a(j, i);
    }
    w[i] = t;
  }
  return w;
}

// same w out reference style as sequential program
Future<void> au(Float64List u, Float64List w) async {
  await mapToIsolates(false, u, w);
}

// same w out reference style as sequential program
Future<void> atu(Float64List u, Float64List w) async {
  await mapToIsolates(true, u, w);
}

// same w out reference style as sequential program
Future<void> atAu(Float64List u, Float64List v, Float64List w) async {
  await au(u, w);
  await atu(w, v);
}

Future<double> spectralNorm(int n) async {
  var u = new Float64List(n)..fillRange(0, n, 1.0),
      v = new Float64List(n),
      w = new Float64List(n),
      vBv = 0.0,
      vv = 0.0;
  // same w out reference style as sequential program
  for (int i = 0; i < 10; ++i) {
    await atAu(u, v, w);
    await atAu(v, u, w);
  }
  for (int i = 0; i < n; ++i) {
    vBv += u[i] * v[i];
    vv += v[i] * v[i];
  }
  return sqrt(vBv / vv);
}

var nIsolates = 0;
final completers = <Completer>[];
final ports = <SendPort>[];
var wOutReference = new Float64List(0);

void main(List<String> args) {
  final mainIsolate = ReceivePort();
  final n = (args.length > 0) ? int.parse(args[0]) : 100;
  nIsolates = Platform.numberOfProcessors;
  Span.initialize(n, nIsolates);

  var i = nIsolates;
  while (i-- > 0) {
    Isolate.spawn(other, mainIsolate.sendPort);
  }
  var awaited = nIsolates;

  mainIsolate.listen((dynamic s) async {
    if (s is Span) {
      // Copy partial auFromTo / atuFromTo result to w out reference.
      List.copyRange(wOutReference, s.start, s.data, s.start, s.end);
      completers.removeLast().complete();
      
    } else if (s is SendPort) {
      ports.add(s);
      if (--awaited == 0) { // Wait until all nIsolates are known.
        print((await spectralNorm(n)).toStringAsFixed(9));
        mainIsolate.close();
      }
    }
  });
}

void other(SendPort p) {
  final otherIsolate = ReceivePort();
  p.send(otherIsolate.sendPort);

  otherIsolate.listen((dynamic s) {
    // update data and send the same Span back
    if (s is Span) {
      if (s.shouldTranspose) {
        s.data = atuFromTo(s.start, s.end, s.data);
      } else {
        s.data = auFromTo(s.start, s.end, s.data);
      }
      p.send(s);
    }
  });
}

var next = 0;

Future<void> mapToIsolates(
    bool shouldTranspose, Float64List u, Float64List w) async {
  wOutReference = w; // same out reference style as sequential program
  final barrier = <Future>[];
  for (var each in Span.spans) {
    each.shouldTranspose = shouldTranspose;
    each.data = u;
    final c = Completer<dynamic>();
    completers.add(c);
    barrier.add(c.future);
    next = (next + 1) % ports.length;
    ports[next].send(each);
  }
  await Future.wait<void>(barrier);
}

class Span {
  static final spans = <Span>[];

  int start, end;
  bool shouldTranspose = false;
  Float64List data = new Float64List(0);
  Span(this.start, this.end);

  static void initialize(int n, int nSpans) {
    final length = (n / nSpans).ceil();
    for (var i = 0; i < nSpans; i++) {
      final start = i * length;
      var end = start + length;
      if (end > n) {
        end = n;
      }
      spans.add(Span(start, end));
    }
  }
}
