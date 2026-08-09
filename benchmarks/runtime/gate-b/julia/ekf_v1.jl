#!/usr/bin/env julia

# Persistent Julia control for the frozen Gate B EKF v1 workload.

using SHA
using Statistics

const ROOT = normpath(joinpath(@__DIR__, "..", "..", "..", ".."))
const TRACE_PATH = joinpath(ROOT, "benchmarks", "runtime", "gate-b", "ekf-input-v1.bin")
const EPISODE_LENGTH = 4096
const SCALED_INSTANCES = (1, 8, 64)
const DT = 0.05
const LANDMARK_X = 25.0
const LANDMARK_Y = -10.0
const QUANTIZATION = 1.0e-10
const REFERENCE_HASH = "ddca8ab17cb390839d4c77e7cecc5203122f249685f5a28c36fd342cf303a758"
const JULIA_HASH = "04c4f5b1b9b4af787525a4e45bf2270fa0653f43cfcb1d85792ed6977b6259ec"
const EXPECTED_STATE = [18.169827258925427, 4.339708695271022, 0.2557219366745068]
const EXPECTED_COVARIANCE = [
    0.3270953723043491 0.1509754472729972 -0.022618166436367253;
    0.1509754472729972 0.07105284175378412 -0.010486015657880304;
    -0.022618166436367253 -0.010486015657880304 0.0016395600302299483
]
const INITIAL_STATE = [2.0, 1.0, 0.15]
const INITIAL_COVARIANCE = [1.0 0.0 0.0; 0.0 1.0 0.0; 0.0 0.0 0.05]
const PROCESS_COVARIANCE = [0.04 0.0; 0.0 0.0025]
const MEASUREMENT_COVARIANCE = [0.25 0.0; 0.0 0.0009]

function load_trace()
    values = reinterpret(Float64, read(TRACE_PATH))
    length(values) == EPISODE_LENGTH * 4 || error("Gate B trace has the wrong length")
    reshape(copy(values), 4, EPISODE_LENGTH)
end

const TRACE = load_trace()

mutable struct Workspace
    instances::Int
    state::Matrix{Float64}
    covariance::Array{Float64,3}
    predicted_state::Vector{Float64}
    motion_jacobian::Matrix{Float64}
    control_jacobian::Matrix{Float64}
    gp::Matrix{Float64}
    predicted_covariance::Matrix{Float64}
    vq::Matrix{Float64}
    process_covariance::Matrix{Float64}
    measurement_jacobian::Matrix{Float64}
    hp::Matrix{Float64}
    innovation_covariance::Matrix{Float64}
    inverse_innovation::Matrix{Float64}
    pht::Matrix{Float64}
    gain::Matrix{Float64}
    correction::Vector{Float64}
    kh::Matrix{Float64}
    joseph_a::Matrix{Float64}
    ap::Matrix{Float64}
    corrected_covariance::Matrix{Float64}
    kr::Matrix{Float64}
    measurement_covariance::Matrix{Float64}
end

function Workspace(instances::Int)
    instances in SCALED_INSTANCES || error("instances must be 1, 8, or 64")
    ws = Workspace(
        instances, zeros(3, instances), zeros(3, 3, instances), zeros(3),
        zeros(3, 3), zeros(3, 2), zeros(3, 3), zeros(3, 3), zeros(3, 2),
        zeros(3, 3), zeros(2, 3), zeros(2, 3), zeros(2, 2), zeros(2, 2),
        zeros(3, 2), zeros(3, 2), zeros(3), zeros(3, 3), zeros(3, 3),
        zeros(3, 3), zeros(3, 3), zeros(3, 2), zeros(3, 3),
    )
    reset!(ws)
    ws
end

function reset!(ws::Workspace)
    @inbounds for instance in 1:ws.instances
        for row in 1:3
            ws.state[row, instance] = INITIAL_STATE[row]
            for column in 1:3
                ws.covariance[row, column, instance] = INITIAL_COVARIANCE[row, column]
            end
        end
    end
end

function matmul!(output, left, right)
    @inbounds for column in axes(right, 2), row in axes(left, 1)
        total = 0.0
        for inner in axes(left, 2)
            total += left[row, inner] * right[inner, column]
        end
        output[row, column] = total
    end
end

function matmul_right_transpose!(output, left, right)
    @inbounds for column in axes(right, 1), row in axes(left, 1)
        total = 0.0
        for inner in axes(left, 2)
            total += left[row, inner] * right[column, inner]
        end
        output[row, column] = total
    end
end

