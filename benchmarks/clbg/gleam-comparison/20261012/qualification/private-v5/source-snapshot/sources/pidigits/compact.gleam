import clbg_io as io
import gleam/string

fn digits(n: Int, q: Int, r: Int, t: Int, k: Int, printed: Int, line: String) -> Nil {
  case printed == n {
    True -> Nil
    False -> {
      let numerator3 = 3 * q + r
      let numerator4 = 4 * q + r
      let digit3 = numerator3 / t
      let digit4 = numerator4 / t
      case digit3 == digit4 {
        True -> {
          let printed = printed + 1
          let line = line <> io.integer(digit3)
          let line = case printed % 10 == 0 || printed == n {
            True -> {
              io.text(line <> string.repeat(" ", { 10 - printed % 10 } % 10) <> "\t:" <> io.integer(printed) <> "\n")
              ""
            }
            False -> line
          }
          digits(n, q * 10, { r - digit3 * t } * 10, t, k, printed, line)
        }
        False -> {
          let factor = 2 * k + 1
          digits(n, q * k, { 2 * q + r } * factor, t * factor, k + 1, printed, line)
        }
      }
    }
  }
}

pub fn main() -> Nil {
  digits(io.argument(), 1, 0, 1, 1, 0, "")
}
