use "gmp"

actor Main
  let _env: Env
  let _n: USize
  let _two: Mpz val = Mpz.from_i64(2)
  let _three: Mpz val = Mpz.from_i64(3)
  let _four: Mpz val = Mpz.from_i64(4)
  let _ten: Mpz val = Mpz.from_i64(10)
  var _q: Mpz val = Mpz.from_i64(1)
  var _r: Mpz val = Mpz.from_i64(0)
  var _t: Mpz val = Mpz.from_i64(1)
  var _k: U64 = 1
  var _printed: USize = 0
  var _line: String iso = recover String end
  new create(env: Env) =>
    _env = env
    _n = try env.args(1)?.usize()? else 27 end
    _step()
  be _step() =>
    var candidates: USize = 0
    while (candidates < 32) and (_printed < _n) do
      let numerator3 = (_q * _three) + _r
      let numerator4 = (_q * _four) + _r
      let digit3 = numerator3 / _t
      let digit4 = numerator4 / _t
      if digit3 == digit4 then
        _line.push(48 + digit3.u64().u8())
        _printed = _printed + 1
        if ((_printed % 10) == 0) or (_printed == _n) then
          var padding = (10 - (_printed % 10)) % 10
          while padding > 0 do
            _line.push(' ')
            padding = padding - 1
          end
          _line.append("\t:" + _printed.string() + "\n")
          let output: String iso = _line = recover String end
          _env.out.write(consume output)
        end
        _r = (_r - (digit3 * _t)) * _ten
        _q = _q * _ten
      else
        let factor = Mpz.from_i64(((2 * _k) + 1).i64())
        _r = ((_q * _two) + _r) * factor
        _q = _q * Mpz.from_i64(_k.i64())
        _t = _t * factor
        _k = _k + 1
      end
      candidates = candidates + 1
    end
    if _printed < _n then
      _step()
    else
      _env.out.flush()
    end
