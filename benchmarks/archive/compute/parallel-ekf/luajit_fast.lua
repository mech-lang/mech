local has_ffi, ffi = pcall(require, "ffi")
if not has_ffi then ffi = nil end

local sin, cos, abs, pi = math.sin, math.cos, math.abs, math.pi
local atan2 = math.atan2 or function(y, x) return math.atan(y, x) end
local instances = math.max(1, tonumber(arg[1]) or 10000)
local turns = math.max(1, tonumber(arg[2]) or 5)
local checked = string.lower(arg[3] or "unchecked") == "checked"
local dt = 0.1
local dt2 = dt * dt
local q0, q1, measurement_noise = 0.01, 0.0025, 0.25
local symmetry_tolerance = 0.0001
local finite_limit = 3.402823466e38

local function array()
  if ffi then
    return ffi.new("float[?]", instances)
  end
  -- Lua tables do not have the zero-initialization that FFI arrays provide.
  -- Seed every lane so the pre-reset warmup has the same defined inputs.
  local result = {}
  for lane = 0, instances - 1 do
    result[lane] = 0.0
  end
  return result
end

local velocity, angular_velocity, bearing = array(), array(), array()
local x0, x1, x2 = array(), array(), array()
local p00, p01, p02, p10, p11, p12, p20, p21, p22 = array(), array(), array(), array(), array(), array(), array(), array(), array()

local function reset()
  for lane = 0, instances - 1 do
    x0[lane], x1[lane], x2[lane] = 55.0, 25.0, 0.4
    p00[lane], p01[lane], p02[lane] = 100.0, 0.0, 0.0
    p10[lane], p11[lane], p12[lane] = 0.0, 100.0, 0.0
    p20[lane], p21[lane], p22[lane] = 0.0, 0.0, 0.15
  end
end

for lane = 0, instances - 1 do
  local phase = 2.0 * pi * lane / instances
  velocity[lane] = 1.0 + 0.05 * sin(3.0 * phase)
  angular_velocity[lane] = 0.015 * (1.0 + 0.1 * sin(2.0 * phase))
  bearing[lane] = -0.55 + 0.01 * sin(7.0 * phase) + 0.005 * sin(11.0 * phase)
end

local function finite(value)
  return value == value and value <= finite_limit and value >= -finite_limit
end

