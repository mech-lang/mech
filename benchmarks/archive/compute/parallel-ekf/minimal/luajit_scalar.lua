local ffi = require("ffi")
local sin = math.sin
local cos = math.cos
local atan2 = math.atan2
local pi = math.pi
local instances = math.max(1, tonumber(arg[1]) or 10000)
local turns = math.max(1, tonumber(arg[2]) or 5)
local checked = string.lower(arg[3] or "unchecked") == "checked"
local finite_limit = 3.402823466e38
local symmetry_tolerance = 1e-4
local velocity = ffi.new("float[?]", instances)
local omega = ffi.new("float[?]", instances)
local bearing = ffi.new("float[?]", instances)
local state = ffi.new("float[?]", instances * 3)
local covariance = ffi.new("float[?]", instances * 9)

local function reset()
  for i = 0, instances - 1 do
    local x = i * 3
    state[x] = 55
    state[x + 1] = 25
    state[x + 2] = 0.4
    local q = i * 9
    for j = 0, 8 do
      covariance[q + j] = 0
    end
    covariance[q] = 100
    covariance[q + 4] = 100
    covariance[q + 8] = 0.15
  end
end

for i = 0, instances - 1 do
  local phase = 2 * pi * i / instances
  velocity[i] = 1 + 0.05 * sin(3 * phase)
  omega[i] = 0.015 * (1 + 0.1 * sin(2 * phase))
  bearing[i] = -0.55 + 0.01 * sin(7 * phase) + 0.005 * sin(11 * phase)
end

reset()

local function arr(n)
  return ffi.new("float[?]", n)
end

local s = {
  f = arr(9),
  ft = arr(9),
  g = arr(6),
  gt = arr(6),
  left = arr(9),
  pp = arr(9),
  ql = arr(6),
  qp = arr(9),
  pht = arr(3),
  a = arr(9),
  at = arr(9),
  ap = arr(9),
  cp = arr(9)
}
local process = ffi.new("float[4]", { 0.01, 0, 0, 0.0025 })

local function finite(value)
  return value == value and value > -finite_limit and value < finite_limit
end

local function valid_candidate(x0, x1, x2)
  if not finite(x0) or not finite(x1) or not finite(x2) then
    return false
  end
  for i = 0, 8 do
    if not finite(s.cp[i]) then
      return false
    end
  end
  if s.cp[0] <= 0 or s.cp[4] <= 0 or s.cp[8] <= 0 then
    return false
  end
  return math.abs(s.cp[1] - s.cp[3]) <= symmetry_tolerance
    and math.abs(s.cp[2] - s.cp[6]) <= symmetry_tolerance
    and math.abs(s.cp[5] - s.cp[7]) <= symmetry_tolerance
end

local function transpose(a, rows, cols, out)
  for c = 0, cols - 1 do
    for r = 0, rows - 1 do
      out[c + r * cols] = a[r + c * rows]
    end
  end
end

local function matmul(a, rows, inner, b, cols, out)
  for c = 0, cols - 1 do
    for r = 0, rows - 1 do
      local sum = 0
      for k = 0, inner - 1 do
        sum = sum + a[r + k * rows] * b[k + c * inner]
      end
      out[r + c * rows] = sum
    end
  end
end

local function step(lane)
  local xi = lane * 3
  local qi = lane * 9
  local theta = state[xi + 2]
  local st = sin(theta)
  local ct = cos(theta)
  local distance = velocity[lane] * 0.1
  local x0 = state[xi] + distance * ct
  local x1 = state[xi + 1] + distance * st
  local x2 = theta + omega[lane] * 0.1
  s.f[0] = 1
  s.f[1] = 0
  s.f[2] = 0
  s.f[3] = 0
  s.f[4] = 1
  s.f[5] = 0
  s.f[6] = -distance * st
  s.f[7] = distance * ct
  s.f[8] = 1
  s.g[0] = ct * 0.1
  s.g[1] = st * 0.1
  s.g[2] = 0
  s.g[3] = 0
  s.g[4] = 0
  s.g[5] = 0.1
  transpose(s.f, 3, 3, s.ft)
  matmul(s.f, 3, 3, covariance + qi, 3, s.left)
  matmul(s.left, 3, 3, s.ft, 3, s.pp)
  matmul(s.g, 3, 2, process, 2, s.ql)
  transpose(s.g, 3, 2, s.gt)
  matmul(s.ql, 3, 2, s.gt, 3, s.qp)
  for i = 0, 8 do
    s.pp[i] = s.pp[i] + s.qp[i]
  end
  local dx = 140 - x0
  local dy = 12 - x1
  local range2 = dx * dx + dy * dy
  local predicted = atan2(dy, dx) - x2
  local raw = bearing[lane] - predicted
  local innovation = atan2(sin(raw), cos(raw))
  local h0 = dy / range2
  local h1 = -dx / range2
  local h2 = -1
  for r = 0, 2 do
    s.pht[r] = s.pp[r] * h0 + s.pp[r + 3] * h1 + s.pp[r + 6] * h2
  end
  local variance = h0 * s.pht[0] + h1 * s.pht[1] + h2 * s.pht[2] + 0.25
  local k0 = s.pht[0] / variance
  local k1 = s.pht[1] / variance
  local k2 = s.pht[2] / variance
  local next0 = x0 + k0 * innovation
  local next1 = x1 + k1 * innovation
  local next2 = x2 + k2 * innovation
  local k = { k0, k1, k2 }
  local h = { h0, h1, h2 }
  for c = 0, 2 do
    for r = 0, 2 do
      s.a[r + c * 3] = (r == c and 1 or 0) - k[r + 1] * h[c + 1]
    end
  end
  transpose(s.a, 3, 3, s.at)
  matmul(s.a, 3, 3, s.pp, 3, s.ap)
  matmul(s.ap, 3, 3, s.at, 3, s.cp)
  for c = 0, 2 do
    for r = 0, 2 do
      s.cp[r + c * 3] = s.cp[r + c * 3] + k[r + 1] * k[c + 1] * 0.25
    end
  end
  if checked and not valid_candidate(next0, next1, next2) then
    return false
  end
  state[xi] = next0
  state[xi + 1] = next1
  state[xi + 2] = next2
  for c = 0, 2 do
    for r = 0, 2 do
      covariance[qi + r + c * 3] = s.cp[r + c * 3]
    end
  end
  return true
end

local function dispatch(count)
  local faults = 0
  for _ = 1, count do
    for lane = 0, instances - 1 do
      if not step(lane) then
        faults = faults + 1
      end
    end
  end
  return faults
end

dispatch(5)
reset()
local started = os.clock()
local faults = dispatch(turns)
local elapsed = os.clock() - started
local checksum = 0
for i = 0, instances * 3 - 1 do
  checksum = checksum + state[i]
end
for i = 0, instances * 9 - 1 do
  checksum = checksum + covariance[i]
end
print("lane: LuaJIT scalar outer loop")
print("instances: " .. instances)
print("turns: " .. turns)
print("validation: " .. (checked and "checked" or "unchecked"))
print("faults: " .. faults)
print(string.format("elapsed_s: %.9f", elapsed))
print(string.format("throughput: %.3f", instances * turns / elapsed))
print(string.format("checksum: %.9f", checksum))
