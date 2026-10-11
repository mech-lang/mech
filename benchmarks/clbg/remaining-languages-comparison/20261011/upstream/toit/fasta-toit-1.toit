
/* The Computer Language Benchmarks game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   transliterated from Dart #3 program by Isaac Gouy 
*/  

import host.pipe
import bytes show Buffer

WIDTH := 60
NL ::= '\n'


repeated-sequence 
    sequence/string 
    size/int 
    -> ByteArray:
    
  codes ::= sequence.to-byte-array
  codes-length ::= codes.size
  lines := Buffer
  
  codes-extended := ByteArray codes-length + WIDTH 
  for column := 0; column < codes-length + WIDTH; column++:  
    codes-extended[column] = codes[column % codes-length]

  offset := 0
  line := ByteArray (WIDTH + 1)
  line[WIDTH] = NL 
  for current-size := size; current-size > 0;: 
    line-length := WIDTH
    if current-size < WIDTH:
      line-length = current-size
      line = ByteArray (line-length + 1) // Shorten fixed-size last line.
      line[line-length] = NL    

    line.replace 0 codes-extended offset (offset + line.size - 1)                
    offset += line-length
    if offset > codes-length: offset -= codes-length
    lines.write line 0 line.size    
    current-size -= line-length     
    
  return lines.bytes  
  
  
M ::= 139968
A ::= 3877
C ::= 29573
seed := 42

next-lcg-number max/float -> float: 
  seed = (seed * A + C) % M
  return max * seed / M   
  
weighted-lcg-sequence 
    code-string/string 
    probabilities/List 
    size/int 
    -> ByteArray: 
    
  codes ::= code-string.to-byte-array  
  lines := Buffer
  sum := 0.0
  for i := 0; i < probabilities.size; i++:  
    sum += probabilities[i]
    probabilities[i] = sum     
    
  line := ByteArray (WIDTH + 1)
  line[WIDTH] = NL
  for current-size := size; current-size > 0;: 
    line-length := WIDTH
    if current-size < WIDTH:
      line-length = current-size
      line = ByteArray (line-length + 1) // Shorten fixed-size last line.
      line[line-length] = NL 
      
    for i := 0; i < line-length; i++: 
      r := next-lcg-number 1.0 
      for j := 0; j < probabilities.size; j++:       
        if r < probabilities[j]:               
          line[i] = codes[j] 
          break                                                
    lines.write line 0 (line-length + 1)      
    current-size -= line-length      
  
  return lines.bytes  

  
main args:
  n := args.size == 1 ? int.parse args[0] : 1000
   
  print ">ONE Homo sapiens alu"   
  pipe.stdout.write
    repeated-sequence
        """
        GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGG\
        GAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGA\
        CCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAAT\
        ACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCA\
        GCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGG\
        AGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCC\
        AGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA\
        """    
        n * 2  
        
  print ">TWO IUB ambiguity codes"  
  pipe.stdout.write 
    weighted-lcg-sequence 
        "acgtBDHKMNRSVWY" 
        [0.27, 0.12, 0.12, 0.27, 0.02, 
            0.02, 0.02, 0.02, 0.02, 0.02, 
            0.02, 0.02, 0.02, 0.02, 0.02,]            
        n * 3          

  print ">THREE Homo sapiens frequency"  
  pipe.stdout.write 
    weighted-lcg-sequence 
        "acgt" 
        [0.3029549426680, 0.1979883004921, 
            0.1975473066391, 0.3015094502008,] 
        n * 5

    