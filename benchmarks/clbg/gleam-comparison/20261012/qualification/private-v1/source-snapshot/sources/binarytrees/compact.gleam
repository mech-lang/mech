import clbg_io as io
import clbg_reference as reference

type Tree {
  Leaf(reference.Reference)
  Node(reference.Reference, Tree, Tree)
}

fn make(depth: Int) -> Tree {
  let identity = reference.new()
  case depth == 0 {
    True -> Leaf(identity)
    False -> Node(identity, make(depth - 1), make(depth - 1))
  }
}

fn check(tree: Tree) -> Int {
  case tree {
    Leaf(_) -> 1
    Node(_, left, right) -> 1 + check(left) + check(right)
  }
}

fn sum(count: Int, depth: Int, total: Int) -> Int {
  case count == 0 {
    True -> total
    False -> sum(count - 1, depth, total + check(make(depth)))
  }
}

fn power_two(exponent: Int, value: Int) -> Int {
  case exponent == 0 {
    True -> value
    False -> power_two(exponent - 1, value * 2)
  }
}

fn depths(depth: Int, maximum: Int) -> Nil {
  case depth > maximum {
    True -> Nil
    False -> {
      let count = power_two(maximum - depth + 4, 1)
      let total = sum(count, depth, 0)
      io.text(io.integer(count) <> "\t trees of depth " <> io.integer(depth) <> "\t check: " <> io.integer(total) <> "\n")
      depths(depth + 2, maximum)
    }
  }
}

pub fn main() -> Nil {
  let requested = io.argument()
  let maximum = case requested < 6 { True -> 6 False -> requested }
  io.text("stretch tree of depth " <> io.integer(maximum + 1) <> "\t check: " <> io.integer(check(make(maximum + 1))) <> "\n")
  let long_lived = make(maximum)
  depths(4, maximum)
  io.text("long lived tree of depth " <> io.integer(maximum) <> "\t check: " <> io.integer(check(long_lived)) <> "\n")
}
