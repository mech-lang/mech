
/* The Computer Language Benchmarks game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   Naive transliteration from bearophile's program
   contributed by Isaac Gouy 
*/  

import io.stdio
import io
    
seq_lines -> List :  
  lines := []
  in := stdio.stdin 
  while line := in.read-line :    
    if line.starts-with ">THREE" : break   
  while line := in.read-line :  
    if line.starts-with ">" : break    
    if not line.starts-with ";": lines.add line  
  return lines
  
base_counts bases/int seq/string -> Map :  
  counts ::= {:} 
  size ::= seq.size + 1 - bases
  for i := 0; i < size; ++i: 
    nucleo := seq[i..i+bases]
    if v := counts.get nucleo : 
      counts[nucleo] += 1    
    else:
      counts[nucleo] = 1       
  return counts 
  
sorted_freq bases/int seq/string :
  kv ::= base_counts bases seq
  size ::= seq.size + 1 - bases
  items := []; kv.do: | k v | items.add [k,v]
  items.sort --in-place: | a b | a[1] == b[1] ? (a[0].compare-to b[0]) : (a[1] < b[1] ? 1 : -1)
  items.do: it[1] = 100.0 * it[1] / size
  return items
  
specific_count code/string seq/string -> int :   
  return (base_counts code.size seq).get code 
      --if-absent=: | _ | 0 
      
main args:
  lines := seq_lines
  seq := (lines.map: it.to-ascii-upper).join ""
  
  [1,2].do: | base |
    (sorted_freq base seq).do: | kv |
      print "$(kv[0]) $(%.3f kv[1])"
    print

  ["GGT", "GGTA", "GGTATT", "GGTATTTTAATT", 
      "GGTATTTTAATTTATAGT"].do: | code |
    print "$(specific_count code seq)\t$(code)"
    
    