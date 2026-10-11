# The Computer Language Benchmarks Game
# https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
#
# contributed by Tupteq
# modified by Simon Descarpentries
# modified by Ivan Baldin
# 2to3 plus Daniel Nanz fix

import sys
from array import array
from multiprocessing import Pool

def do_row(fy):
    local_abs = abs
    two_over_size = 2.0 / size
    xr_offs = range(7, -1, -1)
    xr_iter = range(50)

    result = array('B')
    for x in range(7, size, 8):
        byte_acc = 0
        for offset in xr_offs:
            z = 0j
            c = two_over_size * (x - offset) + fy

            for i in xr_iter:
                z = z * z + c
                if local_abs(z) >= 2:
                    break
            else:
                byte_acc += 1 << offset

        result.append(byte_acc)

    if x != size - 1:
        result.append(byte_acc)

    return result.tobytes()

def main(out):
    out.write(('P4\n%d %d\n' % (size, size)).encode('ASCII'))

    pool = Pool()
    step = 2.0j / size
    for row in pool.imap(do_row, (step*y-(1.5+1j) for y in range(size))):
        out.write(row)

if __name__ == '__main__':
    size = int(sys.argv[1])
    main(sys.stdout.buffer)
