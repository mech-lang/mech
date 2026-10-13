import clbg_io as io
import clbg_vector as vector
import gleam/list
import clbg_process as process

fn integers(i: Int, n: Int) -> List(Int) {
  case i == n { True -> [] False -> [i, ..integers(i + 1, n)] }
}

fn factorials(i: Int, n: Int, product: Int, values: List(Int)) -> List(Int) {
  case i > n {
    True -> list.reverse(values)
    False -> factorials(i + 1, n, product * i, [product * i, ..values])
  }
}

fn rotate_slots(values: vector.Vector(Int), i: Int, width: Int) -> vector.Vector(Int) {
  case i == width - 1 {
    True -> values
    False -> rotate_slots(vector.set(values, i, vector.get(values, i + 1)), i + 1, width)
  }
}

fn rotate(values: vector.Vector(Int), width: Int) -> vector.Vector(Int) {
  let first = vector.get(values, 0)
  vector.set(rotate_slots(values, 0, width), width - 1, first)
}

fn rotations(values: vector.Vector(Int), width: Int, count: Int) -> vector.Vector(Int) {
  case count == 0 { True -> values False -> rotations(rotate(values, width), width, count - 1) }
}

fn initial(width: Int, rank: Int, factorial: vector.Vector(Int), permutation: vector.Vector(Int), counts: vector.Vector(Int)) -> #(vector.Vector(Int), vector.Vector(Int)) {
  case width < 2 {
    True -> #(permutation, counts)
    False -> {
      let shift = rank / vector.get(factorial, width - 1) % width
      initial(width - 1, rank, factorial, rotations(permutation, width, shift), vector.set(counts, width - 1, shift))
    }
  }
}

fn reverse(values: vector.Vector(Int), left: Int, right: Int) -> vector.Vector(Int) {
  case left >= right {
    True -> values
    False -> {
      let a = vector.get(values, left)
      let b = vector.get(values, right)
      reverse(vector.set(vector.set(values, left, b), right, a), left + 1, right - 1)
    }
  }
}

fn flips(values: vector.Vector(Int), total: Int) -> Int {
  let first = vector.get(values, 0)
  case first == 0 { True -> total False -> flips(reverse(values, 0, first), total + 1) }
}

fn next(permutation: vector.Vector(Int), counts: vector.Vector(Int), width: Int, n: Int) -> #(vector.Vector(Int), vector.Vector(Int)) {
  case width > n {
    True -> #(permutation, counts)
    False -> {
      let permutation = rotate(permutation, width)
      let count = vector.get(counts, width - 1) + 1
      case count < width {
        True -> #(permutation, vector.set(counts, width - 1, count))
        False -> next(permutation, vector.set(counts, width - 1, 0), width + 1, n)
      }
    }
  }
}

fn loop(rank: Int, last: Int, n: Int, permutation: vector.Vector(Int), counts: vector.Vector(Int), checksum: Int, maximum: Int) -> #(Int, Int) {
  case rank == last {
    True -> #(checksum, maximum)
    False -> {
      let count = flips(permutation, 0)
      let checksum = checksum + case rank % 2 == 0 { True -> count False -> -count }
      let maximum = case count > maximum { True -> count False -> maximum }
      let #(permutation, counts) = next(permutation, counts, 2, n)
      loop(rank + 1, last, n, permutation, counts, checksum, maximum)
    }
  }
}

fn run(n: Int, first: Int, last: Int) -> #(Int, Int) {
  let factorial = vector.from_list(factorials(1, n, 1, [1]))
  let #(permutation, counts) = initial(n, first, factorial, vector.from_list(integers(0, n)), vector.from_list(list.repeat(0, n)))
  loop(first, last, n, permutation, counts, 0, 0)
}

fn launch(n: Int, total: Int, worker: Int, count: Int) -> List(process.Task(#(Int, Int))) {
  case worker == count {
    True -> []
    False -> {
      let first = total * worker / count
      let last = total * (worker + 1) / count
      let task = process.start(fn() { run(n, first, last) })
      [task, ..launch(n, total, worker + 1, count)]
    }
  }
}

pub fn main() -> Nil {
  let n = io.argument()
  let factorial = vector.from_list(factorials(1, n, 1, [1]))
  let total = vector.get(factorial, n)
  let count = case total < 8 { True -> total False -> 8 }
  let results = list.map(launch(n, total, 0, count), process.join)
  let #(checksum, maximum) = list.fold(results, #(0, 0), fn(acc, value) {
    #(acc.0 + value.0, case acc.1 > value.1 { True -> acc.1 False -> value.1 })
  })
  io.text(io.integer(checksum) <> "\nPfannkuchen(" <> io.integer(n) <> ") = " <> io.integer(maximum) <> "\n")
}
