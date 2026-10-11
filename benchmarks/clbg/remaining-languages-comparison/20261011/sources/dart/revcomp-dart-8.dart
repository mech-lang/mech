/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   contributed by Isaac Gouy based on Jeremy Zerfas's #6 C program 
*/

import 'dart:io';
import 'dart:typed_data';

final nl = '\n'.codeUnitAt(0);
final def = '>'.codeUnitAt(0); // Definition-line.
final lut = codeComplementLookupTable();

// READ_SIZE probably 64x1024 ?

void main() async {
  var sequence = BytesBuilder(copy: false);
  await for (var block in stdin) {
    var start = 0;
    for (var i = 0; i < block.length; i++) {
      if (block[i] != def) continue;
      if (sequence.length > 0 || i > start) {
        sequence.add(block.sublist(start,i));
        reverseAndComplement(sequence.takeBytes());
      }
      start = i;
    }
    sequence.add(block.sublist(start));
  }
  reverseAndComplement(sequence.takeBytes());
}

void reverseAndComplement(List<int> sequence) {
  var frontPos = sequence.indexOf(nl) + 1;
  var backPos = sequence.length - 1;
  while (frontPos <= backPos && sequence[frontPos] == nl) frontPos++;
  while (frontPos <= backPos && sequence[backPos] == nl) backPos--;

  while (frontPos <= backPos) {
    var tmp = lut[sequence[frontPos]];
    sequence[frontPos] = lut[sequence[backPos]];
    sequence[backPos] = tmp;
    while (sequence[++frontPos] == nl);
    while (sequence[--backPos] == nl);
  }
  stdout.add(sequence);
}

Uint8List codeComplementLookupTable() {
  final code =       'ABCDGHKMNRSTUVWY'.codeUnits;
  final complement = 'TVGHCDMKNYSAABWR'.codeUnits;
  final lowercaseOffset = 'a'.codeUnitAt(0) - 'A'.codeUnitAt(0);
  final t = Uint8List.fromList(List.generate(128, (int i) => i)); 
  for (var i = 0; i < code.length; i++) {
    t[code[i]] = complement[i];
    t[code[i] + lowercaseOffset] = complement[i];
  }  
  return t;
}
