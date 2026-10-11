/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   Naive transliteration from Drake Diedrich's C program
   contributed by Isaac Gouy 
*/

const IM = 139968
const IA = 3877
const IC = 29573
const SEED = 42

let seed = SEED
function fastaRand(max) {
   seed = (seed * IA + IC) % IM
   return max * seed / IM
}

const ALU =
  "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGG" +
  "GAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGA" +
  "CCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAAT" +
  "ACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCA" +
  "GCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGG" +
  "AGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCC" +
  "AGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA" 

const iub = "acgtBDHKMNRSVWY"
const iubP = [
   0.27, 0.12, 0.12, 0.27, 
   0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02 
]

const homosapiens = "acgt"
const  homosapiensP = [
   0.3029549426680,
   0.1979883004921,
   0.1975473066391,
   0.3015094502008
]  

const LINELEN = 60

// slowest character-at-a-time output
function repeatFasta(seq, n) {
   const len = seq.length
   let i
   /* explicit line buffer */     
   let b = "";     
   for (i=0; i<n; i++) { 
      b += seq[i % len]
      if (i % LINELEN == LINELEN - 1) {
         b += "\n"
         process.stdout.write(b) 
         b = ""        
      }
   }
   if (i % LINELEN != 0) {   
      b += "\n"   
      process.stdout.write(b) 
   }
}

function randomFasta(seq, probability, n) {
   const len = seq.length
   let i, j
   /* explicit line buffer */     
   let b = "";     
   for (i=0; i<n; i++) {
      let v = fastaRand(1.0)       
      /* slowest idiomatic linear lookup.  Fast if len is short though. */
      for (j=0; j<len-1; j++) {  
         v -= probability[j]
         if (v<0) break     
      }
      b += seq[j]  
      if (i % LINELEN == LINELEN - 1) {
         b += "\n"
         process.stdout.write(b) 
         b = ""        
      }  
   }
   if (i % LINELEN != 0) {   
      b += "\n"   
      process.stdout.write(b) 
   }
}

function main(n) {
   process.stdout.write(">ONE Homo sapiens alu\n")    
   repeatFasta(ALU, n*2)
   
   process.stdout.write(">TWO IUB ambiguity codes\n")     
   randomFasta(iub, iubP, n*3)   
   
   process.stdout.write(">THREE Homo sapiens frequency\n")     
   randomFasta(homosapiens, homosapiensP, n*5)     
}            

main(+process.argv[2] || 1000)