function step!(ws::Workspace, instance::Int, turn::Int)
    @inbounds begin
        velocity = TRACE[1, turn]
        angular_velocity = TRACE[2, turn]
        measured_range = TRACE[3, turn]
        measured_bearing = TRACE[4, turn]
        theta = ws.state[3, instance]
        cosine = cos(theta)
        sine = sin(theta)

        fill!(ws.motion_jacobian, 0.0)
        ws.motion_jacobian[1, 1] = 1.0
        ws.motion_jacobian[2, 2] = 1.0
        ws.motion_jacobian[3, 3] = 1.0
        ws.motion_jacobian[1, 3] = -velocity * sine * DT
        ws.motion_jacobian[2, 3] = velocity * cosine * DT
        fill!(ws.control_jacobian, 0.0)
        ws.control_jacobian[1, 1] = cosine * DT
        ws.control_jacobian[2, 1] = sine * DT
        ws.control_jacobian[3, 2] = DT
        ws.predicted_state[1] = ws.state[1, instance] + velocity * cosine * DT
        ws.predicted_state[2] = ws.state[2, instance] + velocity * sine * DT
        ws.predicted_state[3] = ws.state[3, instance] + angular_velocity * DT

        covariance = @view ws.covariance[:, :, instance]
        matmul!(ws.gp, ws.motion_jacobian, covariance)
        matmul_right_transpose!(ws.predicted_covariance, ws.gp, ws.motion_jacobian)
        matmul!(ws.vq, ws.control_jacobian, PROCESS_COVARIANCE)
        matmul_right_transpose!(ws.process_covariance, ws.vq, ws.control_jacobian)
        ws.predicted_covariance .+= ws.process_covariance

        delta_x = LANDMARK_X - ws.predicted_state[1]
        delta_y = LANDMARK_Y - ws.predicted_state[2]
        q = delta_x * delta_x + delta_y * delta_y
        q > 1.0e-12 || error("landmark distance")
        distance = sqrt(q)
        predicted_bearing = atan(delta_y, delta_x) - ws.predicted_state[3]
        ws.measurement_jacobian[1, 1] = -delta_x / distance
        ws.measurement_jacobian[2, 1] = delta_y / q
        ws.measurement_jacobian[1, 2] = -delta_y / distance
        ws.measurement_jacobian[2, 2] = -delta_x / q
        ws.measurement_jacobian[1, 3] = 0.0
        ws.measurement_jacobian[2, 3] = -1.0
        matmul!(ws.hp, ws.measurement_jacobian, ws.predicted_covariance)
        matmul_right_transpose!(ws.innovation_covariance, ws.hp, ws.measurement_jacobian)
        ws.innovation_covariance .+= MEASUREMENT_COVARIANCE
        determinant = ws.innovation_covariance[1, 1] * ws.innovation_covariance[2, 2] -
            ws.innovation_covariance[1, 2] * ws.innovation_covariance[2, 1]
        abs(determinant) > 1.0e-12 || error("innovation determinant")
        ws.inverse_innovation[1, 1] = ws.innovation_covariance[2, 2] / determinant
        ws.inverse_innovation[2, 1] = -ws.innovation_covariance[2, 1] / determinant
        ws.inverse_innovation[1, 2] = -ws.innovation_covariance[1, 2] / determinant
        ws.inverse_innovation[2, 2] = ws.innovation_covariance[1, 1] / determinant
        matmul_right_transpose!(ws.pht, ws.predicted_covariance, ws.measurement_jacobian)
        matmul!(ws.gain, ws.pht, ws.inverse_innovation)
        innovation_range = measured_range - distance
        innovation_bearing = measured_bearing - predicted_bearing
        for row in 1:3
            ws.correction[row] = ws.gain[row, 1] * innovation_range +
                ws.gain[row, 2] * innovation_bearing
            ws.state[row, instance] = ws.predicted_state[row] + ws.correction[row]
        end

        matmul!(ws.kh, ws.gain, ws.measurement_jacobian)
        for column in 1:3, row in 1:3
            ws.joseph_a[row, column] = (row == column ? 1.0 : 0.0) - ws.kh[row, column]
        end
        matmul!(ws.ap, ws.joseph_a, ws.predicted_covariance)
        matmul_right_transpose!(ws.corrected_covariance, ws.ap, ws.joseph_a)
        matmul!(ws.kr, ws.gain, MEASUREMENT_COVARIANCE)
        matmul_right_transpose!(ws.measurement_covariance, ws.kr, ws.gain)
        ws.corrected_covariance .+= ws.measurement_covariance
        for column in 1:3, row in 1:(column - 1)
            symmetric = 0.5 * (ws.corrected_covariance[row, column] +
                ws.corrected_covariance[column, row])
            ws.corrected_covariance[row, column] = symmetric
            ws.corrected_covariance[column, row] = symmetric
        end
        for column in 1:3, row in 1:3
            covariance[row, column] = ws.corrected_covariance[row, column]
        end
        for row in 1:3
            isfinite(ws.state[row, instance]) || error("non-finite state")
        end
        for column in 1:3, row in 1:3
            isfinite(covariance[row, column]) || error("non-finite covariance")
        end
        for diagonal in 1:3
            covariance[diagonal, diagonal] > 0.0 || error("covariance diagonal")
        end
        symmetry_error = 0.0
        for column in 1:3, row in 1:3
            symmetry_error = max(
                symmetry_error,
                abs(covariance[row, column] - covariance[column, row]),
            )
        end
        symmetry_error <= 1.0e-10 || error("covariance symmetry")
    end
