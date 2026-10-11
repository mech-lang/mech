/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   Naive transliteration from Drake Diedrich's C program
   contributed by Isaac Gouy 
*/

import 'dart:io';

final int IM = 139968;
final int IA = 3877;
final int IC = 29573;
final int SEED = 42;

int seed = SEED;
double fastaRand(double max) {
   seed = (seed * IA + IC) % IM;
   return max * seed / IM;
}

final String ALU =
  "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGG" +
  "GAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGA" +
  "CCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAAT" +
  "ACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCA" +
  "GCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGG" +
  "AGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCC" +
  "AGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA";

final String iub = "acgtBDHKMNRSVWY";
final List<double> iubP = [
   0.27, 0.12, 0.12, 0.27, 
   0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02 
];

final String homosapiens = "acgt";
final List<double> homosapiensP = [
   0.3029549426680,
   0.1979883004921,
   0.1975473066391,
   0.3015094502008
];  

final int LINELEN = 60;

void repeatFasta(String seq, int n) {
   int len = seq.length;
   int i;
   /* explicit line buffer */
   final b = StringBuffer();   
   for (i=0; i<n; i++) {
      b.write( seq[i % len] );
      if (i % LINELEN == LINELEN - 1) { 
         stdout.writeln( b.toString() );
         b.clear();
      }     
   }
   if (i % LINELEN != 0) stdout.writeln( b.toString() ); 
}

void randomFasta(String seq,  List<double> probability, int n) {
   int len = seq.length;
   int i, j;
   /* explicit line buffer */    
   final b = StringBuffer();  
   for (i=0; i<n; i++) {
      double v = fastaRand(1.0);        
      /* slowest idiomatic linear lookup.  Fast if len is short though. */
      for (j=0; j<len-1; j++) {  
         v -= probability[j];
         if (v<0) break;      
      }
      b.write( seq[j] );   
      if (i % LINELEN == LINELEN - 1) { 
         stdout.writeln( b.toString() );
         b.clear();
      }     
   }
   if (i % LINELEN != 0) stdout.writeln( b.toString() );  
}

void main(List<String> args) {
  final n = (args.length > 0) ? int.parse(args[0]) : 1000;
  
   stdout.writeln(">ONE Homo sapiens alu");    
   repeatFasta(ALU, n*2);
   
   stdout.writeln(">TWO IUB ambiguity codes");    
   randomFasta(iub, iubP, n*3);   
   
   stdout.writeln(">THREE Homo sapiens frequency");    
   randomFasta(homosapiens, homosapiensP, n*5);                
}
