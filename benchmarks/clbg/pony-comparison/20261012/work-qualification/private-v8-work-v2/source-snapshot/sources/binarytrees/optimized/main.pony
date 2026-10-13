use "collections"

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

class Node
  let _trace_depth: USize
  var _trace_final: String = ""
  let left: (Node | None)
  let right: (Node | None)
  new create(depth: USize) =>
    _trace_depth = depth
    Trace("NODE\t" + depth.string())
    if depth > 0 then
      left = Node(depth - 1)
      right = Node(depth - 1)
    else
      left = None
      right = None
    end
    let identity = (digestof this).string()
    _trace_final = "FINAL\t" + depth.string() + "\t" + identity + "\n"
    match (left, right)
    | (let first: Node box, let second: Node box) =>
      Trace("GRAPH\t" + depth.string() + "\t" + identity + "\t" + (digestof first).string() + "\t" + (digestof second).string())
    else
      Trace("GRAPH\t" + depth.string() + "\t" + identity + "\t0\t0")
    end
  fun _final() =>
    let written = @write(2, _trace_final.cpointer(), _trace_final.size())
    if written != _trace_final.size().i64() then @exit(1) end
  fun check(): U64 =>
    Trace("CHECK\t" + _trace_depth.string() + "\t" + (digestof this).string())
    match (left, right)
    | (let first: Node box, let second: Node box) => 1 + first.check() + second.check()
    else
      1
    end

actor Worker
  let _parent: Main
  let _depth: USize
  var _remaining: USize
  var _sum: U64 = 0
  new create(parent: Main, depth: USize, count: USize) =>
    _parent = parent
    _depth = depth
    _remaining = count
    _one()
  be _one() =>
    Trace("THREAD\t" + @pthread_self().string())
    if _remaining == 0 then
      Trace("WORKER_DONE\t" + _depth.string() + "\t" + (digestof this).string())
      _parent.done(_sum, _depth, digestof this)
    else
      Trace("TREE\t" + _depth.string())
      let tree = Node(_depth)
      Trace("ROOT_CHURN_NEW\t" + _depth.string() + "\t" + (digestof tree).string() + "\t" + @pthread_self().string())
      _sum = _sum + tree.check()
      Trace("ROOT_CHURN_CHECK\t" + _depth.string() + "\t" + (digestof tree).string() + "\t" + @pthread_self().string())
      _remaining = _remaining - 1
      _one()
    end

actor Main
  let _env: Env
  let _maximum: USize
  let _long: Node
  var _depth: USize = 4
  var _iterations: USize = 0
  var _pending: USize = 0
  var _sum: U64 = 0
  new create(env: Env) =>
    _env = env
    _maximum = (try env.args(1)?.usize()? else 6 end).max(6)
    let stretch = Node(_maximum + 1)
    Trace("ROOT_STRETCH_NEW\t" + (_maximum + 1).string() + "\t" + (digestof stretch).string())
    env.out.print("stretch tree of depth " + (_maximum + 1).string() + "\t check: " + stretch.check().string())
    Trace("LONG_NEW\t" + _maximum.string())
    Trace("ROOT_STRETCH_CHECK\t" + (_maximum + 1).string() + "\t" + (digestof stretch).string())
    _long = Node(_maximum)
    Trace("ROOT_LONG_NEW\t" + _maximum.string() + "\t" + (digestof _long).string())
    _start()
  be _start() =>
    Trace("THREAD\t" + @pthread_self().string())
    _iterations = USize(1) << ((_maximum - _depth) + 4)
    _pending = _iterations.min(8)
    _sum = 0
    for index in Range[USize](0, _pending) do
      let count = ((_iterations * (index + 1)) / _pending) - ((_iterations * index) / _pending)
      Worker(this, _depth, count)
    end
  be done(value: U64, trace_depth: USize, trace_worker: USize) =>
    Trace("THREAD\t" + @pthread_self().string())
    Trace("WORKER_JOIN\t" + trace_depth.string() + "\t" + trace_worker.string())
    _sum = _sum + value
    _pending = _pending - 1
    if _pending == 0 then
      _env.out.print(_iterations.string() + "\t trees of depth " + _depth.string() + "\t check: " + _sum.string())
      _depth = _depth + 2
      if _depth <= _maximum then
        _start()
      else
        Trace("LONG_CHECK\t" + _maximum.string())
        _env.out.print("long lived tree of depth " + _maximum.string() + "\t check: " + _long.check().string())
        Trace("ROOT_LONG_CHECK\t" + _maximum.string() + "\t" + (digestof _long).string())
        _env.out.flush()
      end
    end
