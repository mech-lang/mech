/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   Naive transliteration from Drake Diedrich's C program
   contributed by Isaac Gouy 
*/

using System.Text;

class Fasta {
const int IM = 139968;
const int IA = 3877;
const int IC = 29573;
const int SEED = 42;

static int seed = SEED;
static double fastaRand(double max) {
   seed = (seed * IA + IC) % IM;
   return max * seed / IM;
}

const String ALU =
  "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGG" +
  "GAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGA" +
  "CCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAAT" +
  "ACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCA" +
  "GCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGG" +
  "AGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCC" +
  "AGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA";
static char[] aluC = ALU.ToCharArray(); 

const String iub = "acgtBDHKMNRSVWY";
static double[]iubP = {
   0.27, 0.12, 0.12, 0.27, 
   0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02 
};

const String homosapiens = "acgt";
static double[] homosapiensP = {
   0.3029549426680,
   0.1979883004921,
   0.1975473066391,
   0.3015094502008
};  

const int LINELEN = 60;

// slowest character-at-a-time output
static void repeatFasta(String seq, int n) {
   int len = seq.Length;
   int i;
   var b = new StringBuilder();
   for (i=0; i<n; i++) {
      b.Append(seq[i % len]);
      if (i % LINELEN == LINELEN - 1) {
         Console.WriteLine(b);   
         b.Length = 0;
      }
   }
   if (i % LINELEN != 0) Console.WriteLine(b);   
}

static void randomFasta(String seq, double[] probability, int n) {
   int len = seq.Length;
   int i, j;
   var b = new StringBuilder();   
   for (i=0; i<n; i++) {
      double v = fastaRand(1.0);        
      /* slowest idiomatic linear lookup.  Fast if len is short though. */
      for (j=0; j<len-1; j++) {  
         v -= probability[j];
         if (v<0) break;      
      }
      b.Append(seq[j]);      
      if (i % LINELEN == LINELEN - 1) {
         Console.WriteLine(b);   
         b.Length = 0;
      }
   }
   if (i % LINELEN != 0) Console.WriteLine(b); 
}

static void Main(String[] args){      
   int n = (args.Length > 0) 
      ? Int32.Parse(args[0]) 
      : 1000;     
      
   Console.SetOut( new StreamWriter(Console.OpenStandardOutput()) );      
   
   Console.WriteLine(">ONE Homo sapiens alu");    
   repeatFasta(ALU, n*2);
  
   Console.WriteLine(">TWO IUB ambiguity codes");    
   randomFasta(iub, iubP, n*3);   
   
   Console.WriteLine(">THREE Homo sapiens frequency");    
   randomFasta(homosapiens, homosapiensP, n*5);
   
   Console.Out.Flush();
}            
}
