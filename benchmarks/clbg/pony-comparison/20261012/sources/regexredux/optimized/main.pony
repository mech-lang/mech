use "collections"
use "regex"

primitive Patterns
  fun apply(): Array[String] val =>
    recover val
      let patterns = Array[String](9)
      patterns.push("agggtaaa|tttaccct")
      patterns.push("[cgt]gggtaaa|tttaccc[acg]")
      patterns.push("a[act]ggtaaa|tttacc[agt]t")
      patterns.push("ag[act]gtaaa|tttac[agt]ct")
      patterns.push("agg[act]taaa|ttta[agt]cct")
      patterns.push("aggg[acg]aaa|ttt[cgt]ccct")
      patterns.push("agggt[cgt]aa|tt[acg]accct")
      patterns.push("agggta[cgt]a|t[acg]taccct")
      patterns.push("agggtaa[cgt]|[acg]ttaccct")
      patterns
    end

primitive Substitutions
  fun apply(): Array[(String, String)] val =>
    recover val
      let substitutions = Array[(String, String)](5)
      substitutions.push(("tHa[Nt]", "<4>"))
      substitutions.push(("aND|caN|Ha[DS]|WaS", "<3>"))
      substitutions.push(("a[NSt]|BY", "<2>"))
      substitutions.push(("<[^>]*>", "|"))
      substitutions.push(("\\|[^|][^|]*\\|", "-"))
      substitutions
    end

actor Worker
  let _parent: Main
  let _sequence: String
  let _patterns: Array[String] val
  let _last: USize
  var _index: USize
  var _count: USize = 0
  var _regex: (Regex | None) = None
  var _iterator: (MatchIterator | None) = None
  new create(parent: Main, sequence: String, patterns: Array[String] val, first: USize, last: USize) =>
    _parent = parent
    _sequence = sequence
    _patterns = patterns
    _index = first
    _last = last
    _start()
  be _start() =>
    try
      let expression = Regex(_patterns(_index)?)?
      _regex = expression
      _iterator = expression.matches(_sequence)
      _count = 0
      _step()
    else
      _parent.failed()
    end
  be _step() =>
    try
      let iterator = _iterator as MatchIterator
      var batch: USize = 0
      while iterator.has_next() and (batch < 1024) do
        iterator.next()?
        _count = _count + 1
        batch = batch + 1
      end
      if iterator.has_next() then
        _step()
      else
        _parent.done(_index, _count)
        _index = _index + 1
        if _index < _last then
          _start()
        end
      end
    else
      _parent.failed()
    end

class iso Input is InputNotify
  let _parent: Main
  new create(parent: Main) =>
    _parent = parent
  fun ref apply(data: Array[U8] iso) =>
    _parent.data(consume data)
  fun ref read_failed() =>
    _parent.failed()
  fun ref dispose() =>
    _parent.finish()

actor Main
  let _env: Env
  let _patterns: Array[String] val = Patterns()
  let _substitutions: Array[(String, String)] val = Substitutions()
  let _counts: Array[USize] = Array[USize].init(0, 9)
  var _input: String iso = recover String end
  var _sequence: String = ""
  var _input_length: USize = 0
  var _clean_length: USize = 0
  var _pending: USize = 0
  var _failed: Bool = false
  new create(env: Env) =>
    _env = env
    env.input(recover Input(this) end, 65536)
  be data(bytes: Array[U8] iso) =>
    _input.append(consume bytes)
  be failed() =>
    _failed = true
    _env.exitcode(1)
  be finish() =>
    if _failed then return end
    let input: String = _input = recover String end
    _input_length = input.size()
    try
      let expression = Regex(">.*\n|\n")?
      _sequence = try expression.replace(input, "" where global = true)? else input end
      _clean_length = _sequence.size()
      _pending = 9
      for worker in Range[USize](0, 8) do
        Worker(this, _sequence, _patterns, (9 * worker) / 8, (9 * (worker + 1)) / 8)
      end
    else
      failed()
    end
  be done(index: USize, count: USize) =>
    if _failed then return end
    try
      _counts(index)? = count
      _pending = _pending - 1
      if _pending == 0 then
        for i in Range[USize](0, 9) do
          _env.out.print(_patterns(i)? + " " + _counts(i)?.string())
        end
        _env.out.print("")
        _env.out.print(_input_length.string())
        _env.out.print(_clean_length.string())
        _replace(0)
      end
    else
      failed()
    end
  be _replace(index: USize) =>
    if _failed then return end
    if index < _substitutions.size() then
      try
        let substitution = _substitutions(index)?
        let expression = Regex(substitution._1)?
        _sequence = try expression.replace(_sequence, substitution._2 where global = true)? else _sequence end
        _replace(index + 1)
      else
        failed()
      end
    else
      _env.out.print(_sequence.size().string())
      _env.out.flush()
    end
