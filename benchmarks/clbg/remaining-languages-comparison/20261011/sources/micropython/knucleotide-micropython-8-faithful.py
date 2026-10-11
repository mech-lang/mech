# The Computer Language Benchmarks Game
#   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
#
#   Naive transliteration from bearophile's program
#   contributed by Isaac Gouy 

from sys import stdin

def seq_lines():
    for line in stdin:
        if line.startswith(">THREE"):
            break
    lines = []
    for line in stdin:
        if line.startswith(">"):
            break
        lines.append( line.strip() )       
    return lines
    
def base_counts(bases, seq):  
    counts = {}
    size = len(seq) + 1 - bases
    for i in range(size):  
        nucleo = seq[i: i + bases]  
        if nucleo in counts:   
            counts[nucleo] += 1  
        else:
            counts[nucleo] = 1
    return counts               
    
def sorted_freq(bases, seq):  
    keysValues = base_counts(bases, seq).items()
    size = len(seq) + 1 - bases    
    sorted_ =  sorted(keysValues, key=lambda kv: (-kv[1],kv[0]))     
    return [ (kv[0], 100.0 * kv[1] / size) for kv in sorted_ ]  
      
def specific_count(code, seq):  
    return base_counts(len(code), seq).get(code,0)   
    
def main():
    lines = seq_lines()
    seq = "".join([s.upper() for s in lines])
        
    for base in 1,2:        
        for kv in sorted_freq(base, seq):
           print("%s %.3f" % (kv[0], kv[1]))
        print()      
      
    for code in "GGT", "GGTA", "GGTATT", \
            "GGTATTTTAATT", "GGTATTTTAATTTATAGT":     
        print("%d\t%s" % (specific_count(code, seq), code))       
 
if __name__ == '__main__':
  main()
