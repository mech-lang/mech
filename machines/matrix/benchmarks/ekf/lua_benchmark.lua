local runtime = assert(arg[1], "runtime label must be lua or luajit")
local sample_count = 9
local input_period = 4096
local target_sample_seconds = 0.075
local warmup_seconds = 0.25
local dt = 0.1
local landmark_x = 140.0
local landmark_y = 12.0
local measurement_noise = 0.25
local tau = 2.0 * math.pi
local atan2 = math.atan2 or math.atan

local function wrap_angle(angle)
  return atan2(math.sin(angle), math.cos(angle))
end

local function input_samples()
  local truth = {45.0, 15.0, 0.0}
  local samples = {}
  local base_angular_velocity = tau / (input_period * dt)
  for index = 0, input_period - 1 do
    local phase = tau * index / input_period
    local linear_velocity = 1.0 + 0.05 * math.sin(phase * 3.0)
    local angular_velocity = base_angular_velocity * (1.0 + 0.1 * math.cos(phase * 2.0))
    truth[1] = truth[1] + linear_velocity * math.cos(truth[3]) * dt
    truth[2] = truth[2] + linear_velocity * math.sin(truth[3]) * dt
    truth[3] = wrap_angle(truth[3] + angular_velocity * dt)
    local noise = 0.01 * math.sin(phase * 7.0) + 0.005 * math.cos(phase * 11.0)
    local bearing = wrap_angle(atan2(landmark_y - truth[2], landmark_x - truth[1]) - truth[3] + noise)
    samples[index + 1] = {linear_velocity, angular_velocity, bearing}
  end
  return samples
end

local function transpose(matrix, rows, columns)
  local output = {}
  for row = 0, rows - 1 do
    for column = 0, columns - 1 do
      output[column * rows + row + 1] = matrix[row * columns + column + 1]
    end
  end
  return output
end

local function matmul(lhs, lhs_rows, inner, rhs, rhs_columns)
  local output = {}
  for row = 0, lhs_rows - 1 do
    local lhs_offset = row * inner
    local output_offset = row * rhs_columns
    for column = 0, rhs_columns - 1 do
      local total = 0.0
      for index = 0, inner - 1 do
        total = total + lhs[lhs_offset + index + 1] * rhs[index * rhs_columns + column + 1]
      end
      output[output_offset + column + 1] = total
    end
  end
  return output
end

local function matrix_add(lhs, rhs)
  local output = {}
  for index = 1, #lhs do output[index] = lhs[index] + rhs[index] end
  return output
end

local function matrix_subtract(lhs, rhs)
  local output = {}
  for index = 1, #lhs do output[index] = lhs[index] - rhs[index] end
  return output
end

local function matrix_scale(matrix, scale)
  local output = {}
  for index = 1, #matrix do output[index] = matrix[index] * scale end
  return output
end

local function new_ekf()
  return {
    state = {55.0, 25.0, 0.4},
    covariance = {100.0, 0.0, 0.0, 0.0, 100.0, 0.0, 0.0, 0.0, 0.15},
  }
end

