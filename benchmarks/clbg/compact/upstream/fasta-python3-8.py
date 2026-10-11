# The Computer Language Benchmarks Game
#   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
#
#   Naive transliteration from Drake Diedrich's C program
#   contributed by Isaac Gouy 
#

from sys import argv

IM = 139968
IA = 3877
IC = 29573
SEED = 42

seed = SEED
def fastaRand(max):
   global seed
   seed = (seed * IA + IC) % IM
   return max * seed / IM

ALU = (
  "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGG" 
  "GAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGA"
  "CCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAAT"
  "ACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCA"
  "GCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGG"
  "AGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCC"
  "AGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA" )

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
def repeatFasta(seq, n):
   length = len(seq)
   i = 0
   # explicit line buffer    
   b = ""    
   for i in range(0,n):
      b += seq[i % length]   
      if (i % LINELEN == LINELEN - 1): 
         print(b)
         b = ""             
   if (i % LINELEN != 0):   
      print(b)     

def randomFasta(seq, probability, n):
   length = len(seq)
   i, j = 0, 0
   # explicit line buffer    
   b = ""   
   for i in range(0,n):
      v = fastaRand(1.0)       
      # slowest idiomatic linear lookup.  Fast if len is short though.
      for j in range(0,length):      
         v -= probability[j]
         if (v<0): break     
      b += seq[j]   
      if (i % LINELEN == LINELEN - 1): 
         print(b)
         b = "" 
   if ((i+1) % LINELEN != 0): 
      print(b)   

def main(n):
   print(">ONE Homo sapiens alu")      
   repeatFasta(ALU, n*2)
   
   print(">TWO IUB ambiguity codes")     
   randomFasta(IUB, IUB_P, n*3)   
   
   print(">THREE Homo sapiens frequency")     
   randomFasta(HomoSapiens, HomoSapiens_P, n*5)                

if __name__ == '__main__':
  main(int(argv[1]) if len(argv) > 1 else 1000)
