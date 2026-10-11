/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   contributed by Sergei Frolkin
   + null safety     
*/

import 'dart:async';
import 'dart:io';
import 'dart:typed_data';

const begSequence = 62; //">"
const endOfLine = 10; //"\n"
const Map<String, String> rules = {
  'A': 'T',
  'B': 'V',
  'C': 'G',
  'D': 'H',
  'G': 'C',
  'H': 'D',
  'K': 'M',
  'M': 'K',
  'N': 'N',
  'R': 'Y',
  'S': 'S',
  'T': 'A',
  'U': 'A',
  'V': 'B',
  'Y': 'R',
  'W': 'W',
};

final Uint8List complement = initComplement();

Uint8List initComplement() {
  final res = Uint8List(256);
  for (var i = 0; i < 255; i++) {
    final c = String.fromCharCode(i).toUpperCase();
    final s = rules[c];
    if (s != null) {
      res[i] = s.codeUnits[0];
    } else {
      res[i] = i;
    }
  }
  return res;
}

class Sequence {
  static const chunkSize = 1024 * 1024;
  var buf = new Uint8List(chunkSize);
  int offset = 0;

  void add(int b) {
    // if (offset >= buf.length) buf.length += chunkSize;
    if (offset >= buf.length) {
      var newbuf = new Uint8List(buf.length + chunkSize);
      newbuf.setAll(0, buf);
      buf = newbuf;
    }
    ;
    buf[offset++] = b;
  }

  Uint8List getRevCompListAndReset() {
    for (int i = 0, j = offset - 1; i < j; i++, j--) {
      while (buf[i] == endOfLine) i++;
      if (i > j) break;
      while (buf[j] == endOfLine) j--;
      if (i > j) break;
      int si = buf[i];
      buf[i] = complement[buf[j]];
      buf[j] = complement[si];
    }
    return getListAndReset();
  }

  Uint8List getListAndReset() {
    final end = offset;
    offset = 0;
    // return buf.buffer.asUint8List(0, end);
    return buf.sublist(0, end);
  }

  bool get isNotEmpty => offset > 0;
}

Stream<Uint8List> run() async* {
  var inDNA = true;
  var dna = new Sequence();
  var txt = new Sequence();
  await for (var chunk in stdin) {
    for (var b in chunk) {
      if (inDNA) {
        if (b != begSequence) {
          dna.add(b);
        } else {
          yield dna.getRevCompListAndReset();
          inDNA = false;
          txt.add(b);
        }
      } else {
        txt.add(b);
        if (b == endOfLine) {
          yield txt.getListAndReset();
          inDNA = true;
        }
      }
    }
  }
  if (dna.isNotEmpty) {
    yield dna.getRevCompListAndReset();
  } else if (txt.isNotEmpty) {
    yield txt.getListAndReset();
  }
}

main() {
  run().listen((Uint8List chunk) {
    stdout.add(chunk);
  });
}
