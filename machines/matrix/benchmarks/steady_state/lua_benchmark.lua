local runtime = arg[1]
local sample_count = 9
local target_sample_seconds = 0.075
local warmup_seconds = 0.25

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
  collectgarbage("stop")
  local samples = {}
  for sample = 1, sample_count do
    start = os.clock()
    for _ = 1, batch_iterations do
      operation()
    end
    samples[sample] = (os.clock() - start) * 1000.0 / batch_iterations
  end
  collectgarbage("restart")
  table.sort(samples)
  return samples[5], samples[1], samples[9], batch_iterations
end

local function matrix_value(row, column, salt)
  return ((row * 17 + column * 13 + salt * 19) % 101 + 1) / 101.0
end

local function multiply_inputs(size)
  local lhs, rhs = {}, {}
  for row = 0, size - 1 do
    for column = 0, size - 1 do
      local index = row * size + column + 1
      lhs[index] = matrix_value(row, column, 1)
      rhs[index] = matrix_value(row, column, 2)
    end
  end
  return lhs, rhs
end

local function solve_inputs(size)
  local matrix, rhs = {}, {}
  for row = 0, size - 1 do
    for column = 0, size - 1 do
      local index = row * size + column + 1
      if row == column then
        matrix[index] = size + 4.0
      else
        matrix[index] = ((row * 7 + column * 11) % 19) * 0.01 - 0.09
      end
    end
    rhs[row + 1] = (row % 17 + 1) / 17.0
  end
  return matrix, rhs
end

local function matmul(size)
  local lhs, rhs = multiply_inputs(size)
  local output = {}
  for index = 1, size * size do output[index] = 0.0 end
  local function operation()
    for index = 1, size * size do output[index] = 0.0 end
    for row = 0, size - 1 do
      local row_offset = row * size
      for inner = 0, size - 1 do
        local lhs_value = lhs[row_offset + inner + 1]
        local rhs_offset = inner * size
        for column = 0, size - 1 do
          local output_index = row_offset + column + 1
          output[output_index] = output[output_index] + lhs_value * rhs[rhs_offset + column + 1]
        end
      end
    end
  end
  local median, minimum, maximum, iterations = measure(operation)
  local expected = 0.0
  for inner = 0, size - 1 do expected = expected + lhs[inner + 1] * rhs[inner * size + 1] end
  assert(math.abs(output[1] - expected) < 1e-8)
  return median, minimum, maximum, iterations, output[1]
end

local function transpose(size)
  local input_matrix = multiply_inputs(size)
  local scaled = {}
  local output = {}
  for index = 1, size * size do scaled[index], output[index] = 0.0, 0.0 end
  local pulse = 1.0
  local function operation()
    pulse = pulse == 1.0 and 1.000001 or 1.0
    for index = 1, size * size do scaled[index] = input_matrix[index] * pulse end
    for row = 0, size - 1 do
      local row_offset = row * size
      for column = 0, size - 1 do
        output[column * size + row + 1] = scaled[row_offset + column + 1]
      end
    end
  end
  local median, minimum, maximum, iterations = measure(operation)
  local check_index = math.min(1, size - 1)
  assert(math.abs(output[check_index + 1] - input_matrix[check_index * size + 1] * pulse) < 1e-12)
  return median, minimum, maximum, iterations, output[check_index + 1]
end

local function solve(size)
  local matrix, rhs = solve_inputs(size)
  local work, work_rhs, output = {}, {}, {}
  for index = 1, size * size do work[index] = 0.0 end
  for index = 1, size do work_rhs[index], output[index] = 0.0, 0.0 end
  local function operation()
    for index = 1, size * size do work[index] = matrix[index] end
    for index = 1, size do work_rhs[index] = rhs[index] end
    for pivot_column = 0, size - 1 do
      local pivot_row = pivot_column
      local pivot_value = math.abs(work[pivot_column * size + pivot_column + 1])
      for row = pivot_column + 1, size - 1 do
        local candidate = math.abs(work[row * size + pivot_column + 1])
        if candidate > pivot_value then
          pivot_row, pivot_value = row, candidate
        end
      end
      if pivot_row ~= pivot_column then
        local pivot_offset, swap_offset = pivot_column * size, pivot_row * size
        for column = pivot_column, size - 1 do
          local pivot_index, swap_index = pivot_offset + column + 1, swap_offset + column + 1
          work[pivot_index], work[swap_index] = work[swap_index], work[pivot_index]
        end
        work_rhs[pivot_column + 1], work_rhs[pivot_row + 1] = work_rhs[pivot_row + 1], work_rhs[pivot_column + 1]
      end
      local pivot_offset = pivot_column * size
      local pivot = work[pivot_offset + pivot_column + 1]
      for row = pivot_column + 1, size - 1 do
        local row_offset = row * size
        local factor = work[row_offset + pivot_column + 1] / pivot
        work[row_offset + pivot_column + 1] = 0.0
        for column = pivot_column + 1, size - 1 do
          local index = row_offset + column + 1
          work[index] = work[index] - factor * work[pivot_offset + column + 1]
        end
        work_rhs[row + 1] = work_rhs[row + 1] - factor * work_rhs[pivot_column + 1]
      end
    end
    for row = size - 1, 0, -1 do
      local row_offset = row * size
      local total = work_rhs[row + 1]
      for column = row + 1, size - 1 do
        total = total - work[row_offset + column + 1] * output[column + 1]
      end
      output[row + 1] = total / work[row_offset + row + 1]
    end
  end
  local median, minimum, maximum, iterations = measure(operation)
  local residual = 0.0
  for row = 0, size - 1 do
    local actual = 0.0
    for column = 0, size - 1 do actual = actual + matrix[row * size + column + 1] * output[column + 1] end
    residual = math.max(residual, math.abs(actual - rhs[row + 1]))
  end
  assert(residual < 1e-8, residual)
  return median, minimum, maximum, iterations, output[1]
end

print("runtime,operation,size,median_ms,min_ms,max_ms,batch_iterations,check")
for index = 2, #arg do
  local size = assert(tonumber(arg[index]), "sizes must be integers")
  local mm, mm_min, mm_max, mm_iterations, mm_check = matmul(size)
  print(string.format("%s,matmul,%d,%.9f,%.9f,%.9f,%d,%.12f", runtime, size, mm, mm_min, mm_max, mm_iterations, mm_check))
  local tr, tr_min, tr_max, tr_iterations, tr_check = transpose(size)
  print(string.format("%s,transpose,%d,%.9f,%.9f,%.9f,%d,%.12f", runtime, size, tr, tr_min, tr_max, tr_iterations, tr_check))
  local slv, slv_min, slv_max, slv_iterations, slv_check = solve(size)
  print(string.format("%s,solve,%d,%.9f,%.9f,%.9f,%d,%.12f", runtime, size, slv, slv_min, slv_max, slv_iterations, slv_check))
end
