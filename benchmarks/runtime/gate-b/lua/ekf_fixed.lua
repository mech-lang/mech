-- Preallocated fixed-shape Lua/LuaJIT EKF timeline control.

local EPISODE_LENGTH = 4096
local DT = 0.05
local LANDMARK_X, LANDMARK_Y = 25.0, -10.0
local PROCESS_COVARIANCE = {0.04, 0.0, 0.0, 0.0025}
local MEASUREMENT_COVARIANCE = {0.25, 0.0, 0.0, 0.0009}
local INITIAL_STATE = {2.0, 1.0, 0.15}
local INITIAL_COVARIANCE = {1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.05}
local EXPECTED_STATE = {18.169827258925427, 4.339708695271022, 0.2557219366745068}
local EXPECTED_COVARIANCE = {
  0.3270953723043491, 0.1509754472729972, -0.022618166436367253,
  0.1509754472729972, 0.07105284175378412, -0.010486015657880304,
  -0.022618166436367253, -0.010486015657880304, 0.0016395600302299483,
}
local atan2 = math.atan2 or function(y, x) return math.atan(y, x) end

local function script_root()
  local path = arg[0]:gsub("\\", "/")
  return path:match("^(.*)/benchmarks/runtime/gate%-b/lua/ekf_fixed%.lua$") or "."
end

local function load_trace()
  local handle = assert(io.open(script_root() .. "/benchmarks/runtime/gate-b/ekf-input-v1.bin", "rb"))
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

local function buffer(size)
  local values = {}
  for index = 1, size do values[index] = 0.0 end
  return values
end

local function matmul(out, left, left_rows, left_columns, right, right_columns)
  for column = 0, right_columns - 1 do
    for row = 0, left_rows - 1 do
      local total = 0.0
      for inner = 0, left_columns - 1 do
        total = total + left[inner * left_rows + row + 1]
          * right[column * left_columns + inner + 1]
      end
      out[column * left_rows + row + 1] = total
    end
  end
end

local function matmul_right_transpose(out, left, left_rows, inner_size, right, right_rows)
  for column = 0, right_rows - 1 do
    for row = 0, left_rows - 1 do
      local total = 0.0
      for inner = 0, inner_size - 1 do
        total = total + left[inner * left_rows + row + 1]
          * right[inner * right_rows + column + 1]
      end
      out[column * left_rows + row + 1] = total
    end
  end
end

local function workspace()
  return {
    state = buffer(3), covariance = buffer(9), motion_jacobian = buffer(9),
    control_jacobian = buffer(6), predicted_state = buffer(3), gp = buffer(9),
    predicted_covariance = buffer(9), vq = buffer(6), process_covariance = buffer(9),
    measurement_jacobian = buffer(6), hp = buffer(6), innovation_covariance = buffer(4),
    inverse_innovation = buffer(4), pht = buffer(6), gain = buffer(6),
    innovation = buffer(2), correction = buffer(3), kh = buffer(9), joseph_a = buffer(9),
    ap = buffer(9), corrected_covariance = buffer(9), kr = buffer(6),
    measurement_covariance = buffer(9),
  }
end

local function reset(ws)
  for index = 1, 3 do ws.state[index] = INITIAL_STATE[index] end
  for index = 1, 9 do ws.covariance[index] = INITIAL_COVARIANCE[index] end
end

