use "collections"

primitive Probabilities
  fun iub(): Array[(U8, F64)] val =>
    recover val
      let entries = Array[(U8, F64)](15)
      entries.push(('a', 0.27))
      entries.push(('c', 0.12))
      entries.push(('g', 0.12))
      entries.push(('t', 0.27))
      entries.push(('B', 0.02))
      entries.push(('D', 0.02))
      entries.push(('H', 0.02))
      entries.push(('K', 0.02))
      entries.push(('M', 0.02))
      entries.push(('N', 0.02))
      entries.push(('R', 0.02))
      entries.push(('S', 0.02))
      entries.push(('V', 0.02))
      entries.push(('W', 0.02))
      entries.push(('Y', 0.02))
      cumulative(entries)
      entries
    end
  fun homo(): Array[(U8, F64)] val =>
    recover val
      let entries = Array[(U8, F64)](4)
      entries.push(('a', 0.3029549426680))
      entries.push(('c', 0.1979883004921))
      entries.push(('g', 0.1975473066391))
      entries.push(('t', 0.3015094502008))
      cumulative(entries)
      entries
    end
  fun cumulative(entries: Array[(U8, F64)]) =>
    var total: F64 = 0
    try
      for index in Range[USize](0, entries.size()) do
        let entry = entries(index)?
        total = total + entry._2
        entries(index)? = (entry._1, total)
      end
    end

actor Main
  let _env: Env
  let _n: USize
  let _alu: String = "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCAGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCCAGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA"
  let _iub: Array[(U8, F64)] val = Probabilities.iub()
  let _homo: Array[(U8, F64)] val = Probabilities.homo()
  var _seed: U64 = 42
  var _phase: USize = 1
  var _length: USize
  var _index: USize = 0
  var _column: USize = 0
  new create(env: Env) =>
    _env = env
    _n = try env.args(1)?.usize()? else 1000 end
    _length = 2 * _n
    env.out.print(">ONE Homo sapiens alu")
    _step()
  fun ref _random(): U8 =>
    _seed = ((_seed * 3877) + 29573) % 139968
    let draw = _seed.f64() / 139968.0
    let cdf = if _phase == 2 then _iub else _homo end
    var symbol: U8 = 0
    for entry in cdf.values() do
      symbol = entry._1
      if draw < entry._2 then
        return symbol
      end
    end
    symbol
  be _step() =>
    let stop = (_index + 65536).min(_length)
    let buffer: String iso = recover String((stop - _index) + 1024) end
    try
      while _index < stop do
        let value = if _phase == 1 then _alu(_index % _alu.size())? else _random() end
        buffer.push(value)
        _index = _index + 1
        _column = _column + 1
        if (_column == 60) or (_index == _length) then
          buffer.push('\n')
          _column = 0
        end
      end
      _env.out.write(consume buffer)
      if _index < _length then
        _step()
      else
        _phase = _phase + 1
        _index = 0
        if _phase == 2 then
          _length = 3 * _n
          _env.out.print(">TWO IUB ambiguity codes")
          _step()
        elseif _phase == 3 then
          _length = 5 * _n
          _env.out.print(">THREE Homo sapiens frequency")
          _step()
        else
          _env.out.flush()
        end
      end
    else
      _env.exitcode(1)
    end
