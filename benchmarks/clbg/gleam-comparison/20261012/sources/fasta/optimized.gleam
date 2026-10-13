import clbg_io as io
import gleam/list

const alu = "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCAGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCCAGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA"

fn cumulative(values: List(#(Int, Float)), sum: Float) -> List(#(Int, Float)) {
  case values {
    [] -> []
    [#(base, probability), ..rest] -> {
      let total = sum +. probability
      [#(base, total), ..cumulative(rest, total)]
    }
  }
}

fn choose(value: Float, distribution: List(#(Int, Float))) -> Int {
  case distribution {
    [#(base, bound)] -> base
    [#(base, bound), ..rest] -> case value <. bound {
      True -> base
      False -> choose(value, rest)
    }
    [] -> panic as "Empty probability distribution"
  }
}

fn random_line(count: Int, seed: Int, distribution: List(#(Int, Float)), output: List(Int)) -> #(BitArray, Int) {
  case count == 0 {
    True -> #(io.bytes(list.reverse([10, ..output])), seed)
    False -> {
      let seed = { seed * 3877 + 29573 } % 139968
      let value = io.float(seed) /. 139968.0
      random_line(count - 1, seed, distribution, [choose(value, distribution), ..output])
    }
  }
}

fn random(count: Int, seed: Int, distribution: List(#(Int, Float)), output: List(BitArray)) -> #(Int, List(BitArray)) {
  case count == 0 {
    True -> #(seed, list.reverse(output))
    False -> {
      let width = case count < 60 { True -> count False -> 60 }
      let #(line, seed) = random_line(width, seed, distribution, [])
      random(count - width, seed, distribution, [line, ..output])
    }
  }
}

fn repeat_line(count: Int, index: Int, data: BitArray, output: List(Int)) -> #(BitArray, Int) {
  case count == 0 {
    True -> #(io.bytes(list.reverse([10, ..output])), index)
    False -> repeat_line(count - 1, { index + 1 } % io.size(data), data, [io.byte(data, index), ..output])
  }
}

fn repeated(count: Int, index: Int, data: BitArray, output: List(BitArray)) -> List(BitArray) {
  case count == 0 {
    True -> list.reverse(output)
    False -> {
      let width = case count < 60 { True -> count False -> 60 }
      let #(line, index) = repeat_line(width, index, data, [])
      repeated(count - width, index, data, [line, ..output])
    }
  }
}

pub fn main() -> Nil {
  let n = io.argument()
  let iub = cumulative([#(97, 0.27), #(99, 0.12), #(103, 0.12), #(116, 0.27), #(66, 0.02), #(68, 0.02), #(72, 0.02), #(75, 0.02), #(77, 0.02), #(78, 0.02), #(82, 0.02), #(83, 0.02), #(86, 0.02), #(87, 0.02), #(89, 0.02)], 0.0)
  let homo = cumulative([#(97, 0.3029549426680), #(99, 0.1979883004921), #(103, 0.1975473066391), #(116, 0.3015094502008)], 0.0)
  io.text(">ONE Homo sapiens alu\n")
  io.write(io.join(repeated(2 * n, 0, <<alu:utf8>>, [])))
  io.text(">TWO IUB ambiguity codes\n")
  let #(seed, lines) = random(3 * n, 42, iub, [])
  io.write(io.join(lines))
  io.text(">THREE Homo sapiens frequency\n")
  let #(_, lines) = random(5 * n, seed, homo, [])
  io.write(io.join(lines))
  Nil
}
