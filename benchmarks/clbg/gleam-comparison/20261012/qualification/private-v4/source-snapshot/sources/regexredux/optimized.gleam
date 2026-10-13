import clbg_io as io
import clbg_regex as regex
import gleam/list
import clbg_process as process

fn count(pattern: regex.Pattern, data: BitArray, offset: Int, total: Int) -> Int {
  case regex.search(pattern, data, offset) {
    Error(_) -> total
    Ok(#(start, width)) -> {
      let assert True = width > 0
      count(pattern, data, start + width, total + 1)
    }
  }
}

fn replace(pattern: regex.Pattern, replacement: BitArray, data: BitArray, offset: Int, parts: List(BitArray)) -> BitArray {
  case regex.search(pattern, data, offset) {
    Error(_) -> io.join(list.reverse([io.slice(data, offset, io.size(data) - offset), ..parts]))
    Ok(#(start, width)) -> {
      let assert True = width > 0
      let prefix = io.slice(data, offset, start - offset)
      replace(pattern, replacement, data, start + width, [replacement, prefix, ..parts])
    }
  }
}

fn substitutions(data: BitArray, rules: List(#(String, String))) -> BitArray {
  case rules {
    [] -> data
    [#(source, replacement), ..rest] -> substitutions(replace(regex.compile(source), <<replacement:utf8>>, data, 0, []), rest)
  }
}

fn patterns() -> List(String) {
  ["agggtaaa|tttaccct", "[cgt]gggtaaa|tttaccc[acg]", "a[act]ggtaaa|tttacc[agt]t", "ag[act]gtaaa|tttac[agt]ct", "agg[act]taaa|ttta[agt]cct", "aggg[acg]aaa|ttt[cgt]ccct", "agggt[cgt]aa|tt[acg]accct", "agggta[cgt]a|t[acg]taccct", "agggtaa[cgt]|[acg]ttaccct"]
}

pub fn main() -> Nil {
  let input = io.read_all()
  let cleaned = replace(regex.compile(">[^\n]*\n|\n"), <<>>, input, 0, [])
  let sources = patterns()
  let tasks = list.map(list.take(sources, 7), fn(source) {
    process.start(fn() { count(regex.compile(source), cleaned, 0, 0) })
  })
  let local = list.map(list.drop(sources, 7), fn(source) { count(regex.compile(source), cleaned, 0, 0) })
  let counts = list.append(list.map(tasks, process.join), local)
  list.each(list.zip(sources, counts), fn(entry) {
    io.text(entry.0 <> " " <> io.integer(entry.1) <> "\n")
  })
  let final = substitutions(cleaned, [#("tHa[Nt]", "<4>"), #("aND|caN|Ha[DS]|WaS", "<3>"), #("a[NSt]|BY", "<2>"), #("<[^>]*>", "|"), #("\\|[^|][^|]*\\|", "-")])
  io.text("\n" <> io.integer(io.size(input)) <> "\n" <> io.integer(io.size(cleaned)) <> "\n" <> io.integer(io.size(final)) <> "\n")
}