local function ekf_turn(ekf, sample)
  local linear_velocity, angular_velocity, bearing = sample[1], sample[2], sample[3]
  local theta = ekf.state[3]
  local sin_theta, cos_theta = math.sin(theta), math.cos(theta)
  local distance = linear_velocity * dt
  local predicted_state = {
    ekf.state[1] + distance * cos_theta,
    ekf.state[2] + distance * sin_theta,
    ekf.state[3] + angular_velocity * dt,
  }
  local motion_jacobian = {
    1.0, 0.0, -distance * sin_theta,
    0.0, 1.0, distance * cos_theta,
    0.0, 0.0, 1.0,
  }
  local control_jacobian = {
    cos_theta * dt, 0.0,
    sin_theta * dt, 0.0,
    0.0, dt,
  }
  local process_noise = {0.01, 0.0, 0.0, 0.0025}
  local predicted_covariance = matrix_add(
    matmul(
      matmul(motion_jacobian, 3, 3, ekf.covariance, 3),
      3,
      3,
      transpose(motion_jacobian, 3, 3),
      3
    ),
    matmul(
      matmul(control_jacobian, 3, 2, process_noise, 2),
      3,
      2,
      transpose(control_jacobian, 3, 2),
      3
    )
  )

  local delta_x = landmark_x - predicted_state[1]
  local delta_y = landmark_y - predicted_state[2]
  local squared_range = delta_x * delta_x + delta_y * delta_y
  local predicted_bearing = atan2(delta_y, delta_x) - predicted_state[3]
  local innovation = wrap_angle(bearing - predicted_bearing)
  local observation_jacobian = {delta_y / squared_range, -delta_x / squared_range, -1.0}
  local innovation_variance = matmul(
    matmul(observation_jacobian, 1, 3, predicted_covariance, 3),
    1,
    3,
    transpose(observation_jacobian, 1, 3),
    1
  )[1] + measurement_noise
  local gain = matrix_scale(
    matmul(predicted_covariance, 3, 3, transpose(observation_jacobian, 1, 3), 1),
    1.0 / innovation_variance
  )

  ekf.state = {
    predicted_state[1] + gain[1] * innovation,
    predicted_state[2] + gain[2] * innovation,
    predicted_state[3] + gain[3] * innovation,
  }
  local identity = {1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0}
  local correction = matrix_subtract(identity, matmul(gain, 3, 1, observation_jacobian, 3))
  ekf.covariance = matrix_add(
    matmul(
      matmul(correction, 3, 3, predicted_covariance, 3),
      3,
      3,
      transpose(correction, 3, 3),
      3
    ),
    matrix_scale(matmul(gain, 3, 1, transpose(gain, 3, 1), 3), measurement_noise)
  )
end

local function ekf_check(ekf)
  return ekf.state[1] + ekf.state[2] + ekf.state[3]
    + ekf.covariance[1] + ekf.covariance[5] + ekf.covariance[9]
end

local function validate(samples)
  local expected_state = {80.35682278971947, 24.769631959523576, 0.34224350117788876}
  local expected_covariance = {
    100.24018212103414, -2.34499352078198, -0.23555533772422113,
    -2.3449935207819896, 46.240533034085935, -0.716187029947414,
    -0.23555533772422071, -0.7161870299474145, 0.014716073850468109,
  }
  local ekf = new_ekf()
  for index = 1, 256 do ekf_turn(ekf, samples[index]) end
  local error = 0.0
  for index = 1, 3 do error = math.max(error, math.abs(ekf.state[index] - expected_state[index])) end
  for index = 1, 9 do
    error = math.max(error, math.abs(ekf.covariance[index] - expected_covariance[index]))
  end
  assert(error < 1e-7, string.format("EKF validation error %.12g", error))
end

local function measure(operation)
  local start = os.clock()
  local warmup_iterations = 0
  repeat
    operation()
    warmup_iterations = warmup_iterations + 1
  until warmup_iterations >= 2 and os.clock() - start >= warmup_seconds

  start = os.clock()
  operation()
  local per_iteration = math.max(os.clock() - start, 1e-9)
  local batch_iterations = math.max(1, math.min(100000, math.ceil(target_sample_seconds / per_iteration)))

  collectgarbage("collect")
  collectgarbage("restart")
  local timings = {}
  for sample = 1, sample_count do
    start = os.clock()
    for _ = 1, batch_iterations do operation() end
    timings[sample] = (os.clock() - start) * 1000.0 / batch_iterations
  end
  table.sort(timings)
  return timings[5], timings[1], timings[9], batch_iterations
end

local samples = input_samples()
validate(samples)
local ekf = new_ekf()
local input_index = 1
local function operation()
  ekf_turn(ekf, samples[input_index])
  input_index = input_index == #samples and 1 or input_index + 1
end

local median, minimum, maximum, iterations = measure(operation)
assert(ekf_check(ekf) == ekf_check(ekf))
print("runtime,operation,median_ms,min_ms,max_ms,batch_iterations,check")
print(string.format(
  "%s-loop,ekf,%.9f,%.9f,%.9f,%d,%.12f",
  runtime,
  median,
  minimum,
  maximum,
  iterations,
  ekf_check(ekf)
))
