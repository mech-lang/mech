use "collections"

primitive Complement
  fun apply(): Array[U8] val =>
    recover val
      let table = Array[U8].init(0, 256)
      let symbols = "ACBDGHKMNSRUTWVYacbdghkmnsrutwvy"
      let values = "TGVHCDMKNSYAAWBRTGVHCDMKNSYAAWBR"
      try
        for index in Range[USize](0, symbols.size()) do
          table(symbols(index)?.usize())? = values(index)?
        end
      end
      table
    end

actor Worker
  let _parent: Main
  let _sequence: String
  let _complement: Array[U8] val
  var _remaining: USize
  var _column: USize = 0
  var _output: String iso
  new create(parent: Main, sequence: String, complement: Array[U8] val) =>
    _parent = parent
    _sequence = sequence
    _complement = complement
    _remaining = sequence.size()
    _output = recover String(sequence.size() + (sequence.size() / 60) + 1) end
    _step()
  be _step() =>
    try
      var batch: USize = 0
      while (_remaining > 0) and (batch < 512) do
        _remaining = _remaining - 1
        _output.push(_complement(_sequence(_remaining)?.usize())?)
        _column = _column + 1
        if (_column == 60) or (_remaining == 0) then
          _output.push('\n')
          _column = 0
        end
        batch = batch + 1
      end
      if _remaining > 0 then
        _step()
      else
        let output: String iso = _output = recover String end
        _parent.done(consume output)
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
  let _complement: Array[U8] val = Complement()
  let _records: Array[(String, String)] = Array[(String, String)]
  var _input: String iso = recover String end
  var _index: USize = 0
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
    var header: String = ""
    var sequence = String
    for line in input.split("\r\n").values() do
      if line.at(">") then
        if header.size() > 0 then
          _records.push((header, sequence.clone()))
        end
        header = line.clone()
        sequence.clear()
      elseif not line.at(";") then
        sequence.append(line)
      end
    end
    if header.size() > 0 then
      _records.push((header, sequence.clone()))
    end
    _start()
  be _start() =>
    if _failed then return end
    if _index < _records.size() then
      try
        let record = _records(_index)?
        _env.out.print(record._1)
        Worker(this, record._2, _complement)
      else
        failed()
      end
    else
      _env.out.flush()
    end
  be done(output: String iso) =>
    if _failed then return end
    _env.out.write(consume output)
    _index = _index + 1
    _start()
