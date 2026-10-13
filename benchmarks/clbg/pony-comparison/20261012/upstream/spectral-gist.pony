use col = "collections"
use fmt = "format"

type Error is String
type Result is (F64 | Error)
type MaybeArray is (Array[F64] iso | Error val)

interface ReceiveUV 
  be receive(m: MaybeArray, a: Array[F64] iso)

interface ReceiveA
  be receive(index: USize, a: F64)

interface ResultShower
  be show_result(result: Result)

actor Main is ResultShower
  let _env: Env
  
  new create(env: Env) =>
    _env = env
    let n_str: String = try _env.args(1)? else "100" end
    let n: (USize | None) = try n_str.usize()? else None end
    match n
    | let n': USize => SpectralNorm(this, n').approximate()
    | None => show_result("Invalid argument provided: " + n_str)
    end

  be show_result(result: Result) =>
    match result
    | let err: Error => _env.out.print("ERROR: " + err)
    | let n: F64 => _env.out.print(fmt.Format.float[F64](n where prec = 10))
    end

actor SpectralNorm

  let _shower: ResultShower tag
  let _n: USize

  new create(result_shower: ResultShower tag, n: USize) =>
    _shower = result_shower
    _n = n

  be approximate() =>
    let u: Array[F64] iso = recover Array[F64].init(1.0, _n) end
    let v: Array[F64] iso = recover Array[F64].init(0.0, _n) end
    let r: ReceiveUV tag = this

    // 20 steps of the power method, then receive is called
    Repeater(20, r).go(consume u, consume v)
  
  be receive(u: MaybeArray, v: Array[F64] iso) =>
    match consume u
    | let u': Array[F64] iso => _eigenv(consume v, consume u')
    | let err: Error => _shower.show_result(err)
    end


  fun _eigenv(u: Array[F64] iso, v: Array[F64] iso) =>
    // B=AtA         A multiplied by A transposed
    // v.Bv /(v.v)   eigenvalue of v 
    var vBv: F64 = 0.0
    var vv: F64 = 0.0
    try
      for i in col.Range(0, _n) do
        vBv = vBv +~ (u(i)? *~ v(i)?)
        vv  = vv +~ (v(i)? *~ v(i)?)
      end
      _shower.show_result((vBv /~ vv).sqrt())
    else
      _shower.show_result("Some error occurred in second loop")
    end

"Repeater will repeat calls to MultiplyAv(u, v) then MultiplyAtv(v, u) n times."
actor Repeater
  var _count: USize
  let _onDone: ReceiveUV tag
  var _temp: Array[F64] iso
  var _toggle: Bool

  new create(repeats: USize, onDone: ReceiveUV tag) =>
    _count = repeats
    _onDone = onDone
    _temp = recover Array[F64](0) end
    _toggle = true

  be go(v: Array[F64] iso, atav: Array[F64] iso) =>
    _temp = consume atav
    MultiplyAv(consume v, this).go()

  be receive(u: MaybeArray, v: Array[F64] iso) =>
    if _toggle = not _toggle then
      _receiveMultiplyAv(consume u, consume v)
    else
      _receiveMultiplyAtv(consume u, consume v)
    end

  be _receiveMultiplyAv(u: MaybeArray, v: Array[F64] iso) =>
    // _temp was atav, now becomes v, as we will pass u and atav to MultiplyAtv 
    let atav: Array[F64] iso = _temp = consume v
    match consume u
    | let err: Error => _onDone.receive(err, consume atav)
    | let u': Array[F64] iso => MultiplyAtv(consume u', consume atav, this).go()
    end

  be _receiveMultiplyAtv(u: MaybeArray, atav: Array[F64] iso) =>
    // _temp was v, now becomes atav again so we can repeat the cycle
    let v: Array[F64] iso = _temp = consume atav
    match consume u
    | let err: Error => _onDone.receive(err, consume v)
    | let _: Array[F64] iso => _next(consume v)
    end
    
  fun ref _next(v: Array[F64] iso) =>
  "Decrement the count, and if necessary run cycle again, this time with the parameters v and atav inverted"
    _count = _count - 1
    let atav: Array[F64] iso = _temp = recover Array[F64](0) end
    if _count == 0 then
      // when _count reaches 0, we assume the parameters are in the initial order again (initial count must be even)
      _onDone.receive(consume v, consume atav)
    else
      go(consume atav, consume v)
    end

