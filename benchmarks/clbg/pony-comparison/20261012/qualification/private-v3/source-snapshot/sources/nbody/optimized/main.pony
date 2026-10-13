use "format"

class Body
  var x: F64
  var y: F64
  var z: F64

  var vx: F64
  var vy: F64
  var vz: F64

  var mass: F64

  let solar_mass: F64 = F64.pi() * F64.pi() * 4
  let days_per_year: F64 = 365.24

  new sun() =>
    x = 0
    y = 0
    z = 0
    vx = 0
    vy = 0
    vz = 0
    mass = 1
    _init()

  new jupiter() =>
    x = 4.8414314424647209
    y = -F64(1.16032004402742839)
    z = -F64(1.03622044471123109e-1)

    vx = 1.66007664274403694e-3
    vy = 7.69901118419740425e-3
    vz = -F64(6.90460016972063023e-5)

    mass = 9.54791938424326609e-4
    _init()

  new saturn() =>
    x = 8.34336671824457987
    y = 4.12479856412430479
    z = -F64(4.03523417114321381e-1)

    vx = -F64(2.76742510726862411e-3)
    vy = 4.99852801234917238e-3
    vz = 2.30417297573763929e-5

    mass = 2.85885980666130812e-4
    _init()

  new uranus() =>
    x = 1.28943695621391310e1
    y = -F64(1.51111514016986312e1)
    z = -F64(2.23307578892655734e-1)

    vx = 2.96460137564761618e-3
    vy = 2.37847173959480950e-3
    vz = -F64(2.96589568540237556e-5)

    mass = 4.36624404335156298e-5
    _init()

  new neptune() =>
    x = 1.53796971148509165e1
    y = -F64(2.59193146099879641e1)
    z = 1.79258772950371181e-1

    vx = 2.68067772490389322e-3
    vy = 1.62824170038242295e-3
    vz = -F64(9.51592254519715870e-5)

    mass = 5.15138902046611451e-5
    _init()

  fun ref attract(that: Body, dt: F64) =>
    let dx = x - that.x
    let dy = y - that.y
    let dz = z - that.z
    let d2 = (dx * dx) + (dy * dy) + (dz * dz)
    let mag = dt / (d2 * d2.sqrt())

    vx = vx - ((dx * that.mass) * mag)
    vy = vy - ((dy * that.mass) * mag)
    vz = vz - ((dz * that.mass) * mag)

    that.vx = that.vx + ((dx * mass) * mag)
    that.vy = that.vy + ((dy * mass) * mag)
    that.vz = that.vz + ((dz * mass) * mag)

  fun ref integrate(dt: F64) =>
    x = x + (vx * dt)
    y = y + (vy * dt)
    z = z + (vz * dt)

  fun ke(): F64 => 0.5 * mass * ((vx * vx) + (vy * vy) + (vz * vz))

  fun pe(that: Body box): F64 =>
    let dx = x - that.x
    let dy = y - that.y
    let dz = z - that.z
    let d = ((dx * dx) + (dy * dy) + (dz * dz)).sqrt()
    (mass * that.mass) / d

  fun ref _init() =>
    vx = vx * days_per_year
    vy = vy * days_per_year
    vz = vz * days_per_year
    mass = mass * solar_mass

actor Main
  let system: Array[Body] = Array[Body](5)
  let sun: Body = Body.sun()
  let _env: Env
  let _total: USize
  var _index: USize = 0

  new create(env: Env) =>
    _env = env

    _total = try
      env.args(1)?.usize()?
    else
      1000
    end

    system
      .>push(sun)
      .>push(Body.jupiter())
      .>push(Body.saturn())
      .>push(Body.uranus())
      .>push(Body.neptune())

    offset_momentum()
    print_energy()

    _step()

  be _step() =>
    let stop = (_index + 10000).min(_total)
    while _index < stop do
      advance(0.01)
      _index = _index + 1
    end
    if _index < _total then
      _step()
    else
      print_energy()
      _env.out.flush()
    end

  fun ref advance(dt: F64) =>
    let count = system.size()
    var i: USize = 0

    while i < count do
      try
        let body = system(i)?
        var j = i + 1

        while j < count do
          body.attract(system(j)?, dt)
          j = j + 1
        end
      end

      i = i + 1
    end

    try
      i = 0
      while i < system.size() do
        let body = system(i)?
        body.integrate(dt)
        i = i + 1
      end
    end

  fun ref print_energy() =>
    _env.out.print(Format.float[F64](energy() where fmt = FormatFix, prec = 9))

  fun ref energy(): F64 =>
    let count = system.size()
    var e: F64 = 0
    var i: USize = 0
    while i < count do
      try
        let body = system(i)?
        e = e + body.ke()
        var j = i + 1
        while j < count do
          e = e - body.pe(system(j)?)
          j = j + 1
        end
      end
      i = i + 1
    end
    e

  fun ref offset_momentum() =>
    var px: F64 = 0
    var py: F64 = 0
    var pz: F64 = 0

    try
      var i: USize = 0
      while i < system.size() do
        var body = system(i)?
        px = px - (body.vx * body.mass)
        py = py - (body.vy * body.mass)
        pz = pz - (body.vz * body.mass)
        i = i + 1
      end
    end

    sun.vx = px / sun.mass
    sun.vy = py / sun.mass
    sun.vz = pz / sun.mass
