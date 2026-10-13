import clbg_io as io
import gleam/dict
import gleam/bit_array
import gleam/list
import clbg_process as process
import gleam/order.{Eq, Gt, Lt}

fn end_of_line(data: BitArray, offset: Int) -> Int {
  case offset == io.size(data) {
    True -> offset
    False -> case io.byte(data, offset) == 10 || io.byte(data, offset) == 13 {
      True -> offset
      False -> end_of_line(data, offset + 1)
    }
  }
}

fn next_line(data: BitArray, end: Int) -> Int {
  let next = case end < io.size(data) && io.byte(data, end) == 13 { True -> end + 1 False -> end }
  case next < io.size(data) && io.byte(data, next) == 10 { True -> next + 1 False -> next }
}

fn records(data: BitArray, offset: Int, selected: Bool, pieces: List(BitArray)) -> BitArray {
  case offset == io.size(data) {
    True -> io.join(list.reverse(pieces))
    False -> {
      let end = end_of_line(data, offset)
      let line = io.slice(data, offset, end - offset)
      let next = next_line(data, end)
      case io.size(line) == 0 {
        True -> records(data, next, selected, pieces)
        False -> case io.byte(line, 0) {
          62 -> case selected {
            True -> io.join(list.reverse(pieces))
            False -> {
              let width = io.size(line)
              let selected = width >= 6 && io.slice(line, 0, 6) == <<">THREE":utf8>> && { width == 6 || io.byte(line, 6) == 32 || io.byte(line, 6) == 9 }
              records(data, next, selected, pieces)
            }
          }
          59 -> records(data, next, selected, pieces)
          _ -> records(data, next, selected, case selected { True -> [line, ..pieces] False -> pieces })
        }
      }
    }
  }
}

fn code(value: Int) -> Int {
  case value {
    65 | 97 -> 0
    67 | 99 -> 1
    71 | 103 -> 2
    84 | 116 -> 3
    _ -> panic as "Invalid nucleotide"
  }
}

fn encode(sequence: BitArray, offset: Int, width: Int, key: Int) -> Int {
  case width == 0 {
    True -> key
    False -> encode(sequence, offset + 1, width - 1, key * 4 + code(io.byte(sequence, offset)))
  }
}

fn power_four(width: Int, total: Int) -> Int {
  case width == 0 { True -> total False -> power_four(width - 1, total * 4) }
}

fn histogram(sequence: BitArray, width: Int, index: Int, key: Int, modulus: Int, counts: dict.Dict(Int, Int)) -> dict.Dict(Int, Int) {
  case index == io.size(sequence) {
    True -> counts
    False -> {
      let key = { key * 4 + code(io.byte(sequence, index)) } % modulus
      let counts = case index + 1 >= width {
        True -> {
          let existing = case dict.get(counts, key) { Ok(value) -> value Error(_) -> 0 }
          dict.insert(counts, key, existing + 1)
        }
        False -> counts
      }
      histogram(sequence, width, index + 1, key, modulus, counts)
    }
  }
}

fn decode(key: Int, width: Int, output: List(Int)) -> String {
  case width == 0 {
    True -> { let assert Ok(text) = bit_array.to_string(io.bytes(output)) text }
    False -> decode(key / 4, width - 1, [io.byte(<<"ACGT":utf8>>, key % 4), ..output])
  }
}

fn compare(a: #(Int, Int), b: #(Int, Int)) -> order.Order {
  case a.1 > b.1 { True -> Lt False -> case a.1 < b.1 {
    True -> Gt
    False -> case a.0 < b.0 { True -> Lt False -> case a.0 > b.0 { True -> Gt False -> Eq } }
  } }
}

fn frequency(counts: dict.Dict(Int, Int), width: Int) -> Nil {
  let total = dict.fold(counts, 0, fn(total, _, value) { total + value })
  list.each(list.sort(dict.to_list(counts), compare), fn(entry) {
    io.text(decode(entry.0, width, []) <> " " <> io.decimal(100.0 *. io.float(entry.1) /. io.float(total), 3) <> "\n")
  })
  io.text("\n")
}

fn query(counts: dict.Dict(Int, Int), word: String) -> Nil {
  let bytes = <<word:utf8>>
  let key = encode(bytes, 0, io.size(bytes), 0)
  let count = case dict.get(counts, key) { Ok(value) -> value Error(_) -> 0 }
  io.text(io.integer(count) <> "\t" <> word <> "\n")
}

pub fn main() -> Nil {
  let sequence = records(io.read_all(), 0, False, [])
  let tasks = list.map([1, 2, 3, 4, 6, 12, 18], fn(width) {
    process.start(fn() { histogram(sequence, width, 0, 0, power_four(width, 1), dict.new()) })
  })
  let counts = list.map(tasks, process.join)
  let assert [one, two, three, four, six, twelve, eighteen] = counts
  frequency(one, 1)
  frequency(two, 2)
  query(three, "GGT")
  query(four, "GGTA")
  query(six, "GGTATT")
  query(twelve, "GGTATTTTAATT")
  query(eighteen, "GGTATTTTAATTTATAGT")
}
