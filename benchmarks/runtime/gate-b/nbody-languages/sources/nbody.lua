-- The Computer Language Benchmarks Game
-- https://benchmarksgame-team.pages.debian.net/benchmarksgame/program/nbody-lua-2.html
-- contributed by Mike Pall; modified by Geoff Leyland
-- Adapted only to time the steady-state advance loop and emit CSV.

local sqrt = math.sqrt
local PI = 3.141592653589793
local SOLAR_MASS = 4 * PI * PI
local DAYS_PER_YEAR = 365.24
local bodies = {
  {
    x = 0, y = 0, z = 0,
    vx = 0, vy = 0, vz = 0,
    mass = SOLAR_MASS
  },
  {
    x = 4.84143144246472090e+00,
    y = -1.16032004402742839e+00,
    z = -1.03622044471123109e-01,
    vx = 1.66007664274403694e-03 * DAYS_PER_YEAR,
    vy = 7.69901118419740425e-03 * DAYS_PER_YEAR,
    vz = -6.90460016972063023e-05 * DAYS_PER_YEAR,
    mass = 9.54791938424326609e-04 * SOLAR_MASS
  },
  {
    x = 8.34336671824457987e+00,
    y = 4.12479856412430479e+00,
    z = -4.03523417114321381e-01,
    vx = -2.76742510726862411e-03 * DAYS_PER_YEAR,
    vy = 4.99852801234917238e-03 * DAYS_PER_YEAR,
    vz = 2.30417297573763929e-05 * DAYS_PER_YEAR,
    mass = 2.85885980666130812e-04 * SOLAR_MASS
  },
  {
    x = 1.28943695621391310e+01,
    y = -1.51111514016986312e+01,
    z = -2.23307578892655734e-01,
    vx = 2.96460137564761618e-03 * DAYS_PER_YEAR,
    vy = 2.37847173959480950e-03 * DAYS_PER_YEAR,
    vz = -2.96589568540237556e-05 * DAYS_PER_YEAR,
    mass = 4.36624404335156298e-05 * SOLAR_MASS
  },
  {
    x = 1.53796971148509165e+01,
    y = -2.59193146099879641e+01,
    z = 1.79258772950371181e-01,
    vx = 2.68067772490389322e-03 * DAYS_PER_YEAR,
    vy = 1.62824170038242295e-03 * DAYS_PER_YEAR,
    vz = -9.51592254519715870e-05 * DAYS_PER_YEAR,
    mass = 5.15138902046611451e-05 * SOLAR_MASS
  }
}

local function advance(system, nbody, dt)
  for i = 1, nbody do
    local bi = system[i]
    local bix, biy, biz, bimass = bi.x, bi.y, bi.z, bi.mass
    local bivx, bivy, bivz = bi.vx, bi.vy, bi.vz
    for j = i + 1, nbody do
      local bj = system[j]
      local dx, dy, dz = bix - bj.x, biy - bj.y, biz - bj.z
      local mag = sqrt(dx * dx + dy * dy + dz * dz)
      mag = dt / (mag * mag * mag)
      local bm = bj.mass * mag
      bivx = bivx - dx * bm
      bivy = bivy - dy * bm
      bivz = bivz - dz * bm
      bm = bimass * mag
      bj.vx = bj.vx + dx * bm
      bj.vy = bj.vy + dy * bm
      bj.vz = bj.vz + dz * bm
    end
    bi.vx, bi.vy, bi.vz = bivx, bivy, bivz
    bi.x = bix + dt * bivx
    bi.y = biy + dt * bivy
    bi.z = biz + dt * bivz
  end
end

local function energy(system, nbody)
  local e = 0
  for i = 1, nbody do
    local bi = system[i]
    local vx, vy, vz, bim = bi.vx, bi.vy, bi.vz, bi.mass
    e = e + 0.5 * bim * (vx * vx + vy * vy + vz * vz)
    for j = i + 1, nbody do
      local bj = system[j]
      local dx, dy, dz = bi.x - bj.x, bi.y - bj.y, bi.z - bj.z
      local distance = sqrt(dx * dx + dy * dy + dz * dz)
      e = e - bim * bj.mass / distance
    end
  end
  return e
end

local function offset_momentum(system, nbody)
  local px, py, pz = 0, 0, 0
  for i = 1, nbody do
    local bi = system[i]
    local bim = bi.mass
    px = px + bi.vx * bim
    py = py + bi.vy * bim
    pz = pz + bi.vz * bim
  end
  system[1].vx = -px / SOLAR_MASS
  system[1].vy = -py / SOLAR_MASS
  system[1].vz = -pz / SOLAR_MASS
end

local turns = tonumber(arg and arg[1]) or 1000000
local implementation = (arg and arg[2]) or "lua-game-2"
local nbody = #bodies
offset_momentum(bodies, nbody)
local initial_energy = energy(bodies, nbody)
local warmup = {}
for i = 1, nbody do
  local body = bodies[i]
  warmup[i] = {
    x = body.x, y = body.y, z = body.z,
    vx = body.vx, vy = body.vy, vz = body.vz,
    mass = body.mass
  }
end
for _ = 1, 1000 do
  advance(warmup, nbody, 0.01)
end
collectgarbage("collect")
local memory_before = collectgarbage("count") * 1024
local started = os.clock()
for _ = 1, turns do
  advance(bodies, nbody, 0.01)
end
local seconds = os.clock() - started
local memory_after = collectgarbage("count") * 1024
local final_energy = energy(bodies, nbody)
io.write(string.format(
  "%s,%d,%.9f,%.3f,%.3f,%.12f,%.12f,0,%.0f\n",
  implementation, turns, seconds, seconds * 1e9 / turns,
  turns / seconds, initial_energy, final_energy, memory_after - memory_before
))
