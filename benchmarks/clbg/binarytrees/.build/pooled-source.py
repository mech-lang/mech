from pathlib import Path
p=Path('benchmarks/clbg/binarytrees')
common='''Leaf(depth<u64>) = (root<ref>) :=
  root := heap/new-refs((heap/null(), heap/null())).
Branch(depth<u64>) = (root<ref>) :=
  left := Make(depth - 1)
  right := Make(depth - 1)
  root := heap/new-refs((left, right)).
Make(depth<u64>) = (root<ref>) :=
  root := control/if(depth == 0, Leaf, Branch, depth).
One(left<ref>, right<ref>) = (count<u64>) :=
  count := 1.
CountChildren(left<ref>, right<ref>) = (count<u64>) :=
  count := 1 + Count(left) + Count(right).
Count(root<ref>) = (count<u64>) :=
  (left, right) := heap/read-refs(root, 2)
  count := control/if(left == heap/null(), One, CountChildren, (left, right)).
Tree(depth<u64>) = (check<u64>) :=
  root := Make(depth)
  check := Count(root).
More(index<u64>, end<u64>, depth<u64>, sum<u64>) = (more<bool>) :=
  more := index < end.
Step(index<u64>, end<u64>, depth<u64>, sum<u64>) = (next<u64>, stop<u64>, d<u64>, check<u64>) :=
  count := memory/with-frame(Tree, depth)
  next := index + 1
  stop := end
  d := depth
  check := sum + count.
AllocatedChunk(start<u64>, end<u64>, depth<u64>) = (check<u64>) :=
  (index, stop, d, check) := control/while((start, end, depth, 0), More, Step).
Chunk(start<u64>, end<u64>, depth<u64>) = (check<u64>) :=
  capacity := number/shift-left(1, depth + 1) - 1
  check := memory/with-ref-arena(capacity, 2, AllocatedChunk, (start, end, depth)).
MoreDepths(depth<u64>, maximum<u64>) = (more<bool>) :=
  more := depth <= maximum.
Depth(depth<u64>, maximum<u64>) = (next<u64>, stop<u64>) :=
  iterations := number/shift-left(1, maximum - depth + 4)
  CHECK
  printed := io/print("{}\\t trees of depth {}\\t check: {}\\n", iterations, depth, check)
  next := depth + 2
  stop := maximum.
Long(maximum<u64>) = (check<u64>) :=
  long-lived := Make(maximum)
  (done, stop) := control/while((4, maximum), MoreDepths, Depth)
  check := Count(long-lived).
n := number/bounded(io/argument-u64(1), 0, 21)
maximum := number/max(6, n)
stretch-count := memory/with-ref-arena(number/shift-left(1, maximum + 2) - 1, 2, Tree, maximum + 1)
stretch-line := io/print("stretch tree of depth {}\\t check: {}\\n", maximum + 1, stretch-count)
long-count := memory/with-ref-arena(number/shift-left(1, maximum + 1) - 1, 2, Long, maximum)
long-line := io/print("long lived tree of depth {}\\t check: {}\\n", maximum, long-count)
'''
(p/'binarytrees.mec').write_text(common.replace('CHECK','check := Chunk(0, iterations, depth)'))
(p/'binarytrees-parallel.mec').write_text(common.replace('CHECK','check := parallel/reduce-ranges(0, iterations, number/max(1, iterations / 64), Chunk, depth, "sum")'))