end

function run_episode!(ws::Workspace)
    @inbounds for turn in 1:EPISODE_LENGTH, instance in 1:ws.instances
        step!(ws, instance, turn)
    end
end

function quantize(value::Float64)
    scaled = value / QUANTIZATION
    scaled >= 0.0 ? floor(Int64, scaled + 0.5) : ceil(Int64, scaled - 0.5)
end

function update_i64_le!(context, value::Int64, buffer::Vector{UInt8})
    bits = reinterpret(UInt64, value)
    @inbounds for index in 1:8
        buffer[index] = UInt8((bits >> (8 * (index - 1))) & 0xff)
    end
    SHA.update!(context, buffer)
end

function validate!()
    ws = Workspace(1)
    context = SHA.SHA256_CTX()
    buffer = zeros(UInt8, 8)
    for turn in 1:EPISODE_LENGTH
        step!(ws, 1, turn)
        for value in @view ws.state[:, 1]
            update_i64_le!(context, quantize(value), buffer)
        end
        for value in @view ws.covariance[:, :, 1]
            update_i64_le!(context, quantize(value), buffer)
        end
    end
    actual_hash = bytes2hex(SHA.digest!(context))
    actual_hash == JULIA_HASH || error("Julia trajectory hash mismatch: $actual_hash")
    maximum_error = max(
        maximum(abs.(ws.state[:, 1] .- EXPECTED_STATE)),
        maximum(abs.(ws.covariance[:, :, 1] .- EXPECTED_COVARIANCE)),
    )
    maximum_error <= 1.0e-10 || error("final-state error: $maximum_error")
    actual_hash
end

function benchmark(instances::Int, samples::Int; timeline::Bool=false)
    ws = Workspace(instances)
    reset!(ws)
    run_episode!(ws)
    durations = Float64[]
    allocated = Int[]
    gc_seconds = Float64[]
    for _ in 1:samples
        reset!(ws)
        result = @timed run_episode!(ws)
        push!(durations, result.time)
        push!(allocated, result.bytes)
        push!(gc_seconds, result.gctime)
    end
    if timeline
        for sample in eachindex(durations)
            println(
                "{\"lane\":\"julia-persistent\",\"sample\":$(sample - 1)," *
                "\"turns\":$EPISODE_LENGTH,\"elapsed_ns\":$(round(Int, durations[sample] * 1.0e9))," *
                "\"gc_ns\":$(round(Int, gc_seconds[sample] * 1.0e9))," *
                "\"allocated_bytes\":$(allocated[sample])}"
            )
        end
        return
    end
    order = sortperm(durations)
    middle = order[(samples + 1) ÷ 2]
    println(join((
        "julia-persistent", instances, samples,
        durations[middle] * 1000.0,
        minimum(durations) * 1000.0,
        maximum(durations) * 1000.0,
        allocated[middle], gc_seconds[middle] * 1000.0,
    ), ','))
end

function main(arguments)
    samples = 9
    instances = collect(SCALED_INSTANCES)
    timeline = false
    index = 1
    while index <= length(arguments)
        if arguments[index] == "--samples"
            index += 1
            samples = parse(Int, arguments[index])
        elseif arguments[index] == "--instances"
            index += 1
            instances = [parse(Int, arguments[index])]
        elseif arguments[index] == "--timeline"
            timeline = true
        elseif arguments[index] == "--self-test"
            println("julia_hash=$(validate!()) reference_hash=$REFERENCE_HASH")
            return
        else
            error("unknown argument: $(arguments[index])")
        end
        index += 1
    end
    samples > 0 || error("samples must be positive")
    timeline || println("runtime,instances,samples,median_ms,min_ms,max_ms,allocated_bytes,gc_ms")
    diagnostic_hash = validate!()
    for count in instances
        benchmark(count, samples; timeline=timeline)
    end
    println(stderr, "validated trajectory $diagnostic_hash (reference $REFERENCE_HASH) on Julia $(VERSION), threads=$(Threads.nthreads())")
end

main(ARGS)
