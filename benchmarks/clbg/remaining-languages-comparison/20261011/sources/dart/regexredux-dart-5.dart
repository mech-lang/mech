/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   contributed by Isaac Gouy, RegExp from Jos Hirth's program
*/

import 'dart:io';
import 'dart:isolate';

void main() {
  final mainIsolate = ReceivePort();
  var initial = 0, cleaned = 0;

  File('/dev/stdin').readAsString().then((String s) {
    final z = (s.replaceAll(RegExp('>.*\n|\n', multiLine: true), ''));

    Isolate.spawn(magicReplacements, Data(z, mainIsolate.sendPort));
    printPatternMatches(z);
    initial = s.length;
    cleaned = z.length;
  });

  mainIsolate.listen((dynamic magicLength) {
    if (magicLength is int) {
      print('\n$initial\n$cleaned\n$magicLength');
      mainIsolate.close();
    }
  });
}

void printPatternMatches(String s) {
  const simple = [
    'agggtaaa|tttaccct',
    '[cgt]gggtaaa|tttaccc[acg]',
    'a[act]ggtaaa|tttacc[agt]t',
    'ag[act]gtaaa|tttac[agt]ct',
    'agg[act]taaa|ttta[agt]cct',
    'aggg[acg]aaa|ttt[cgt]ccct',
    'agggt[cgt]aa|tt[acg]accct',
    'agggta[cgt]a|t[acg]taccct',
    'agggtaa[cgt]|[acg]ttaccct'
  ];

  for (var each in simple) {
    final regex = RegExp(each, multiLine: true);
    print('$each ${regex.allMatches(s).length}');
  }
}

int magicLengthOf(String s) {
  const magic = [
    ['tHa[Nt]', '<4>'],
    ['aND|caN|Ha[DS]|WaS', '<3>'],
    ['a[NSt]|BY', '<2>'],
    ['<[^>]*>', '|'],
    ['\\|[^|][^|]*\\|', '-'],
  ];

  for (var each in magic) {
    final regex = RegExp(each.first, multiLine: true);
    s = s.replaceAll(regex, each.last);
  }
  return s.length;
}

void magicReplacements(Data some) {
  some.sender.send(magicLengthOf(some.string));
}

class Data {
  String string;
  SendPort sender;
  Data(this.string, this.sender);
}
