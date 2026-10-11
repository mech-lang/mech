# Equal-size distinct two-slot records in a growable standard array arena.
import sys
from array import array
class Tree:
    def __init__(self, depth):
        self.nodes = array("i")
        self.root = self.build(depth)
    def build(self, depth):
        left = -1
        right = -1
        if depth:
            left = self.build(depth-1)
            right = self.build(depth-1)
        index = len(self.nodes)
        self.nodes.append(left)
        self.nodes.append(right)
        return index
    def check(self, index=None):
        if index is None:
            index = self.root
        left = self.nodes[index]
        if left == -1:
            return 1
        return 1+self.check(left)+self.check(self.nodes[index+1])
maximum = max(6,int(sys.argv[1]))
stretch = maximum+1
print("stretch tree of depth %d\t check: %d" % (stretch,Tree(stretch).check()))
long_lived = Tree(maximum)
for depth in range(4,maximum+1,2):
    count = 1<<(maximum-depth+4)
    total = 0
    for iteration in range(count):
        total += Tree(depth).check()
    print("%d\t trees of depth %d\t check: %d" % (count,depth,total))
print("long lived tree of depth %d\t check: %d" % (maximum,long_lived.check()))
