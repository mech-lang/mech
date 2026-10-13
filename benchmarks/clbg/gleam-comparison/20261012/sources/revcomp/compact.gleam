import clbg_io as io
import gleam/list

fn complement(value: Int) -> Int {
  let value = case value >= 97 && value <= 122 { True -> value - 32 False -> value }
  case value {
    65 -> 84
    67 -> 71
    66 -> 86
    68 -> 72
    71 -> 67
    72 -> 68
    75 -> 77
    77 -> 75
    78 -> 78
    83 -> 83
    82 -> 89
    85 -> 65
    84 -> 65
    87 -> 87
    86 -> 66
    89 -> 82
    _ -> 0
  }
}

fn line(sequence: BitArray, index: Int, count: Int, output: List(Int)) -> #(BitArray, Int) {
  case count == 0 || index < 0 {
    True -> #(io.bytes(list.reverse([10, ..output])), index)
    False -> line(sequence, index - 1, count - 1, [complement(io.byte(sequence, index)), ..output])
  }
}

fn body(sequence: BitArray, index: Int) -> Nil {
  case index < 0 {
    True -> Nil
    False -> {
      let #(bytes, index) = line(sequence, index, 60, [])
      io.write(bytes)
      body(sequence, index)
    }
  }
}

fn flush(header: BitArray, pieces: List(BitArray)) -> Nil {
  case io.size(header) > 0 {
    True -> io.write(<<header:bytes, 10>>)
    False -> Nil
  }
  let sequence = io.join(list.reverse(pieces))
  body(sequence, io.size(sequence) - 1)
}

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

fn records(data: BitArray, offset: Int, header: BitArray, pieces: List(BitArray)) -> Nil {
  case offset == io.size(data) {
    True -> flush(header, pieces)
    False -> {
      let end = end_of_line(data, offset)
      let line = io.slice(data, offset, end - offset)
      let next = next_line(data, end)
      case io.size(line) == 0 {
        True -> records(data, next, header, pieces)
        False -> case io.byte(line, 0) {
          62 -> {
            flush(header, pieces)
            records(data, next, line, [])
          }
          59 -> records(data, next, header, pieces)
          _ -> records(data, next, header, case io.size(header) > 0 { True -> [line, ..pieces] False -> pieces })
        }
      }
    }
  }
}

pub fn main() -> Nil {
  records(io.read_all(), 0, <<>>, [])
}
