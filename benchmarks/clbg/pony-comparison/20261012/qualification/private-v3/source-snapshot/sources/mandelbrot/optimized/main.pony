use "collections"

primitive Pixel
  fun apply(x: USize, y: USize, n: USize): U8 =>
    let cr = (x.f64() * (2.0 / n.f64())) - 1.5
    let ci = (y.f64() * (2.0 / n.f64())) - 1.0
    var zr: F64 = 0
    var zi: F64 = 0
    var tr: F64 = 0
    var ti: F64 = 0
    var iteration: USize = 0
    while iteration < 50 do
      zi = ((zr + zr) * zi) + ci
      zr = (tr - ti) + cr
      tr = zr * zr
      ti = zi * zi
      iteration = iteration + 1
      if (tr + ti) > 4.0 then
        return 0
      end
    end
    if (tr + ti) <= 4.0 then 1 else 0 end

actor Worker
  let _parent: Main
  let _index: USize
  let _n: USize
  let _last: USize
  var _row: USize
  var _bytes: String iso
  new create(parent: Main, index: USize, n: USize, first: USize, last: USize) =>
    _parent = parent
    _index = index
    _n = n
    _row = first
    _last = last
    _bytes = recover String((last - first) * ((n + 7) / 8)) end
    _step()
  be _step() =>
    if _row < _last then
      for column in Range[USize](0, (_n + 7) / 8) do
        var byte: U8 = 0
        for lane in Range[USize](0, USize(8).min(_n - (column * 8))) do
          byte = byte or (Pixel((column * 8) + lane, _row, _n) << (7 - lane).u8())
        end
        _bytes.push(byte)
      end
      _row = _row + 1
      _step()
    else
      let bytes: String iso = _bytes = recover String end
      _parent.done(_index, consume bytes)
    end

actor Main
  let _env: Env
  let _n: USize
  let _blocks: Array[String val]
  var _pending: USize
  new create(env: Env) =>
    _env = env
    _n = try env.args(1)?.usize()? else 200 end
    _pending = _n.min(8).max(1)
    _blocks = Array[String val].init("", _pending)
    if _n == 0 then
      env.exitcode(1)
      return
    end
    for index in Range[USize](0, _pending) do
      Worker(this, index, _n, (_n * index) / _pending, (_n * (index + 1)) / _pending)
    end
  be done(index: USize, bytes: String iso) =>
    try
      _blocks(index)? = consume bytes
      _pending = _pending - 1
      if _pending == 0 then
        _env.out.print("P4\n" + _n.string() + " " + _n.string())
        for block in _blocks.values() do
          _env.out.write(block)
        end
        _env.out.flush()
      end
    else
      _env.exitcode(1)
    end
