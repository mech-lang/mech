/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   contributed by Isaac Gouy, RegExp from Jos Hirth's program
*/

import 'dart:io';

void main() {
  File('/dev/stdin').readAsString().then((String s) {
    final z = s.replaceAll(RegExp('>.*\n|\n', multiLine: true), '');
    printPatternMatches(z);
    print('\n${s.length}\n${z.length}\n${magicLengthOf(z)}');
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
