# The Computer Language Benchmarks Game
# https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
#
# contributed by Jesse Millikan
# Modified by Wesley Moxam and Michael Klaus
# Modified by  Chris Houhoulis:
#   trigger GC; move stretch_tree into loop; use while instead of a block
#  *reset* 

def item_check(left, right)
  return 1 if left.nil?
  1 + item_check(*left) + item_check(*right)
end

def bottom_up_tree(depth)
  return [nil, nil] if depth == 0
  depth -= 1
  [bottom_up_tree(depth), bottom_up_tree(depth)]
end

max_depth = ARGV[0].to_i
min_depth = 4

max_depth = [min_depth + 2, max_depth].max

1.times do
  stretch_depth = max_depth + 1
  stretch_tree = bottom_up_tree(stretch_depth)
  puts "stretch tree of depth #{stretch_depth}\t check: " +
    "#{item_check(*stretch_tree)}"
end
GC.start

long_lived_tree = bottom_up_tree(max_depth)

base_depth = max_depth + min_depth
min_depth.step(max_depth, 2) do |depth|
  iterations = 2 ** (base_depth - depth)

  check, i = 0, 1

  while i <= iterations
    temp_tree = bottom_up_tree(depth)
    check += item_check(*temp_tree)
    i += 1
  end

  puts "#{iterations}\t trees of depth #{depth}\t check: #{check}"
end
GC.start

puts "long lived tree of depth #{max_depth}\t check: " +
  "#{item_check(*long_lived_tree)}"
