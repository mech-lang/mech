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

def tree_job(depth : Int32, iterations : Int64, done : Channel(Tuple(Int32, Int64)))
  done.send({depth, check_many(depth, iterations)})
end
maximum = Math.max(6, (ARGV[0]? || "10").to_i)
puts "stretch tree of depth #{maximum + 1}\t check: #{stretch_check(maximum + 1)}"
long_lived = tree(maximum)
Fiber::ExecutionContext.default.resize(8)
depths = [] of Int32
4.step(to: maximum, by: 2) { |depth| depths << depth }
depths.each_slice(8) do |batch|
  done = Channel(Tuple(Int32, Int64)).new(batch.size)
  batch.each do |depth|
    spawn tree_job(depth, 1_i64 << (maximum - depth + 4), done)
  end
  results = Array(Tuple(Int32, Int64)).new(batch.size) { done.receive }
  results.sort_by! { |record| record[0] }
  results.each do |record|
    depth = record[0]
    iterations = 1_i64 << (maximum - depth + 4)
    puts "#{iterations}\t trees of depth #{depth}\t check: #{record[1]}"
  end
end
puts "long lived tree of depth #{maximum}\t check: #{long_lived.check}"
