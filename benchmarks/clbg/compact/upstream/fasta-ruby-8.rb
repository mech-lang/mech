# The Computer Language Benchmarks Game
#   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
#
#   Naive transliteration from Drake Diedrich's C program
#   contributed by Isaac Gouy 
#

IM = 139968
IA = 3877
IC = 29573
SEED = 42

$seed = SEED
def fastaRand(max)
   $seed = ($seed * IA + IC) % IM
   return max * $seed / IM
end

ALU =
  "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGG" +
  "GAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGA" +
  "CCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAAT" +
  "ACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCA" +
  "GCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGG" +
  "AGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCC" +
  "AGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA" 

IUB = "acgtBDHKMNRSVWY"
IUB_P = [
   0.27, 0.12, 0.12, 0.27, 
   0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02 
]

HomoSapiens = "acgt"
HomoSapiens_P = [
   0.3029549426680,
   0.1979883004921,
   0.1975473066391,
   0.3015094502008
]  

LINELEN = 60

# slowest character-at-a-time output
def repeatFasta(seq, n)
   len = seq.length
   # explicit line buffer      
   b = "";   
   for i in 0..n-1
      b << seq[i % len]
      if (i % LINELEN == LINELEN - 1) then 
         b << "\n";      
         print b 
         b = ""       
      end   
   end
   if (i % LINELEN != 0) then 
      b << "\n"   
      print b 
   end 
end

def randomFasta(seq, probability, n) 
   len = seq.length
   # explicit line buffer      
   b = "";    
   for i in 0..n-1
      v = fastaRand 1.0       
      # slowest idiomatic linear lookup.  Fast if len is short though.  
      for j in 0..len-1      
         v -= probability[j]
         if v < 0 then break end    
      end
      b << seq[j]  
      if (i % LINELEN == LINELEN - 1) then 
         b << "\n"      
         print b 
         b = ""       
      end      
   end
   if ((i+1) % LINELEN != 0) then 
      b << "\n"  
      print b 
   end 
end

def main() 
   n = (ARGV[0] || 1000).to_i 
   
   print ">ONE Homo sapiens alu\n"   
   repeatFasta(ALU, n*2)
   
   print ">TWO IUB ambiguity codes\n"     
   randomFasta(IUB, IUB_P, n*3)   

   print ">THREE Homo sapiens frequency\n"  
   randomFasta(HomoSapiens, HomoSapiens_P, n*5)     
end            

main
