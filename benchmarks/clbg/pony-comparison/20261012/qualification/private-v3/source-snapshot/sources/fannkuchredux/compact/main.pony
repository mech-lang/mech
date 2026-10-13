use "collections"

actor Worker
  let _parent: Main
  let _n: USize
  let _last: USize
  let _permutation: Array[USize]
  let _rotations: Array[USize]
  var _rank: USize
  var _checksum: I64 = 0
  var _maximum: USize = 0
  new create(parent: Main, n: USize, first: USize, last: USize) =>
    _parent = parent
    _n = n
    _rank = first
    _last = last
    _permutation = Array[USize](n)
    _rotations = Array[USize].init(0, n)
    let factorial = Array[USize](n + 1)
    factorial.push(1)
    for index in Range[USize](0, n) do
      _permutation.push(index)
      try factorial.push(factorial(index)? * (index + 1)) end
    end
    try
      var width = n
      while width >= 2 do
        let shift = (first / factorial(width - 1)?) % width
        _rotations(width - 1)? = shift
        for turn in Range[USize](0, shift) do
          _rotate(width)?
        end
        width = width - 1
      end
    else
      _parent.failed()
      return
    end
    _step()
  fun ref _rotate(width: USize) ? =>
    let first = _permutation(0)?
    for index in Range[USize](0, width - 1) do
      _permutation(index)? = _permutation(index + 1)?
    end
    _permutation(width - 1)? = first
  be _step() =>
    try
      let stop = (_rank + 1024).min(_last)
      while _rank < stop do
        let work = _permutation.clone()
        var flips: USize = 0
        while work(0)? != 0 do
          var left: USize = 0
          var right = work(0)?
          while left < right do
            let value = work(left)?
            work(left)? = work(right)?
            work(right)? = value
            left = left + 1
            right = right - 1
          end
          flips = flips + 1
        end
        _checksum = _checksum + (if (_rank and 1) == 0 then flips.i64() else -flips.i64() end)
        _maximum = _maximum.max(flips)
        for width in Range[USize](2, _n + 1) do
          _rotate(width)?
          let turns = _rotations(width - 1)? + 1
          if turns < width then
            _rotations(width - 1)? = turns
            break
          end
          _rotations(width - 1)? = 0
        end
        _rank = _rank + 1
      end
      if _rank < _last then
        _step()
      else
        _parent.done(_checksum, _maximum)
      end
    else
      _parent.failed()
    end

actor Main
  let _env: Env
  let _n: USize
  var _pending: USize = 0
  var _checksum: I64 = 0
  var _maximum: USize = 0
  new create(env: Env) =>
    _env = env
    _n = try env.args(1)?.usize()? else 7 end
    if (_n < 1) or (_n > 12) then
      env.exitcode(1)
      return
    end
    var total: USize = 1
    for width in Range[USize](1, _n + 1) do
      total = total * width
    end
    _pending = 1
    for index in Range[USize](0, _pending) do
      Worker(this, _n, (total * index) / _pending, (total * (index + 1)) / _pending)
    end
  be failed() =>
    _env.exitcode(1)
  be done(checksum: I64, maximum: USize) =>
    _checksum = _checksum + checksum
    _maximum = _maximum.max(maximum)
    _pending = _pending - 1
    if _pending == 0 then
      _env.out.print(_checksum.string())
      _env.out.print("Pfannkuchen(" + _n.string() + ") = " + _maximum.string())
      _env.out.flush()
    end
