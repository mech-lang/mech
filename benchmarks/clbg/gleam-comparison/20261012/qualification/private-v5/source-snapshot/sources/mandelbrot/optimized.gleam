import clbg_io as io
import gleam/list
import clbg_process as process

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

fn rows(n: Int, y: Int, last: Int, output: List(BitArray)) -> BitArray {
  case y == last {
    True -> io.join(list.reverse(output))
    False -> rows(n, y + 1, last, [io.bytes(row(n, y, 0, 0, 0, [])), ..output])
  }
}

fn launch(n: Int, worker: Int, count: Int) -> List(process.Task(BitArray)) {
  case worker == count {
    True -> []
    False -> {
      let first = n * worker / count
      let last = n * (worker + 1) / count
      let task = process.start(fn() { rows(n, first, last, []) })
      [task, ..launch(n, worker + 1, count)]
    }
  }
}

pub fn main() -> Nil {
  let n = io.argument()
  let count = case n < 8 { True -> n False -> 8 }
  io.text("P4\n" <> io.integer(n) <> " " <> io.integer(n) <> "\n")
  launch(n, 0, count) |> list.map(process.join) |> list.each(io.write)
}
