import clbg_io as io
import gleam/list
import gleam/string

fn digits(n: Int, q: Int, r: Int, t: Int, k: Int, printed: Int, line: String, lines: List(BitArray)) -> List(BitArray) {
  case printed == n {
    True -> list.reverse(lines)
    False -> {
      let numerator3 = 3 * q + r
      let numerator4 = 4 * q + r
      let digit3 = numerator3 / t
      let digit4 = numerator4 / t
      case digit3 == digit4 {
        True -> {
          let printed = printed + 1
          let line = line <> io.integer(digit3)
          let #(line, lines) = case printed % 10 == 0 || printed == n {
            True -> {
              let output = line <> string.repeat(" ", { 10 - printed % 10 } % 10) <> "\t:" <> io.integer(printed) <> "\n"
              #("", [<<output:utf8>>, ..lines])
            }
            False -> #(line, lines)
          }
          digits(n, q * 10, { r - digit3 * t } * 10, t, k, printed, line, lines)
        }
        False -> {
          let factor = 2 * k + 1
          digits(n, q * k, { 2 * q + r } * factor, t * factor, k + 1, printed, line, lines)
        }
      }
    }
  }
}

pub fn main() -> Nil {
  io.write(io.join(digits(io.argument(), 1, 0, 1, 1, 0, "", [])))
}
