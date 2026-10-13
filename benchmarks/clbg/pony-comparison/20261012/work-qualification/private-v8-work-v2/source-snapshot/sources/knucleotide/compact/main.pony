use "collections"
use "format"

use @write[I64](fd: I32, buffer: Pointer[U8] tag, count: USize)
use @pthread_self[USize]()
use @exit[None](status: I32)

primitive Trace
  fun apply(message: String) =>
    let line = message + "\n"
    var offset: USize = 0
    while offset < line.size() do
      let written = @write(2, line.cpointer(offset), line.size() - offset)
      if written <= 0 then @exit(1) end
      offset = offset + written.usize()
    end

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

actor Worker
  let _parent: Main
  let _slot: USize
  let _sequence: String
  let _width: USize
  let _last: USize
  var _index: USize = 0
  var _counts: Map[String, U64] iso = recover Map[String, U64] end
  new create(parent: Main, slot: USize, sequence: String, width: USize) =>
    _parent = parent
    _slot = slot
    _sequence = sequence
    _width = width
    _last = if sequence.size() >= width then (sequence.size() - width) + 1 else 0 end
    _step()
  be _step() =>
    Trace("THREAD\t" + @pthread_self().string())
    let stop = (_index + 65536).min(_last)
    while _index < stop do
      let word: String = _sequence.substring(_index.isize(), (_index + _width).isize())
      _counts(word) = _counts.get_or_else(word, 0) + 1
      Trace("WINDOW\t" + _width.string() + "\t" + _index.string() + "\t" + word)
      _index = _index + 1
    end
    if _index < _last then
      _step()
    else
      let counts: Map[String, U64] iso = _counts = recover Map[String, U64] end
      Trace("MAP_DONE\t" + _width.string() + "\t" + counts.size().string())
      for (word, count) in counts.pairs() do
        Trace("MAP\t" + _width.string() + "\t" + word + "\t" + count.string())
      end
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
  let _maps: Array[Map[String, U64] val] = Array[Map[String, U64] val].init(recover val Map[String, U64] end, 7)
  var _input: String iso = recover String end
  var _sequence: String = ""
  var _completed: USize = 0
  var _failed: Bool = false
  new create(env: Env) =>
    _env = env
    env.input(recover Input(this) end, 65536)
  be data(bytes: Array[U8] iso) =>
    Trace("THREAD\t" + @pthread_self().string())
    _input.append(consume bytes)
  be failed() =>
    Trace("THREAD\t" + @pthread_self().string())
    _failed = true
    _env.exitcode(1)
  be finish() =>
    Trace("THREAD\t" + @pthread_self().string())
    if _failed then return end
    let input: String = _input = recover String end
    try
      _sequence = DNA(input)?
      Worker(this, 0, _sequence, _lengths(0)?)
    else
      failed()
    end
  be done(slot: USize, counts: Map[String, U64] iso) =>
    Trace("THREAD\t" + @pthread_self().string())
    if _failed then return end
    try
      _maps(slot)? = consume counts
      Trace("MAP_PARENT\t" + _lengths(slot)?.string() + "\t" + _maps(slot)?.size().string())
      _completed = _completed + 1
      if _completed == 7 then
        _print()?
      else
        Worker(this, _completed, _sequence, _lengths(_completed)?)
      end
    else
      failed()
    end
  fun ref _print() ? =>
    for trace_slot in Range[USize](0, 7) do
      for (trace_word, trace_count) in _maps(trace_slot)?.pairs() do
        Trace("PARENT_MAP\t" + _lengths(trace_slot)?.string() + "\t" + trace_word + "\t" + trace_count.string())
      end
    end
    for index in Range[USize](0, 2) do
      let entries = Array[Entry]
      var total: U64 = 0
      for (word, count) in _maps(index)?.pairs() do
        entries.push(Entry(word, count))
        total = total + count
      end
      Sort[Array[Entry], Entry](entries)
      for entry in entries.values() do
        let percentage = (100.0 * entry.count.f64()) / total.f64()
        _env.out.print(entry.word + " " + Format.float[F64](percentage where fmt = FormatFix, prec = 3))
      end
      _env.out.print("")
    end
    for index in Range[USize](0, 5) do
      let word = _queries(index)?
      _env.out.print(_maps(index + 2)?.get_or_else(word, 0).string() + "\t" + word)
    end
    _env.out.flush()
