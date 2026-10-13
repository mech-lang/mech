class Node
  property left : Node?
  property right : Node?
  def initialize(@left : Node? = nil, @right : Node? = nil)
    AUDIT_LOCK.synchronize do
      AUDIT_NODES << self
      AUDIT.puts "NODE|#{object_id}|#{@left.try(&.object_id) || 0_u64}|#{@right.try(&.object_id) || 0_u64}"
    end
  end
  def check : Int64
    left = @left
    right = @right
    left && right ? 1_i64 + left.check + right.check : 1_i64
  end
end
# Isolated unscored audit only. Retain bounded constructor identities so the GC
# cannot reuse a collected object's address while the forest is reconciled.
AUDIT_PATH = ENV["MECH_CRYSTAL_WORK_TRACE"]
raise "Existing audit path" if File.exists?(AUDIT_PATH)
AUDIT = File.open(AUDIT_PATH, "w")
AUDIT_LOCK = Mutex.new
AUDIT_NODES = [] of Node
AUDIT.puts "TRACE_C1|binarytrees|compact|#{(ARGV[0]? || "10").to_i}"
def tree(depth : Int32) : Node
  depth == 0 ? Node.new : Node.new(tree(depth - 1), tree(depth - 1))
end
def stretch_check(depth)
  root = tree(depth)
  result = root.check
  AUDIT_LOCK.synchronize { AUDIT.puts "STRETCH|#{root.object_id}|#{depth}|#{result}" }
  result
end
def check_many(depth, iterations)
  total = 0_i64
  iterations.times do |index|
    root = tree(depth)
    result = root.check
    total += result
    AUDIT_LOCK.synchronize { AUDIT.puts "TREE|#{depth}|#{index}|#{root.object_id}|#{result}" }
  end
  total
end

maximum = Math.max(6, (ARGV[0]? || "10").to_i)
puts "stretch tree of depth #{maximum + 1}\t check: #{stretch_check(maximum + 1)}"
long_lived = tree(maximum)
AUDIT_LOCK.synchronize { AUDIT.puts "LONG_START|#{long_lived.object_id}|#{maximum}" }
4.step(to: maximum, by: 2) do |depth|
  iterations = 1_i64 << (maximum - depth + 4)
  puts "#{iterations}\t trees of depth #{depth}\t check: #{check_many(depth, iterations)}"
end
AUDIT_LOCK.synchronize { AUDIT.puts "LONG_END|#{long_lived.object_id}|#{maximum}|#{long_lived.check}" }
puts "long lived tree of depth #{maximum}\t check: #{long_lived.check}"

AUDIT.puts "END_C1"
AUDIT.close
