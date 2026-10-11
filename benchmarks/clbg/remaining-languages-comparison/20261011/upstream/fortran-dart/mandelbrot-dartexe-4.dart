/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   Use Isolate.exit() added in Dart 2.15 
   Contributed by Isaac Gouy. Make time on each Isolate similar.
   renderRow from Andrey Filatkin's node #3 program. 
*/

import 'dart:io';
import 'dart:isolate';
import 'dart:typed_data';

const iter = 50, limit = 4.0;

void main(List<String> args) {
  final h = (args.length > 0) ? int.parse(args[0]) : 200, w = h;

  final assignments = segments(h).map((each) => assign(each));

  stdout.write('P4\n$w $h\n');
  Future.wait(assignments).then((segments) {
    for (var each in segments) {
      for (var row in each) {
        stdout.add(row);
      }
    }
  });
}

Future<List<Uint8List>> assign(Index assignment) async {
  final p = ReceivePort();
  await Isolate.spawn(renderRows, [p.sendPort, assignment]);
  return await p.first;
}

Future renderRows(List<dynamic> args) async {
  final SendPort p = args[0];
  final Index index = args[1];
  final int w = index.n;
  final bytesPerRow = w >> 3;

  final initialR = new Float64List(w);
  final initialI = new Float64List(w);
  final inv = 2 / w;
  for (var xy = 0; xy < w; xy++) {
    final i = inv * xy;
    initialR[xy] = i - 1.5;
    initialI[xy] = i - 1.0;
  }

  Uint8List renderRow(int y) {
    final row = Uint8List(bytesPerRow);

    for (var xByte = 0; xByte < bytesPerRow; xByte++) {
      final ci = initialI[y];
      var res = 0;
      for (var i = 0; i < 8; i += 2) {
        final x = xByte << 3;
        final cr1 = initialR[x + i];
        final cr2 = initialR[x + i + 1];

        var zr1 = cr1;
        var zi1 = ci;

        var zr2 = cr2;
        var zi2 = ci;

        var b = 0;

        for (var j = 0; j < iter; j++) {
          final tr1 = zr1 * zr1;
          final ti1 = zi1 * zi1;
          zi1 = 2 * zr1 * zi1 + ci;
          zr1 = tr1 - ti1 + cr1;

          if (tr1 + ti1 > limit) {
            b |= 2;
            if (b == 3) {
              break;
            }
          }

          final tr2 = zr2 * zr2;
          final ti2 = zi2 * zi2;
          zi2 = 2 * zr2 * zi2 + ci;
          zr2 = tr2 - ti2 + cr2;

          if (tr2 + ti2 > limit) {
            b |= 1;
            if (b == 3) {
              break;
            }
          }
        }
        res = (res << 2) | b;
      }
      row[xByte] = ~res;
    }
    return row;
  }

  final rows = <Uint8List>[];
  for (var y = index.lo; y < index.hi; y++) {
    rows.add(renderRow(y));
  }
  Isolate.exit(p, rows);
}

List<Index> segments(int n) {
  const weights = [0.35, 0.5, 0.65];
  var lo = 0, weighted = 0;
  final s = <Index>[];
  for (var i = 0; i < weights.length; i++) {
    weighted = (weights[i] * n).floor();
    s.add(Index(lo, weighted, n));
    lo = weighted;
  }
  s.add(Index(lo, n, n));
  return s;
}

class Index {
  var lo = 0, hi = 0, n = 0;
  Index(this.lo, this.hi, this.n);
}
