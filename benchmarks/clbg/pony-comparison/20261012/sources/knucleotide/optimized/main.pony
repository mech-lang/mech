use "collections"
use "format"

primitive DNA
  fun apply(input: String): String ? =>
    recover val
      let sequence = String
      var selected = false
      for line in input.split("\r\n").values() do
        if line.at(";") then
          continue
        elseif line.at(">") then
          if selected then break end
          selected = (line == ">THREE") or line.at(">THREE ") or line.at(">THREE\t")
        elseif selected then
          sequence.append(line.upper())
        end
      end
      for value in sequence.values() do
        if (value != 'A') and (value != 'C') and (value != 'G') and (value != 'T') then
          error
        end
      end
      sequence
    end

class val Entry is Comparable[Entry]
  let word: String
  let count: U64
  new val create(word': String, count': U64) =>
    word = word'
    count = count'
  fun eq(that: Entry box): Bool => (word == that.word) and (count == that.count)
  fun ne(that: Entry box): Bool => not eq(that)
  fun lt(that: Entry box): Bool =>
    if count == that.count then word < that.word else count > that.count end
  fun le(that: Entry box): Bool => lt(that) or eq(that)
  fun gt(that: Entry box): Bool => that.lt(this)
  fun ge(that: Entry box): Bool => gt(that) or eq(that)

primitive Lengths
  fun apply(): Array[USize] val =>
    recover val
      let values = Array[USize](7)
      values.push(1)
      values.push(2)
      values.push(3)
      values.push(4)
      values.push(6)
      values.push(12)
      values.push(18)
      values
    end

primitive Queries
  fun apply(): Array[String] val =>
    recover val
      let values = Array[String](5)
      values.push("GGT")
      values.push("GGTA")
      values.push("GGTATT")
      values.push("GGTATTTTAATT")
      values.push("GGTATTTTAATTTATAGT")
      values
    end

primitive Packed
  fun code(value: U8): U64 =>
    match value
    | 'A' => 0
    | 'C' => 1
    | 'G' => 2
    | 'T' => 3
    else
      0
    end
  fun encode(word: String): U64 =>
    var result: U64 = 0
    for value in word.values() do
      result = (result << 2) or code(value)
    end
    result
  fun decode(key: U64, width: USize): String =>
    recover val
      let word = String(width)
      try
        for index in Range[USize](0, width) do
          let shift = 2 * (width - index - 1)
          word.push("ACGT"(((key >> shift) and 3).usize())?)
        end
      end
      word
    end

actor Worker
  let _parent: Main
  let _slot: USize
  let _sequence: String
  let _width: USize
  var _index: USize = 0
  var _key: U64 = 0
  var _counts: Map[U64, U64] iso = recover Map[U64, U64] end
  new create(parent: Main, slot: USize, sequence: String, width: USize) =>
    _parent = parent
    _slot = slot
    _sequence = sequence
    _width = width
    _step()
  be _step() =>
    let stop = (_index + 65536).min(_sequence.size())
    try
      let mask = (U64(1) << (2 * _width)) - 1
      while _index < stop do
        _key = ((_key << 2) or Packed.code(_sequence(_index)?)) and mask
        if (_index + 1) >= _width then
          _counts(_key) = _counts.get_or_else(_key, 0) + 1
        end
        _index = _index + 1
      end
    else
      _parent.failed()
      return
    end
    if _index < _sequence.size() then
      _step()
    else
      let counts: Map[U64, U64] iso = _counts = recover Map[U64, U64] end
      _parent.done(_slot, consume counts)
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
  let _lengths: Array[USize] val = Lengths()
  let _queries: Array[String] val = Queries()
  let _maps: Array[Map[U64, U64] val] = Array[Map[U64, U64] val].init(recover val Map[U64, U64] end, 7)
  var _input: String iso = recover String end
  var _sequence: String = ""
  var _completed: USize = 0
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
    try
      _sequence = DNA(input)?
      for slot in Range[USize](0, 7) do
        Worker(this, slot, _sequence, _lengths(slot)?)
      end
    else
      failed()
    end
  be done(slot: USize, counts: Map[U64, U64] iso) =>
    if _failed then return end
    try
      _maps(slot)? = consume counts
      _completed = _completed + 1
      if _completed == 7 then
        _print()?
      end
    else
      failed()
    end
  fun ref _print() ? =>
    for index in Range[USize](0, 2) do
      let entries = Array[Entry]
      var total: U64 = 0
      for (key, count) in _maps(index)?.pairs() do
        entries.push(Entry(Packed.decode(key, _lengths(index)?), count))
        total = total + count
      end
      Sort[Array[Entry], Entry](entries)
      for entry in entries.values() do
        let percentage = 100.0 * entry.count.f64() / total.f64()
        _env.out.print(entry.word + " " + Format.float[F64](percentage where fmt = FormatFix, prec = 3))
      end
      _env.out.print("")
    end
    for index in Range[USize](0, 5) do
      let word = _queries(index)?
      _env.out.print(_maps(index + 2)?.get_or_else(Packed.encode(word), 0).string() + "\t" + word)
    end
    _env.out.flush()
