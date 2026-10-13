# The Computer Language Benchmarks Game
# http://benchmarksgame.alioth.debian.org/

# Contributed by Akzhan Abdulin.
# Based on the Java program from Heikki Salokanto, Chandra Sekar, Mike Krüger.

puts "stuk4"
n = (ARGV[0]? || 0).to_i
min_depth = 4
max_depth = Math.max min_depth + 2, n
stretch_depth = max_depth + 1
check = TreeNode.create(0, stretch_depth).check

puts "stretch tree of depth #{max_depth + 1}\t check: #{check}"

long_lived_tree = TreeNode.create 0, max_depth
min_depth.step(to: max_depth, by: 2) do |depth|
  iterations = 1 << (max_depth - depth + min_depth)
  check = 0

  1.upto(iterations) do |i|
    check += TreeNode.create(i, depth).check
    check += TreeNode.create(-i, depth).check
  end
  puts "#{iterations << 1}\t trees of depth #{depth}\t check: #{check}"
end

puts "long lived tree of depth #{max_depth}\t check: #{long_lived_tree.check}"

class TreeNode
  property left : TreeNode?
  property right : TreeNode?
  property item : Int32

  def self.create(item, depth) : TreeNode
    TreeNode.new item, depth - 1
  end

  def initialize(@item, depth = 0)
    if depth > 0
      self.left = TreeNode.new 2 * item - 1, depth - 1
      self.right = TreeNode.new 2 * item, depth - 1
    end
  end

  def check
    return item if (lft = left).nil?
    return item if (rgt = right).nil?
    lft.check - rgt.check + item
  end
end
