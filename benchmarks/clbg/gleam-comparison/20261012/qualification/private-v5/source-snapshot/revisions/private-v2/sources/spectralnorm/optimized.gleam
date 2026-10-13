import clbg_io as io
import gleam/list
import clbg_process as process

fn row(values: List(Float), i: Int, j: Int, transpose: Bool, total: Float) -> Float {
  case values {
    [] -> total
    [value, ..rest] -> {
      let diagonal = i + j
      let offset = case transpose { True -> j False -> i }
      let denominator = diagonal * { diagonal + 1 } / 2 + offset + 1
      row(rest, i, j + 1, transpose, total +. value /. io.float(denominator))
    }
  }
}

fn rows(values: List(Float), i: Int, n: Int, transpose: Bool, output: List(Float)) -> List(Float) {
  case i == n {
    True -> list.reverse(output)
    False -> rows(values, i + 1, n, transpose, [row(values, i, 0, transpose, 0.0), ..output])
  }
}

fn launch(values: List(Float), n: Int, transpose: Bool, worker: Int, count: Int) -> List(process.Task(List(Float))) {
  case worker == count {
    True -> []
    False -> {
      let first = n * worker / count
      let last = n * { worker + 1 } / count
      let task = process.start(fn() { rows(values, first, last, transpose, []) })
      [task, ..launch(values, n, transpose, worker + 1, count)]
    }
  }
}

fn parallel(values: List(Float), n: Int, transpose: Bool) -> List(Float) {
  let count = case n < 8 { True -> n False -> 8 }
  launch(values, n, transpose, 0, count) |> list.map(process.join) |> list.flatten
}

fn multiply(values: List(Float), n: Int) -> List(Float) {
  parallel(parallel(values, n, False), n, True)
}

fn repeat(count: Int, u: List(Float), v: List(Float), n: Int) -> #(List(Float), List(Float)) {
  case count == 0 {
    True -> #(u, v)
    False -> {
      let v = multiply(u, n)
      repeat(count - 1, multiply(v, n), v, n)
    }
  }
}

fn sums(u: List(Float), v: List(Float), uv: Float, vv: Float) -> Float {
  case u, v {
    [a, ..us], [b, ..vs] -> sums(us, vs, uv +. a *. b, vv +. b *. b)
    _, _ -> io.sqrt(uv /. vv)
  }
}

pub fn main() -> Nil {
  let n = io.argument()
  let #(u, v) = repeat(10, list.repeat(1.0, n), list.repeat(0.0, n), n)
  io.text(io.decimal(sums(u, v, 0.0, 0.0), 9) <> "\n")
}
