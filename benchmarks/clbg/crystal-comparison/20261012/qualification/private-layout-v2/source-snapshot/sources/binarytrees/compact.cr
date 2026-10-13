class Node
  property left : Node?
  property right : Node?
  def initialize(@left : Node? = nil, @right : Node? = nil)
  end
  def check : Int64
    left = @left
    right = @right
    left && right ? 1_i64 + left.check + right.check : 1_i64
  end
end
def tree(depth : Int32) : Node
  depth == 0 ? Node.new : Node.new(tree(depth - 1), tree(depth - 1))
end
def stretch_check(depth)
  tree(depth).check
end
def check_many(depth, iterations)
  total = 0_i64
  iterations.times { total += tree(depth).check }
  total
end

maximum = Math.max(6, (ARGV[0]? || "10").to_i)
puts "stretch tree of depth #{maximum + 1}\t check: #{stretch_check(maximum + 1)}"
long_lived = tree(maximum)
4.step(to: maximum, by: 2) do |depth|
  iterations = 1_i64 << (maximum - depth + 4)
  puts "#{iterations}\t trees of depth #{depth}\t check: #{check_many(depth, iterations)}"
end
puts "long lived tree of depth #{maximum}\t check: #{long_lived.check}"
