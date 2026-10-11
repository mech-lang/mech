// The Computer Language Benchmarks Game
// https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
//
// updated the version of Jos Hirth with inspiration from the 
// Java version by Ziad Hatahet
// isolates added by Maximilian Simmoteit

import 'dart:math' as math;
import 'dart:typed_data';
import 'dart:isolate';
import 'dart:async';
import 'dart:io';

final num_threads = 2 * Platform.numberOfProcessors;

double a(int i, int j) {
  var div = ((i + j) * (i + j + 1) >> 1) + i + 1;
  return 1.0 / div;
}

Float64List timesCalculation(Float64List u, int ii, int n, bool transpose) {
  final ul = u.length;
  var v = Float64List(n);
  for (var i = ii; i < ii + n; i++) {
    var vi = 0.0;
    for (var j = 0; j < ul; j++) {
      if (transpose) {
        vi += u[j] * a(j, i);
      } else {
        vi += u[j] * a(i, j);
      }
    }
    v[i - ii] = vi;
  }
  return v;
}

void calculateTimes(SendPort initialReplyTo) {
  var port = ReceivePort();
  initialReplyTo.send(port.sendPort);

  port.listen((msg) {
    Float64List u = msg['u'];
    int from = msg['from'];
    int len = msg['len'];
    bool transpose = msg['transp'];
    SendPort replyTo = msg['port'];

    var timesSegment = timesCalculation(u, from, len, transpose);
    replyTo.send(timesSegment);
  });
}

Future<void> AtAu(Float64List u, Float64List v, Float64List w,
    List<SendPort?> isolate_communication_send) async {
  var partial_u_to_w_results = List<Future<dynamic>?>.filled(num_threads, null);
  var segmentSize = List.filled(num_threads, u.length ~/ num_threads);
  segmentSize[0] += u.length % num_threads;

  var from = 0;
  for (var i = 0; i < num_threads; i++) {
    var len = segmentSize[i];
    var response = ReceivePort();
    var localFrom = from;
    isolate_communication_send[i]?.send({
      'u': u,
      'from': localFrom,
      'len': len,
      'transp': false,
      'port': response.sendPort
    });
    var newFuture = response.first;
    partial_u_to_w_results[i] = newFuture;
    from += len;
  }

  var w_idx = 0;

  // write the partial results to w
  for (var res_float_list in partial_u_to_w_results) {
    var float_list = (await res_float_list)!;
    for (var i = 0; i < float_list.length; i++) {
      w[w_idx] = float_list[i];
      w_idx++;
    }
  }

  var partial_w_to_v_results = List<Future<dynamic>?>.filled(num_threads, null);
  from = 0;
  for (var i = 0; i < num_threads; i++) {
    var len = segmentSize[i];
    var response = ReceivePort();
    var localFrom = from;
    isolate_communication_send[i]?.send({
      'u': w,
      'from': localFrom,
      'len': len,
      'transp': true,
      'port': response.sendPort
    });
    var newFuture = response.first;
    partial_w_to_v_results[i] = newFuture;
    from += len;
  }

  var v_idx = 0;

  // write the partial results to v
  for (var res_float_list in partial_w_to_v_results) {
    var float_list = (await res_float_list)!;
    for (var i = 0; i < float_list.length; i++) {
      v[v_idx] = float_list[i];
      v_idx++;
    }
  }
}

Future<double> spectralNorm(n) async {
  var u = Float64List(n)..fillRange(0, n, 1.0),
      v = Float64List(n),
      w = Float64List(n),
      vv = 0.0 as double,
      vBv = 0.0 as double;

  var isolate_communication_send = List<SendPort?>.filled(num_threads, null);
  // spawn Isolates
  for (var i = 0; i < num_threads; i++) {
    var response = ReceivePort();
    isolate_communication_send[i] =
        await Isolate.spawn(calculateTimes, response.sendPort)
            .then((_) => response.first) as SendPort;
  }

  for (var i = 0; i < 10; ++i) {
    await AtAu(u, v, w, isolate_communication_send);
    await AtAu(v, u, w, isolate_communication_send);
  }

  for (var i = 0; i < n; ++i) {
    vBv += u[i] * v[i];
    vv += v[i] * v[i];
  }

  return math.sqrt(vBv / vv);
}

void main(args) async {
  var n = args.length > 0 ? int.parse(args[0]) : 100;
  print((await spectralNorm(n)).toStringAsFixed(9));
}
