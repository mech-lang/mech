/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   contributed by Isaac Gouy based on Jeremy Zerfas's #5 C program 
   Write buffered, naive parallel.
*/

import 'dart:io';
import 'dart:isolate';
import 'dart:typed_data';

const width = 60;
final nl = '\n'.codeUnitAt(0);

Uint8List repeatedSequence(String sequence, int size) {
  final codes = sequence.codeUnits;
  final codesLength = codes.length;
  final lines = BytesBuilder();

  final codesExtended = Uint8List(codesLength + width);
  for (var column = 0; column < codesLength + width; column++) {
    codesExtended[column] = codes[column % codesLength];
  }
  var offset = 0;
  var line = Uint8List(width + 1);
  line[width] = nl;
  for (var currentSize = size; currentSize > 0;) {
    var lineLength = width;
    if (currentSize < width) {
      lineLength = currentSize;
      line = Uint8List(lineLength + 1); // Shorten fixed-size last line.
      line[lineLength] = nl;
    }
    line.setRange(0, lineLength, codesExtended, offset);
    offset += lineLength;
    if (offset > codesLength) offset -= codesLength;
    lines.add(line);
    currentSize -= lineLength;
  }
  return lines.takeBytes();
}

const iM = 139968;
const iA = 3877;
const iC = 29573;
var seed = 42;

double nextLcgNumber(int max) {
  seed = (seed * iA + iC) % iM;
  return max / iM * seed;
}

Uint8List weightedLcgSequence(
    String codeString, List<double> probabilities, int size) {
  final codes = codeString.codeUnits;
  final lines = BytesBuilder();
  var sum = 0.0;
  for (var i = 0; i < probabilities.length; i++) {
    sum += probabilities[i];
    probabilities[i] = sum * iM;
  }
  var line = Uint8List(width + 1);
  line[width] = nl;
  for (var currentSize = size; currentSize > 0;) {
    var lineLength = width;
    if (currentSize < width) {
      lineLength = currentSize;
      line = Uint8List(lineLength + 1); // Shorten fixed-size last line.
      line[lineLength] = nl;
    }
    final last = probabilities.length - 1;
    for (var column = 0; column < lineLength; column++) {
      final r = nextLcgNumber(iM);
      var i = 0;
      for (; i < last; i++) {
        if (probabilities[i] > r) break;
      }
      line[column] = codes[i];
    }
    lines.add(line);
    currentSize -= lineLength;
  }
  return lines.takeBytes();
}

void main(List<String> args) {
  final mainIsolate = ReceivePort();
  final n = (args.length > 0) ? int.parse(args[0]) : 1000;

  Isolate.spawn(
      other,
      Request(
          '>ONE Homo sapiens alu',
          'GGCCGGGCGCGGTGGCTCACGCCT'
              'GTAATCCCAGCACTTTGGGAGGCC'
              'GAGGCGGGCGGATCACCTGAGGTC'
              'AGGAGTTCGAGACCAGCCTGGCCA'
              'ACATGGTGAAACCCCGTCTCTACT'
              'AAAAATACAAAAATTAGCCGGGCG'
              'TGGTGGCGCGCGCCTGTAATCCCA'
              'GCTACTCGGGAGGCTGAGGCAGGA'
              'GAATCGCTTGAACCCGGGAGGCGG'
              'AGGTTGCAGTGAGCCGAGATCGCG'
              'CCACTGCACTCCAGCCTGGGCGAC'
              'AGAGCGAGACTCCGTCTCAAAAA',
          n * 2,
          mainIsolate.sendPort));

  final sequenceTwo = weightedLcgSequence(
      'acgtBDHKMNRSVWY',
      [0.27, 0.12, 0.12, 0.27, 0.02] +
          [0.02, 0.02, 0.02, 0.02, 0.02] +
          [0.02, 0.02, 0.02, 0.02, 0.02],
      n * 3);

  final sequenceThree = weightedLcgSequence(
      'acgt',
      [0.3029549426680, 0.1979883004921, 0.1975473066391, 0.3015094502008],
      n * 5);

  mainIsolate.listen((dynamic _) {
    stdout.writeln('>TWO IUB ambiguity codes');
    stdout.add(sequenceTwo);
    stdout.writeln('>THREE Homo sapiens frequency');
    stdout.add(sequenceThree);
    mainIsolate.close();
  });
}

void other(Request ini) {
  stdout.writeln(ini.defLine);
  stdout.add(repeatedSequence(ini.sequence, ini.size));
  ini.p.send(1);
}

class Request {
  String defLine;
  String sequence;
  int size;
  SendPort p;
  Request(this.defLine, this.sequence, this.size, this.p);
}
