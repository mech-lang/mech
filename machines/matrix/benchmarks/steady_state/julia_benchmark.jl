#!/usr/bin/env julia

using LinearAlgebra
using Statistics

const SAMPLE_COUNT = 9
const TARGET_SAMPLE_SECONDS = 0.075
const WARMUP_SECONDS = 0.25

function measure(operation)
    started = time()
    warmup_iterations = 0
    while warmup_iterations < 2 || time() - started < WARMUP_SECONDS
        operation()
        warmup_iterations += 1
    end

    started_ns = time_ns()
    operation()
    per_iteration = max((time_ns() - started_ns) / 1.0e9, 1.0e-9)
    batch_iterations = clamp(ceil(Int, TARGET_SAMPLE_SECONDS / per_iteration), 1, 100_000)

    samples = Float64[]
    for _ in 1:SAMPLE_COUNT
        GC.gc()
        started_ns = time_ns()
        for _ in 1:batch_iterations
            operation()
        end
        push!(samples, (time_ns() - started_ns) / 1.0e6 / batch_iterations)
    end
    sort!(samples)
    return median(samples), first(samples), last(samples), batch_iterations
end

matrix_value(row, column, salt) = (mod(row * 17 + column * 13 + salt * 19, 101) + 1) / 101.0

function multiply_inputs(size)
    lhs = Matrix{Float64}(undef, size, size)
    rhs = Matrix{Float64}(undef, size, size)
    for column in 0:size-1, row in 0:size-1
        lhs[row + 1, column + 1] = matrix_value(row, column, 1)
        rhs[row + 1, column + 1] = matrix_value(row, column, 2)
    end
    return lhs, rhs
end

function solve_inputs(size)
    matrix = Matrix{Float64}(undef, size, size)
    for column in 0:size-1, row in 0:size-1
        matrix[row + 1, column + 1] = row == column ? size + 4.0 : mod(row * 7 + column * 11, 19) * 0.01 - 0.09
    end
    rhs = [mod(row, 17) + 1.0 for row in 0:size-1] ./ 17.0
    return matrix, rhs
end

function emit(operation, size, result, check)
    median_ms, minimum_ms, maximum_ms, iterations = result
    println("julia,$operation,$size,$(round(median_ms, digits=9)),$(round(minimum_ms, digits=9)),$(round(maximum_ms, digits=9)),$iterations,$(round(check, digits=12))")
end

function benchmark(size)
    lhs, rhs = multiply_inputs(size)
    product = zeros(size, size)
    matmul_result = measure(() -> mul!(product, lhs, rhs))
    expected = dot(@view(lhs[1, :]), @view(rhs[:, 1]))
    @assert abs(product[1, 1] - expected) < 1.0e-8

    scaled = similar(lhs)
    transposed = similar(lhs)
    pulse = Ref(1.0)
    function transpose_operation()
        pulse[] = pulse[] == 1.0 ? 1.000_001 : 1.0
        @. scaled = lhs * pulse[]
        copyto!(transposed, transpose(scaled))
    end
    transpose_result = measure(transpose_operation)
    check_index = min(2, size)
    @assert abs(transposed[1, check_index] - lhs[check_index, 1] * pulse[]) < 1.0e-12

    solve_matrix, solve_rhs = solve_inputs(size)
    solve_output = zeros(size)
    solve_result = measure(() -> copyto!(solve_output, solve_matrix \ solve_rhs))
    residual = maximum(abs.(solve_matrix * solve_output - solve_rhs))
    @assert residual < 1.0e-8

    emit("matmul", size, matmul_result, product[1, 1])
    emit("transpose", size, transpose_result, transposed[1, check_index])
    emit("solve", size, solve_result, solve_output[1])
end

isempty(ARGS) && error("provide one or more positive matrix sizes")
println("runtime,operation,size,median_ms,min_ms,max_ms,batch_iterations,check")
for size in parse.(Int, ARGS)
    size > 0 || error("matrix sizes must be positive")
    benchmark(size)
end
