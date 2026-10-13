import clbg_io as io
import gleam/list

fn inside(cr: Float, ci: Float, zr: Float, zi: Float, remaining: Int) -> Int {
  let zr2 = zr *. zr
  let zi2 = zi *. zi
  case zr2 +. zi2 >. 4.0 {
    True -> 0
    False -> case remaining == 0 {
      True -> 1
      False -> inside(cr, ci, zr2 -. zi2 +. cr, 2.0 *. zr *. zi +. ci, remaining - 1)
    }
  }
}

fn row(n: Int, y: Int, x: Int, bits: Int, used: Int, output: List(Int)) -> List(Int) {
  case x == n {
    True -> {
      let output = case used == 0 {
        True -> output
        False -> [bits * padding(8 - used, 1), ..output]
      }
      list.reverse(output)
    }
    False -> {
      let cr = 2.0 *. io.float(x) /. io.float(n) -. 1.5
      let ci = 2.0 *. io.float(y) /. io.float(n) -. 1.0
      let bits = bits * 2 + inside(cr, ci, 0.0, 0.0, 50)
      case used == 7 {
        True -> row(n, y, x + 1, 0, 0, [bits, ..output])
        False -> row(n, y, x + 1, bits, used + 1, output)
      }
    }
  }
}

fn padding(n: Int, value: Int) -> Int {
  case n == 0 { True -> value False -> padding(n - 1, value * 2) }
}

fn rows(n: Int, y: Int) -> Nil {
  case y == n {
    True -> Nil
    False -> {
      io.write(io.bytes(row(n, y, 0, 0, 0, [])))
      rows(n, y + 1)
    }
  }
}

pub fn main() -> Nil {
  let n = io.argument()
  io.text("P4\n" <> io.integer(n) <> " " <> io.integer(n) <> "\n")
  rows(n, 0)
}