local function step(ws, input)
  local state, covariance = ws.state, ws.covariance
  local velocity, angular_velocity = input[1], input[2]
  local cosine, sine = math.cos(state[3]), math.sin(state[3])
  local g, v = ws.motion_jacobian, ws.control_jacobian
  g[1], g[2], g[3] = 1.0, 0.0, 0.0
  g[4], g[5], g[6] = 0.0, 1.0, 0.0
  g[7], g[8], g[9] = -velocity * sine * DT, velocity * cosine * DT, 1.0
  v[1], v[2], v[3] = cosine * DT, sine * DT, 0.0
  v[4], v[5], v[6] = 0.0, 0.0, DT
  local predicted_state = ws.predicted_state
  predicted_state[1] = state[1] + velocity * cosine * DT
  predicted_state[2] = state[2] + velocity * sine * DT
  predicted_state[3] = state[3] + angular_velocity * DT

  matmul(ws.gp, g, 3, 3, covariance, 3)
  matmul_right_transpose(ws.predicted_covariance, ws.gp, 3, 3, g, 3)
  matmul(ws.vq, v, 3, 2, PROCESS_COVARIANCE, 2)
  matmul_right_transpose(ws.process_covariance, ws.vq, 3, 2, v, 3)
  for index = 1, 9 do
    ws.predicted_covariance[index] = ws.predicted_covariance[index] + ws.process_covariance[index]
  end

  local delta_x = LANDMARK_X - predicted_state[1]
  local delta_y = LANDMARK_Y - predicted_state[2]
  local q = delta_x * delta_x + delta_y * delta_y
  assert(q > 1.0e-12, "landmark distance")
  local distance = math.sqrt(q)
  local predicted_bearing = atan2(delta_y, delta_x) - predicted_state[3]
  local h = ws.measurement_jacobian
  h[1], h[2] = -delta_x / distance, delta_y / q
  h[3], h[4] = -delta_y / distance, -delta_x / q
  h[5], h[6] = 0.0, -1.0
  matmul(ws.hp, h, 2, 3, ws.predicted_covariance, 3)
  matmul_right_transpose(ws.innovation_covariance, ws.hp, 2, 3, h, 2)
  for index = 1, 4 do
    ws.innovation_covariance[index] = ws.innovation_covariance[index] + MEASUREMENT_COVARIANCE[index]
  end
  local s = ws.innovation_covariance
  local determinant = s[1] * s[4] - s[3] * s[2]
  assert(math.abs(determinant) > 1.0e-12, "innovation determinant")
  local inverse = ws.inverse_innovation
  inverse[1], inverse[2] = s[4] / determinant, -s[2] / determinant
  inverse[3], inverse[4] = -s[3] / determinant, s[1] / determinant
  matmul_right_transpose(ws.pht, ws.predicted_covariance, 3, 3, h, 2)
  matmul(ws.gain, ws.pht, 3, 2, inverse, 2)
  ws.innovation[1] = input[3] - distance
  ws.innovation[2] = input[4] - predicted_bearing
  matmul(ws.correction, ws.gain, 3, 2, ws.innovation, 1)
  for index = 1, 3 do state[index] = predicted_state[index] + ws.correction[index] end

  matmul(ws.kh, ws.gain, 3, 2, h, 3)
  for index = 1, 9 do
    ws.joseph_a[index] = ((index == 1 or index == 5 or index == 9) and 1.0 or 0.0) - ws.kh[index]
  end
  matmul(ws.ap, ws.joseph_a, 3, 3, ws.predicted_covariance, 3)
  matmul_right_transpose(ws.corrected_covariance, ws.ap, 3, 3, ws.joseph_a, 3)
  matmul(ws.kr, ws.gain, 3, 2, MEASUREMENT_COVARIANCE, 2)
  matmul_right_transpose(ws.measurement_covariance, ws.kr, 3, 2, ws.gain, 3)
  for index = 1, 9 do
    ws.corrected_covariance[index] = ws.corrected_covariance[index] + ws.measurement_covariance[index]
  end
  for column = 0, 2 do
    for row = 0, column - 1 do
      local left, right = column * 3 + row + 1, row * 3 + column + 1
      local symmetric = 0.5 * (ws.corrected_covariance[left] + ws.corrected_covariance[right])
      ws.corrected_covariance[left], ws.corrected_covariance[right] = symmetric, symmetric
    end
  end
  for index = 1, 9 do covariance[index] = ws.corrected_covariance[index] end
  for index = 1, 3 do assert(state[index] == state[index] and math.abs(state[index]) < math.huge) end
  for index = 1, 9 do assert(covariance[index] == covariance[index] and math.abs(covariance[index]) < math.huge) end
  assert(covariance[1] > 0.0 and covariance[5] > 0.0 and covariance[9] > 0.0)
end

local function run_episode(ws)
  for turn = 1, EPISODE_LENGTH do step(ws, TRACE[turn]) end
end

local samples = 60
for index = 1, #arg do
  if arg[index] == "--samples" then samples = assert(tonumber(arg[index + 1])) end
end
assert(samples > 0, "--samples must be positive")
local ws = workspace()
reset(ws)
run_episode(ws)
for index = 1, 3 do assert(math.abs(ws.state[index] - EXPECTED_STATE[index]) <= 1.0e-9) end
for index = 1, 9 do
  assert(math.abs(ws.covariance[index] - EXPECTED_COVARIANCE[index]) <= 1.0e-9)
end
local runtime = type(jit) == "table" and "luajit-fixed-preallocated" or "lua-fixed-preallocated"
for sample = 0, samples - 1 do
  reset(ws)
  local memory_before = collectgarbage("count")
  local started = os.clock()
  run_episode(ws)
  local elapsed_ns = math.floor((os.clock() - started) * 1.0e9 + 0.5)
  local memory_after = collectgarbage("count")
  print(string.format(
    '{"lane":"%s","sample":%d,"turns":%d,"elapsed_ns":%d,"gc_ns":null,"heap_kb_before":%.3f,"heap_kb_after":%.3f}',
    runtime, sample, EPISODE_LENGTH, elapsed_ns, memory_before, memory_after
  ))
end
