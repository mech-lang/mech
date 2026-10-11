# The Computer Language Benchmarks Game
# https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
#
# submitted by Ian Osgood
# modified by Sokolov Yura
# modified by bearophile
# modified by xfm for parallelization
# modified by Justin Peel 

from sys import stdin
from collections import defaultdict
from multiprocessing import Pool

def gen_freq(frame) :
    global sequence
    frequences = defaultdict(int)
    if frame != 1:
        for ii in range(len(sequence)-frame+1) :
            frequences[sequence[ii:ii+frame]] += 1
    else:
        for nucleo in sequence:
            frequences[nucleo] += 1
    return frequences


def sort_seq(length):
    frequences = gen_freq(length)
    n= sum(frequences.values())

    l = sorted(list(frequences.items()), reverse=True, key=lambda seq_freq: (seq_freq[1],seq_freq[0]))

    return '\n'.join("%s %.3f" % (st, 100.0*fr/n) for st,fr in l)


def find_seq(s):
    t = gen_freq(len(s))
    return (s,t.get(s,0))

def prepare(f=stdin) :
    for line in f:
        if line[0] in ">;":
            if line[1:3] == "TH":
                break

    seq = []
    app = seq.append
    for line in f:
        if line[0] in ">;":
            break
        app( line )
    return "".join(seq).upper().replace('\n','')

def init(arg):
    global sequence
    sequence = arg

def main():
    global sequence
    sequence = prepare()
    p=Pool()

    
    res2 = p.map_async(find_seq,reversed("GGT GGTA GGTATT GGTATTTTAATT GGTATTTTAATTTATAGT".split()))
    res1 = p.map_async(sort_seq,(1,2))
    
    for s in res1.get() : print (s+'\n')
    res2 = reversed([r for r in res2.get()])
    print ("\n".join("{1:d}\t{0}".format(*s) for s in res2))

if __name__=='__main__' :
    main()
