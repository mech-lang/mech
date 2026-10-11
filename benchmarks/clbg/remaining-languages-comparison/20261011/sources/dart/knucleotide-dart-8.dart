/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
 
   Naive transliteration from bearophile's program 
   contributed by Isaac Gouy
*/

import 'dart:collection';
import 'dart:io';

List seqLines() {
   while (true) {
      final line = stdin.readLineSync();    
      if (line == null || line.startsWith(">THREE")) {
         break;
      } 
   }
   final lines = <String>[];
   while (true) {
      final line = stdin.readLineSync();    
      if (line == null || line.startsWith(">")) {
         break;
      } 
      lines.add(line);      
   }   
   return lines;
}

HashMap<String, int> baseCounts(int bases, String seq) {
   final counts = new HashMap<String, int>();   
   final size = seq.length + 1 - bases;   
   for (int i = 0; i < size; i++) {  
      final nucleo = seq.substring(i, i + bases);
      if (counts.containsKey(nucleo))
         counts[nucleo] = (counts[nucleo] ?? 0) + 1;
      else
         counts[nucleo] = 1;      
   }
   return counts;
}

Iterable<(String, String)> sortedFreq(int bases, String seq) {
   final keysValues = baseCounts(bases, seq).entries.toList();
   final size = seq.length + 1 - bases;     
   keysValues.sort((a, b) => a.value == b.value ? a.key.compareTo(b.key) : b.value.compareTo(a.value)); 
   return 
      keysValues.map((each) => (each.key,  
         (100.0 * each.value / size ).toStringAsFixed(3))); 
}

specificCount(code, seq) {  
   return baseCounts(code.length, seq)[code] ?? 0;
}

void main(List<String> args) {
   var lines = seqLines();   
   var seq = lines.map((each) => each.toUpperCase()).join();    
   
   for (final base in [1, 2]) {  
      for (final (k,v) in sortedFreq(base,seq)) {
         print("$k $v");
      }
      print("");   
   }
       
   for (final code in ["GGT", "GGTA", "GGTATT",
            "GGTATTTTAATT", "GGTATTTTAATTTATAGT"]) {     
      print("${specificCount(code,seq)}\t$code"); 
   }
}
