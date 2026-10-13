use "collections"
use "format"

actor Worker
  let _parent: Main
  let _first: USize
  let _last: USize
  let _input: Array[F64] val
  let _transpose: Bool
  var _row: USize
  var _output: Array[F64] iso
  new create(parent: Main, input: Array[F64] val, first: USize, last: USize, transpose: Bool) =>
    _parent = parent
    _input = input
    _first = first
    _last = last
    _row = first
    _transpose = transpose
    _output = recover Array[F64](last - first) end
    _step()
  be _step() =>
    if _row < _last then
      var value: F64 = 0
      try
        for column in Range[USize](0, _input.size()) do
          let i = if _transpose then column else _row end
          let j = if _transpose then _row else column end
          let denominator = (((i + j) * (i + j + 1)) / 2) + i + 1
          value = value + (_input(column)? / denominator.f64())
        end
        _output.push(value)
        _row = _row + 1
        _step()
      else
        _parent.failed()
      end
    else
      let output: Array[F64] iso = _output = recover Array[F64] end
      _parent.done(_first, consume output)
    end

actor Main
  let _env: Env
  let _n: USize
  var _u: Array[F64] val
  var _v: Array[F64] val
  var _temporary: Array[F64] val
  var _output: Array[F64] iso
  var _phase: USize = 0
  var _round: USize = 0
  var _pending: USize = 0
  new create(env: Env) =>
    _env = env
    _n = (try env.args(1)?.usize()? else 100 end).max(1)
    _u = recover val Array[F64].init(1, _n) end
    _v = recover val Array[F64].init(0, _n) end
    _temporary = recover val Array[F64].init(0, _n) end
    _output = recover Array[F64] end
    _start()
  be failed() =>
    _env.exitcode(1)
  be _start() =>
    _output = recover Array[F64].init(0, _n) end
    _pending = _n.min(8)
    let input = if (_phase == 0) then _u elseif (_phase == 2) then _v else _temporary end
    let transpose = (_phase and 1) == 1
    for worker in Range[USize](0, _pending) do
      Worker(this, input, (_n * worker) / _pending, (_n * (worker + 1)) / _pending, transpose)
    end
  be done(first: USize, output: Array[F64] iso) =>
    try
      for index in Range[USize](0, output.size()) do
        _output(first + index)? = output(index)?
      end
      _pending = _pending - 1
      if _pending == 0 then
        let result: Array[F64] val = _output = recover Array[F64] end
        match _phase
        | 0 => _temporary = result
        | 1 => _v = result
        | 2 => _temporary = result
        | 3 => _u = result
        end
        _phase = (_phase + 1) % 4
        if _phase == 0 then
          _round = _round + 1
        end
        if _round < 10 then
          _start()
        else
          var uv: F64 = 0
          var vv: F64 = 0
          for index in Range[USize](0, _n) do
            uv = uv + (_u(index)? * _v(index)?)
            vv = vv + (_v(index)? * _v(index)?)
          end
          _env.out.print(Format.float[F64]((uv / vv).sqrt() where fmt = FormatFix, prec = 9))
          _env.out.flush()
        end
      end
    else
      failed()
    end
