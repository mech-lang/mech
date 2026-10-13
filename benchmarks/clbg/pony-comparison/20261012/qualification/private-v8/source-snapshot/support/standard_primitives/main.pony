use "collections"
use "gmp"
use "regex"
use @pthread_self[USize]()

actor Producer
  let _parent: Main
  let _slot: USize
  new create(parent: Main, slot: USize) =>
    _parent = parent
    _slot = slot
    _produce()
  be _produce() =>
    let table = recover iso
      let entries = Map[U64, String](4096)
      for index in Range[USize](0, 4096) do
        entries(index.u64()) = index.string() + ":" + _slot.string()
      end
      entries
    end
    _parent.done(_slot, consume table, @pthread_self())

actor Main
  let _env: Env
  let _tables: Array[(Map[U64, String] val | None)] = Array[(Map[U64, String] val | None)].init(None, 7)
  let _threads: Map[USize, Bool] = Map[USize, Bool]
  let _original: Mpz val = Mpz.from_i64(1125899906842624)
  var _pending: USize = 7
  var _batch: USize = 0
  new create(env: Env) =>
    _env = env
    for slot in Range[USize](0, 7) do
      Producer(this, slot)
    end
  be done(slot: USize, table: Map[U64, String] iso, thread: USize) =>
    try
      if _tables(slot)? isnt None then error end
      _tables(slot)? = consume table
      _threads(thread) = true
      _pending = _pending - 1
      if _pending == 0 then
        if (_threads.size() < 1) or (_threads.size() > 8) then error end
        for index in Range[USize](0, 7) do
          let entries = _tables(index)? as Map[U64, String] val
          if entries.size() != 4096 then error end
          for key in Range[USize](0, 4096) do
            if entries(key.u64())? != (key.string() + ":" + index.string()) then error end
          end
          _tables(index)? = None
        end
        _contract()
      end
    else
      _env.exitcode(1)
    end
  be _contract() =>
    try
      let positive: Mpz val = Mpz.from_i64(17)
      let negative: Mpz val = Mpz.from_i64(-17)
      let five: Mpz val = Mpz.from_i64(5)
      if (positive / five).i64() != 3 then error end
      if (negative / five).i64() != -4 then error end
      if (positive * five).i64() != 85 then error end
      if positive.i64() != 17 then error end
      if ((_original * five) / five) != _original then error end
      let decimal = "1234567890123456789012345678901234567890"
      let parsed: Mpz val = Mpz.from_string(decimal)
      if parsed.string() != decimal then error end
      let expression = Regex("aa")?
      let iterator = expression.matches("aaaaa")
      var count: USize = 0
      while iterator.has_next() do
        iterator.next()?
        count = count + 1
      end
      if count != 2 then error end
      let substitute = Regex("aND|Ha[DS]")?
      let first: String val = substitute.replace("taNDHaD", "<3>" where global = true)?
      let tags = Regex("<[^>]*>")?
      let second = tags.replace(first, "|" where global = true)?
      if second != "t||" then error end
      _churn()
    else
      _env.exitcode(1)
    end
  be _churn() =>
    try
      let seven: Mpz val = Mpz.from_i64(7)
      for index in Range[USize](0, 128) do
        let value: Mpz val = Mpz.from_i64(index.i64())
        let updated = (_original * seven) + value
        if (updated - value) != (_original * seven) then error end
        if _original.i64() != 1125899906842624 then error end
      end
      _batch = _batch + 1
      if _batch < 16 then
        _churn()
      else
        _env.out.write("PONY_PRIMITIVES_OK\x00\n")
        _env.out.flush()
      end
    else
      _env.exitcode(1)
    end