local function step(lane)
  local theta = x2[lane]
  local st, ct = sin(theta), cos(theta)
  local distance = velocity[lane] * dt
  local predicted_x0 = x0[lane] + distance * ct
  local predicted_x1 = x1[lane] + distance * st
  local predicted_x2 = theta + angular_velocity[lane] * dt
  local f02, f12 = -distance * st, distance * ct

  local ap00 = p00[lane] + f02 * p20[lane]
  local ap01 = p01[lane] + f02 * p21[lane]
  local ap02 = p02[lane] + f02 * p22[lane]
  local ap10 = p10[lane] + f12 * p20[lane]
  local ap11 = p11[lane] + f12 * p21[lane]
  local ap12 = p12[lane] + f12 * p22[lane]
  local ap20, ap21, ap22 = p20[lane], p21[lane], p22[lane]
  local process00 = ct * ct * dt2 * q0
  local process01 = ct * st * dt2 * q0
  local process11 = st * st * dt2 * q0
  local process22 = dt2 * q1
  local predicted_p00 = ap00 + ap02 * f02 + process00
  local predicted_p01 = ap01 + ap02 * f12 + process01
  local predicted_p02 = ap02
  local predicted_p10 = ap10 + ap12 * f02 + process01
  local predicted_p11 = ap11 + ap12 * f12 + process11
  local predicted_p12 = ap12
  local predicted_p20 = ap20 + ap22 * f02
  local predicted_p21 = ap21 + ap22 * f12
  local predicted_p22 = ap22 + process22

  local dx, dy = 140.0 - predicted_x0, 12.0 - predicted_x1
  local squared_range = dx * dx + dy * dy
  local predicted_bearing = atan2(dy, dx) - predicted_x2
  local raw_innovation = bearing[lane] - predicted_bearing
  local innovation = atan2(sin(raw_innovation), cos(raw_innovation))
  local h0, h1, h2 = dy / squared_range, -dx / squared_range, -1.0
  local pht0 = predicted_p00 * h0 + predicted_p01 * h1 + predicted_p02 * h2
  local pht1 = predicted_p10 * h0 + predicted_p11 * h1 + predicted_p12 * h2
  local pht2 = predicted_p20 * h0 + predicted_p21 * h1 + predicted_p22 * h2
  local variance = h0 * pht0 + h1 * pht1 + h2 * pht2 + measurement_noise
  local k0, k1, k2 = pht0 / variance, pht1 / variance, pht2 / variance
  local candidate_x0 = predicted_x0 + k0 * innovation
  local candidate_x1 = predicted_x1 + k1 * innovation
  local candidate_x2 = predicted_x2 + k2 * innovation

  local a00, a01, a02 = 1.0 - k0 * h0, -k0 * h1, -k0 * h2
  local a10, a11, a12 = -k1 * h0, 1.0 - k1 * h1, -k1 * h2
  local a20, a21, a22 = -k2 * h0, -k2 * h1, 1.0 - k2 * h2
  local b00 = a00 * predicted_p00 + a01 * predicted_p10 + a02 * predicted_p20
  local b01 = a00 * predicted_p01 + a01 * predicted_p11 + a02 * predicted_p21
  local b02 = a00 * predicted_p02 + a01 * predicted_p12 + a02 * predicted_p22
  local b10 = a10 * predicted_p00 + a11 * predicted_p10 + a12 * predicted_p20
  local b11 = a10 * predicted_p01 + a11 * predicted_p11 + a12 * predicted_p21
  local b12 = a10 * predicted_p02 + a11 * predicted_p12 + a12 * predicted_p22
  local b20 = a20 * predicted_p00 + a21 * predicted_p10 + a22 * predicted_p20
  local b21 = a20 * predicted_p01 + a21 * predicted_p11 + a22 * predicted_p21
  local b22 = a20 * predicted_p02 + a21 * predicted_p12 + a22 * predicted_p22
  local candidate_p00 = b00 * a00 + b01 * a01 + b02 * a02 + k0 * k0 * measurement_noise
  local candidate_p01 = b00 * a10 + b01 * a11 + b02 * a12 + k0 * k1 * measurement_noise
  local candidate_p02 = b00 * a20 + b01 * a21 + b02 * a22 + k0 * k2 * measurement_noise
  local candidate_p10 = b10 * a00 + b11 * a01 + b12 * a02 + k1 * k0 * measurement_noise
  local candidate_p11 = b10 * a10 + b11 * a11 + b12 * a12 + k1 * k1 * measurement_noise
  local candidate_p12 = b10 * a20 + b11 * a21 + b12 * a22 + k1 * k2 * measurement_noise
  local candidate_p20 = b20 * a00 + b21 * a01 + b22 * a02 + k2 * k0 * measurement_noise
  local candidate_p21 = b20 * a10 + b21 * a11 + b22 * a12 + k2 * k1 * measurement_noise
  local candidate_p22 = b20 * a20 + b21 * a21 + b22 * a22 + k2 * k2 * measurement_noise

  if checked then
    local valid = finite(candidate_x0) and finite(candidate_x1) and finite(candidate_x2)
    valid = valid and finite(candidate_p00) and finite(candidate_p01) and finite(candidate_p02)
    valid = valid and finite(candidate_p10) and finite(candidate_p11) and finite(candidate_p12)
    valid = valid and finite(candidate_p20) and finite(candidate_p21) and finite(candidate_p22)
    valid = valid and candidate_p00 > 0.0 and candidate_p11 > 0.0 and candidate_p22 > 0.0
    valid = valid and abs(candidate_p01 - candidate_p10) <= symmetry_tolerance
    valid = valid and abs(candidate_p02 - candidate_p20) <= symmetry_tolerance
    valid = valid and abs(candidate_p12 - candidate_p21) <= symmetry_tolerance
    if not valid then
      return 1
    end
  end

  x0[lane], x1[lane], x2[lane] = candidate_x0, candidate_x1, candidate_x2
  p00[lane], p01[lane], p02[lane] = candidate_p00, candidate_p01, candidate_p02
  p10[lane], p11[lane], p12[lane] = candidate_p10, candidate_p11, candidate_p12
  p20[lane], p21[lane], p22[lane] = candidate_p20, candidate_p21, candidate_p22
  return 0
end

local function dispatch(count)
  local faults = 0
  for _ = 1, count do
    for lane = 0, instances - 1 do
      faults = faults + step(lane)
    end
  end
  return faults
end

dispatch(5)
reset()
local started = os.clock()
local faults = dispatch(turns)
local elapsed = os.clock() - started
local checksum = 0.0
for lane = 0, instances - 1 do
  checksum = checksum + x0[lane] + x1[lane] + x2[lane]
end
for lane = 0, instances - 1 do
  checksum = checksum + p00[lane] + p01[lane] + p02[lane] + p10[lane] + p11[lane] + p12[lane] + p20[lane] + p21[lane] + p22[lane]
end
print("lane: " .. (ffi and "LuaJIT" or "Lua") .. " fixed-shape flat")
print("instances: " .. instances)
print("turns: " .. turns)
print(string.format("elapsed_s: %.9f", elapsed))
print(string.format("throughput: %.3f", instances * turns / elapsed))
print(string.format("checksum: %.9f", checksum))
print("validation: " .. (checked and "checked" or "unchecked"))
print("faults: " .. faults)
