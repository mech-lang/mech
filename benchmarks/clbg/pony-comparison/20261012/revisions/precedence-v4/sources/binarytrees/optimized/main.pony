use "collections"

class Node
  let left: (Node | None)
  let right: (Node | None)
  new create(depth: USize) =>
    if depth > 0 then
      left = Node(depth - 1)
      right = Node(depth - 1)
    else
      left = None
      right = None
    end
  fun check(): U64 =>
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
    if _remaining == 0 then
      _parent.done(_sum)
    else
      let tree = Node(_depth)
      _sum = _sum + tree.check()
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
    env.out.print("stretch tree of depth " + (_maximum + 1).string() + "\t check: " + stretch.check().string())
    _long = Node(_maximum)
    _start()
  be _start() =>
    _iterations = USize(1) << ((_maximum - _depth) + 4)
    _pending = _iterations.min(8)
    _sum = 0
    for index in Range[USize](0, _pending) do
      let count = ((_iterations * (index + 1)) / _pending) - ((_iterations * index) / _pending)
      Worker(this, _depth, count)
    end
  be done(value: U64) =>
    _sum = _sum + value
    _pending = _pending - 1
    if _pending == 0 then
      _env.out.print(_iterations.string() + "\t trees of depth " + _depth.string() + "\t check: " + _sum.string())
      _depth = _depth + 2
      if _depth <= _maximum then
        _start()
      else
        _env.out.print("long lived tree of depth " + _maximum.string() + "\t check: " + _long.check().string())
        _env.out.flush()
      end
    end
