
/* The Computer Language Benchmarks game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   Naive transliteration from Drake Diedrich's C program
   contributed by Isaac Gouy
*/  

M ::= 139968
A ::= 3877
C ::= 29573
SEED ::= 42

seed := SEED
fasta_rand max/float -> float: 
  seed = (seed * A + C) % M
  return seed.to-float / 139968.0   

ALU ::= """
        GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGG\
        GAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGA\
        CCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAAT\
        ACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCA\
        GCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGG\
        AGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCC\
        AGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA\
        """
        
IUB ::= "acgtBDHKMNRSVWY" 

IUB_P ::= [0.27, 0.12, 0.12, 0.27, 0.02, 
            0.02, 0.02, 0.02, 0.02, 0.02, 
            0.02, 0.02, 0.02, 0.02, 0.02,] 

HOMOSAPIENS ::= "acgt"

HOMOSAPIENS_P ::= [
  0.3029549426680,
  0.1979883004921,
  0.1975473066391,
  0.3015094502008
]

LINELEN := 60 
NL ::= "\n"

// slowest character-at-a-time output
repeat_fasta seq/string n/int:
  len ::= seq.size
  i := 0
  for i=0; i<n; i++:   
    c := string.from-rune seq[i % len]
    write-on-stdout_ c false
    if (i % LINELEN == LINELEN - 1): print "" 
  if (i % LINELEN != 0): print ""

random_fasta seq/string probability/List n/int:
  len ::= seq.size
  total := 0.0
  len.repeat:
    total += probability[it]
    probability[it] = total
  i := 0; j := 0
  for i=0; i<n; i++:  
    v := fasta_rand 1.0
    // slowest idiomatic linear lookup. Fast if len is short though.
    for j=0; j<len - 1; j++:
      if v < probability[j]: break      
    c := string.from-rune seq[j]      
    write-on-stdout_ c false      
    if (i % LINELEN == LINELEN - 1): print "" 
  if (i % LINELEN != 0): print ""      
      
main args:
  n := args.size == 1 ? int.parse args[0] : 1000

  print ">ONE Homo sapiens alu"
  repeat_fasta ALU n*2
  
  print ">TWO IUB ambiguity codes"  
  random_fasta IUB IUB_P n*3
  
  print ">THREE Homo sapiens frequency"  
  random_fasta HOMOSAPIENS HOMOSAPIENS_P n*5  

    