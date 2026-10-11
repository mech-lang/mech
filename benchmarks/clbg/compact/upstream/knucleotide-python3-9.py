# The Computer Language Benchmarks Game
# https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
#
# submitted by Ian Osgood
# modified by Sokolov Yura
# modified by bearophile
# modified by xfm for parallelization
# modified by Justin Peel 
# modified by Jean-Baptiste Lamy 

from sys import stdin
from collections import defaultdict

def gen_freq(frame):
    global sequence
    frequences = defaultdict(int)
    if frame == 1:
        for nucleo in sequence:
            frequences[nucleo] += 1
    else:
        for ii in range(len(sequence) - frame + 1) :
            frequences[sequence[ii : ii + frame]] += 1
    return frequences

def gen_result(arg):
    if isinstance(arg, int):
        frequences = gen_freq(arg)
        n = sum(frequences.values())
        l = sorted(frequences.items(), reverse = True, key = lambda seq_freq: seq_freq[::-1])
        return "".join("%s %.3f\n" % (st, 100.0 * fr / n) for st, fr in l) + "\n"
    else:
        frequences = gen_freq(len(arg))
        return "%s\t%s\n" % (frequences[arg], arg)

def prepare() :
    for line in stdin:
        if (line[0] == ">") and (line[1:3] == "TH"):
            break
        
    seq = ""
    for line in stdin:
        if line[0] == ">":
            break
        seq += line
    return seq.upper().replace('\n','')

def main():
    global sequence
    sequence = prepare()
    
    from concurrent.futures import ProcessPoolExecutor
    
    with ProcessPoolExecutor() as executor:
        r = executor.map(gen_result, ["GGTATTTTAATTTATAGT", "GGTATTTTAATT", "GGTATT", "GGTA", "GGT", 2, 1])
        
    print("".join(reversed(list(r))), end = "")
    
    
if __name__=='__main__' :
    main()
