
/* The Computer Language Benchmarks game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   transliterated from Jeremy Zerfas's #6 C program by Isaac Gouy  
*/ 

import host.pipe
import bytes show Buffer
  
NL ::= '\n'
DEF ::= '>' // Definition-line.
lut ::= code-complement-lookup-table

// READ_SIZE 64x1024

main args:
  sequence := Buffer
  while block := pipe.stdin.read: 
    sequence-start := block.index-of DEF 
    if sequence-start >= 0:    
      if sequence.size > 0:    
        sequence.write block 0 sequence-start 
        reverse-and-complement sequence.bytes ; sequence.clear
        sequence.write block sequence-start block.size        
        block = #[]                     
    sequence.write block 0 block.size    
  reverse-and-complement sequence.bytes ; sequence.clear

reverse-and-complement sequence/ByteArray -> none:
  front-pos := (sequence.index-of NL) + 1
  back-pos := sequence.size - 1  
  while front-pos <= back-pos and sequence[front-pos] == NL: front-pos++
  while front-pos <= back-pos and sequence[back-pos] == NL: back-pos--  

  while front-pos <= back-pos: 
    tmp := lut[ sequence[front-pos] ]
    sequence[front-pos] = lut[ sequence[back-pos] ] 
    sequence[back-pos] = tmp                
    while sequence[++front-pos] == NL: continue     
    
//  while sequence[--back-pos] == NL: // Prefix decrement has been removed
    back-pos = back-pos - 1
    while sequence[back-pos] == NL: 
      back-pos = back-pos - 1  
      
  pipe.stdout.write sequence


code-complement-lookup-table -> ByteArray:
  code ::=       "ABCDGHKMNRSTUVWY".to-byte-array
  complement ::= "TVGHCDMKNYSAABWR".to-byte-array  
  OFFSET ::= 'a' - 'A'
  t ::= (string.from_runes (List 128 : it)).to-byte-array
  for i := 0; i < code.size; i++: 
    t[code[i]] = complement[i]   
    t[code[i] + OFFSET] = complement[i] 
    
  return t

    