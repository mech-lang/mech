<?
/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   Naive transliteration from Drake Diedrich's C program
   contributed by Isaac Gouy 
*/

const IM = 139968;
const IA = 3877;
const IC = 29573;
const SEED = 42;
function fastaRand($max) {
   static $seed = SEED;
   $seed = ($seed * IA + IC ) % IM;
   return $max * $seed / IM;
}

const ALU =
   "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGG" .
   "GAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGA" .
   "CCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAAT" .
   "ACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCA" .
   "GCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGG" .
   "AGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCC" .
   "AGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA";

const IUB = "acgtBDHKMNRSVWY";
const IUB_P = array(
   0.27,
   0.12,
   0.12,
   0.27,
   0.02,
   0.02,
   0.02,
   0.02,
   0.02,
   0.02,
   0.02,
   0.02,
   0.02,
   0.02,
   0.02 
);

const HOMOSAPIENS = "acgt";
const HOMOSAPIENS_P = array(
   0.3029549426680,
   0.1979883004921,
   0.1975473066391,
   0.3015094502008
);

const LINELEN= 60;

// slowest character-at-a-time output
function repeatFasta($seq, $n) {
   $len = strlen($seq);
   $i;
   /* explicit line buffer */     
   $b = "";   
   for ($i=0; $i<$n; $i++) {
      $b .= $seq[$i % $len];
      if ($i % LINELEN == LINELEN - 1) {
         $b .= "\n";
         print $b;
         $b = "";       
      }
   }
   if ($i % LINELEN != 0) {   
      $b .= "\n";   
      print $b;    
   }      
}

function randomFasta($seq, $probability, $n) {
   $len = strlen($seq);
   $i; $j;
   /* explicit line buffer */     
   $b = "";      
   for ($i=0; $i<$n; $i++) {
      $v = fastaRand(1.0);
      /* slowest idiomatic linear lookup.  Fast if len is short though. */
      for ($j=0; $j<$len-1; $j++) {
         $v -= $probability[$j];
         if ($v<0) break;
      }
      $b .= $seq[$j];
      if ($i % LINELEN == LINELEN - 1) {
         $b .= "\n";
         print $b;
         $b = "";       
      }
   }
   if ($i % LINELEN != 0) {   
      $b .= "\n";   
      print $b;    
   } 
}

function main($n) {
   print ">ONE Homo sapiens alu\n";
   repeatFasta(ALU, $n*2);
  
   print ">TWO IUB ambiguity codes\n";
   randomFasta(IUB, IUB_P, $n*3);
  
   print ">THREE Homo sapiens frequency\n";
   randomFasta(HOMOSAPIENS, HOMOSAPIENS_P, $n*5);
}

main( $argc > 1 ? $argv[1] : 1000 );

?>
