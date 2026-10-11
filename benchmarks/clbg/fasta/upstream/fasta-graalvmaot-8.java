/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   Naive transliteration from Drake Diedrich's C program
   contributed by Isaac Gouy 
*/

import java.io.IOException;

final class fasta {
static final int IM = 139968;
static final int IA = 3877;
static final int IC = 29573;
static final int SEED = 42;

static int seed = SEED;
static double fastaRand(double max) {
   seed = (seed * IA + IC) % IM;
   return max * seed / IM;
}

static final String ALU =
  "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGG" +
  "GAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGA" +
  "CCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAAT" +
  "ACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCA" +
  "GCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGG" +
  "AGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCC" +
  "AGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA";

static final String iub = "acgtBDHKMNRSVWY";
static final double[]iubP = {
   0.27, 0.12, 0.12, 0.27, 
   0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02 
};

static final String homosapiens = "acgt";
static final double[] homosapiensP = {
   0.3029549426680,
   0.1979883004921,
   0.1975473066391,
   0.3015094502008
};   

static final int LINELEN = 60;

// slowest character-at-a-time output
static void repeatFasta(String seq, int n) {
   int len = seq.length();
   int i;
   var b = new StringBuilder(); 
   for (i=0; i<n; i++) {
      b.append(seq.charAt(i % len));   
      if (i % LINELEN == LINELEN - 1) {
         System.out.println(b.toString());
         b.setLength(0);  
      }
   }
   if (i % LINELEN != 0) System.out.println(b.toString());   
}

static void randomFasta(String seq, double[] probability, int n) {
   int len = seq.length();
   int i, j;
   var b = new StringBuilder();    
   for (i=0; i<n; i++) {
      double v = fastaRand(1.0);        
      /* slowest idiomatic linear lookup.  Fast if len is short though. */
      for (j=0; j<len-1; j++) {  
         v -= probability[j];
         if (v<0) break;      
      }
      b.append(seq.charAt(j));        
      if (i % LINELEN == LINELEN - 1) {
         System.out.println(b.toString()); 
         b.setLength(0);         
      }         
   }
   if (i % LINELEN != 0) System.out.println(b.toString());
}

public static void main(String[] args) throws IOException {
   final int n = (args.length > 0) ? Integer.parseInt(args[0]) : 1000;
   
   System.out.println(">ONE Homo sapiens alu");    
   repeatFasta(ALU, n*2);
   
   System.out.println(">TWO IUB ambiguity codes");    
   randomFasta(iub, iubP, n*3);   
   
   System.out.println(">THREE Homo sapiens frequency");    
   randomFasta(homosapiens, homosapiensP, n*5);     
}            
}