actor MultiplyAv
  var _u: Array[F64] iso
  let _v: Array[F64] val
  let _onDone: ReceiveUV tag
  let _n: USize

  new create(v: Array[F64] iso, onDone: ReceiveUV tag) =>
    _n = v.size()
    _v = consume v
    _u = recover Array[F64].init(0.0, _n) end
    _onDone = onDone

  be go() =>
    for i in col.Range(0, _n) do
      M(this).m1(_n, _v, i)
    end
  
  be receive(i: USize, a: F64) =>
    if _u.size() == 0 then return end // indicates previous error or already done
    try _u(i)? = a else
      let empty: Array[F64] iso = recover Array[F64](0) end
      _u = consume empty
      _onDone.receive("ERROR updating array", recover Array[F64](0) end)
      return
    end
    if _isLastIndex(i) then
      let empty1: Array[F64] iso = recover Array[F64](0) end
      let empty2: Array[F64] val = recover Array[F64](0) end
      let u: Array[F64] iso = _u = consume empty1
      let v: Array[F64] iso = recover iso
        let v = Array[F64](_v.size())
        _v.copy_to(v, 0, 0, _v.size())
        v
      end
      _onDone.receive(consume u, consume v)
    end
  
  fun _isLastIndex(i: USize): Bool =>
    (i + 1) == _n    

actor MultiplyAtv
  let _v: Array[F64] val
  var _atv: Array[F64] iso
  let _onDone: ReceiveUV tag
  let _n: USize

  new create(v: Array[F64] iso, atv: Array[F64] iso, onDone: ReceiveUV tag) =>
    _v = consume v
    _atv = consume atv
    _onDone = onDone
    _n = _v.size()

  be go() =>
    for i in col.Range(0, _n) do
      M(this).m2(_n, _v, i)
    end
  
  be receive(i: USize, a: F64) =>
    if _atv.size() == 0 then return end // indicates previous error or already done
    try _atv(i)? = a else
      let empty: Array[F64] iso = recover Array[F64](0) end
      _atv = consume empty
      _onDone.receive("ERROR updating array", recover Array[F64](0) end)
      return
    end
    if _isLastIndex(i) then
      let v: Array[F64] iso = recover iso
        let v = Array[F64](_v.size())
        _v.copy_to(v, 0, 0, _v.size())
        v
      end
      let empty: Array[F64] iso = recover Array[F64](0) end
      let atv: Array[F64] iso = _atv = consume empty
      _onDone.receive(consume v, consume atv)
    end
  
  fun _isLastIndex(i: USize): Bool =>
    (i + 1) == _n

actor M
  let _receiver: ReceiveA tag

  new create(r: ReceiveA tag) =>
    _receiver = r

  // inner loop of MultiplyAv
  be m1(n: USize, v: Array[F64] val, i: USize) =>
    var a: F64 = 0.0
    for j in col.Range(0, n) do
      try a = a +~ (_a(i,j)*~v(j)?) end
    end
    _receiver.receive(i, a)

  // inner loop of MultiplyAtv
  be m2(n: USize, v: Array[F64] val, i: USize) =>
    var a: F64 = 0.0
    for j in col.Range(0, n) do
      try a = a +~ (_a(j,i)*~v(j)?) end
    end
    _receiver.receive(i, a)

  /* return element i,j of infinite matrix A */
  fun _a(i: USize, j: USize): F64 =>
    F64(1.0) /~ ( (((i+~j).f64()*~(i+~j+~1).f64()) /~ 2.0 ) +~ (i+~1).f64() )

