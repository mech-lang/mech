# The Computer Language Benchmarks Game
# https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
#
# contributed by Antoine Pitrou
# modified by Dominique Wahli and Daniel Nanz
# modified by Joerg Baumann
# modified by Jonathan Ultis

import sys
import multiprocessing as mp

def make_tree(dd):
    if dd > 0:
        return (make_tree(dd-1), make_tree(dd-1))
    return (None, None)

def check_tree(node):
    l, r = node
    if l is None:
        return 1
    else:
        return 1 + check_tree(l) + check_tree(r)

def make_check(dd, make=make_tree, check=check_tree):
    return check(make(dd))

def main(n, min_depth=4):

    max_depth = max(min_depth + 2, n)
    stretch_depth = max_depth + 1

    if mp.cpu_count() > 1:
        pool = mp.Pool()
        chunkmap = pool.map
    else:
        chunkmap = map

    print('stretch tree of depth {0}\t check: {1}'.format(
          stretch_depth, make_check(stretch_depth)))

    long_lived_tree = make_tree(max_depth)

    mmd = max_depth + min_depth
    for dd in range(min_depth, stretch_depth, 2):
        ii = 2 ** (mmd - dd)
        cs = sum(chunkmap(make_check, (dd,)*ii))
        print('{0}\t trees of depth {1}\t check: {2}'.format(ii, dd, cs))

    print('long lived tree of depth {0}\t check: {1}'.format(
          max_depth, check_tree(long_lived_tree)))


if __name__ == '__main__':
    main(int(sys.argv[1]))
