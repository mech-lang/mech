# The Computer Language Benchmarks Game
# https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
#
# contributed by Jacob Lee, Steven Bethard, et al
# 2to3, fixed by Daniele Varrazzo
# modified by Daniel Nanz

import sys


def show(seq, table=bytes.maketrans(b'ACBDGHKMNSRUTWVYacbdghkmnsrutwvy',
                                    b'TGVHCDMKNSYAAWBRTGVHCDMKNSYAAWBR'),
         write=sys.stdout.buffer.write, nl=b'\n'):
    
    [header, s] = seq.split(nl, 1)
    s = s.translate(table, nl)[: : -1]
    
    write(b'>' + header + nl)
    for i in range(0, len(s), 60):
        write(s[i : i + 60] + nl)



def main():
    
    sys.stdin = sys.stdin.detach()
    seqs = b''.join([line for line in sys.stdin]).split(b'>')[1 : ]
    
    for seq in seqs:
        show(seq)        


main()
