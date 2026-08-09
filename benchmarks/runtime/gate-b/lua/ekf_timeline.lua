#!/usr/bin/env lua

-- Ordered scalar Lua timing samples for the frozen Gate B EKF.

local EPISODE_LENGTH = 4096
local DT = 0.05
local LANDMARK = {25.0, -10.0}
local Q = {0.04, 0.0, 0.0, 0.0025}
local R = {0.25, 0.0, 0.0, 0.0009}
local INITIAL_STATE = {2.0, 1.0, 0.15}
local INITIAL_COVARIANCE = {1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.05}
local EXPECTED_STATE = {18.169827258925427, 4.339708695271022, 0.2557219366745068}
local atan2 = math.atan2 or function(y, x) return math.atan(y, x) end

local function script_root()
  local path = arg[0]:gsub("\\", "/")
  return path:match("^(.*)/benchmarks/runtime/gate%-b/lua/ekf_timeline%.lua$") or "."
end

local function load_trace()
  local path = script_root() .. "/benchmarks/runtime/gate-b/ekf-input-v1.bin"
  local handle = assert(io.open(path, "rb"))
  local bytes = handle:read("*a")
  handle:close()
  assert(#bytes == EPISODE_LENGTH * 32, "Gate B trace has the wrong size")
  local rows = {}
  if string.unpack then
    local offset = 1
    for turn = 1, EPISODE_LENGTH do
      local v, omega, range, bearing
      v, omega, range, bearing, offset = string.unpack("<dddd", bytes, offset)
      rows[turn] = {v, omega, range, bearing}
    end
  else
    local ffi = require("ffi")
    local values = ffi.cast("const double *", bytes)
    for turn = 1, EPISODE_LENGTH do
      local index = (turn - 1) * 4
      rows[turn] = {
        tonumber(values[index]), tonumber(values[index + 1]),
        tonumber(values[index + 2]), tonumber(values[index + 3]),
      }
    end
  end
  return rows
end

local TRACE = load_trace()

local function multiply(left, left_rows, left_columns, right, right_columns)
  local result = {}
  for column = 0, right_columns - 1 do
    for row = 0, left_rows - 1 do
      local total = 0.0
      for inner = 0, left_columns - 1 do
        total = total + left[inner * left_rows + row + 1]
          * right[column * left_columns + inner + 1]
      end
      result[column * left_rows + row + 1] = total
    end
  end
  return result
end

local function transpose(matrix, rows, columns)
  local result = {}
  for column = 0, columns - 1 do
    for row = 0, rows - 1 do
      result[row * columns + column + 1] = matrix[column * rows + row + 1]
    end
  end
  return result
end

local function add(left, right)
  local result = {}
  for index = 1, #left do result[index] = left[index] + right[index] end
  return result
end

local function step(state, covariance, input)
  local px, py, theta = state[1], state[2], state[3]
  local velocity, angular_velocity = input[1], input[2]
  local cosine, sine = math.cos(theta), math.sin(theta)
  local g = {
    1.0, 0.0, 0.0, 0.0, 1.0, 0.0,
    -velocity * sine * DT, velocity * cosine * DT, 1.0,
  }
  local v = {cosine * DT, sine * DT, 0.0, 0.0, 0.0, DT}
  local predicted_state = {
    px + velocity * cosine * DT,
    py + velocity * sine * DT,
    theta + angular_velocity * DT,
  }
  local gp = multiply(g, 3, 3, covariance, 3)
  local predicted_covariance = multiply(gp, 3, 3, transpose(g, 3, 3), 3)
  local vq = multiply(v, 3, 2, Q, 2)
  predicted_covariance = add(
    predicted_covariance,
    multiply(vq, 3, 2, transpose(v, 3, 2), 3)
  )
  local delta_x = LANDMARK[1] - predicted_state[1]
  local delta_y = LANDMARK[2] - predicted_state[2]
  local q = delta_x * delta_x + delta_y * delta_y
  local distance = math.sqrt(q)
  local h = {
    -delta_x / distance, delta_y / q,
    -delta_y / distance, -delta_x / q,
    0.0, -1.0,
  }
  local hp = multiply(h, 2, 3, predicted_covariance, 3)
  local s = add(multiply(hp, 2, 3, transpose(h, 2, 3), 2), R)
  local determinant = s[1] * s[4] - s[3] * s[2]
  local inverse = {
    s[4] / determinant, -s[2] / determinant,
    -s[3] / determinant, s[1] / determinant,
  }
  local pht = multiply(predicted_covariance, 3, 3, transpose(h, 2, 3), 2)
  local gain = multiply(pht, 3, 2, inverse, 2)
  local innovation = {
    input[3] - distance,
    input[4] - (atan2(delta_y, delta_x) - predicted_state[3]),
  }
  local correction = multiply(gain, 3, 2, innovation, 1)
  local corrected_state = {
    predicted_state[1] + correction[1],
    predicted_state[2] + correction[2],
    predicted_state[3] + correction[3],
  }
  local kh = multiply(gain, 3, 2, h, 3)
  local identity = {1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0}
  local a = {}
  for index = 1, 9 do a[index] = identity[index] - kh[index] end
  local ap = multiply(a, 3, 3, predicted_covariance, 3)
  local joseph = multiply(ap, 3, 3, transpose(a, 3, 3), 3)
  local kr = multiply(gain, 3, 2, R, 2)
  local corrected = add(joseph, multiply(kr, 3, 2, transpose(gain, 3, 2), 3))
  local corrected_t = transpose(corrected, 3, 3)
  for index = 1, 9 do corrected[index] = 0.5 * (corrected[index] + corrected_t[index]) end
  assert(q > 1.0e-12, "landmark distance")
  assert(math.abs(determinant) > 1.0e-12, "innovation determinant")
  for index = 1, 3 do
    assert(corrected_state[index] == corrected_state[index], "non-finite state")
    assert(math.abs(corrected_state[index]) < math.huge, "non-finite state")
  end
  for index = 1, 9 do
    assert(corrected[index] == corrected[index], "non-finite covariance")
    assert(math.abs(corrected[index]) < math.huge, "non-finite covariance")
  end
  assert(corrected[1] > 0.0 and corrected[5] > 0.0 and corrected[9] > 0.0,
    "covariance diagonal")
  local symmetry_error = 0.0
  for column = 0, 2 do
    for row = 0, 2 do
      symmetry_error = math.max(symmetry_error,
        math.abs(corrected[column * 3 + row + 1] - corrected[row * 3 + column + 1]))
    end
  end
  assert(symmetry_error <= 1.0e-10, "covariance symmetry")
  return corrected_state, corrected
end

local function reset_state()
  local state = {INITIAL_STATE[1], INITIAL_STATE[2], INITIAL_STATE[3]}
  local covariance = {}
  for index = 1, 9 do covariance[index] = INITIAL_COVARIANCE[index] end
  return state, covariance
end

local function run_episode(state, covariance)
  for turn = 1, EPISODE_LENGTH do state, covariance = step(state, covariance, TRACE[turn]) end
  return state, covariance
end

local samples = 60
for index = 1, #arg do
  if arg[index] == "--samples" then samples = assert(tonumber(arg[index + 1])) end
end
assert(samples > 0, "--samples must be positive")

local lane = type(jit) == "table" and "luajit-scalar" or "lua-scalar"
for sample = 0, samples do
  local state, covariance = reset_state()
  local memory_before = collectgarbage("count")
  local started = os.clock()
  state, covariance = run_episode(state, covariance)
  local elapsed_ns = math.floor((os.clock() - started) * 1.0e9 + 0.5)
  for index = 1, 3 do
    assert(math.abs(state[index] - EXPECTED_STATE[index]) <= 1.0e-9, "final state mismatch")
  end
  if sample > 0 then
    local memory_after = collectgarbage("count")
    local gc_cycle_inferred = memory_after < memory_before and "true" or "false"
    print(string.format(
      '{"lane":"%s","sample":%d,"turns":%d,"elapsed_ns":%d,"gc_ns":null,"gc_cycle_inferred":%s,"heap_kb_before":%.3f,"heap_kb_after":%.3f,"clock":"process_cpu"}',
      lane, sample - 1, EPISODE_LENGTH, elapsed_ns,
      gc_cycle_inferred, memory_before, memory_after
    ))
  end
end
