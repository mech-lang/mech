#!/usr/bin/env julia

using StaticArrays

const ROOT = normpath(joinpath(@__DIR__, "..", "..", "..", ".."))
const TRACE_PATH = joinpath(ROOT, "benchmarks", "runtime", "gate-b", "ekf-input-v1.bin")
const EPISODE_LENGTH = 4096
const DT = 0.05
const LANDMARK_X = 25.0
const LANDMARK_Y = -10.0
const PROCESS_COVARIANCE = @SMatrix [0.04 0.0; 0.0 0.0025]
const MEASUREMENT_COVARIANCE = @SMatrix [0.25 0.0; 0.0 0.0009]
const INITIAL_STATE = @SVector [2.0, 1.0, 0.15]
const INITIAL_COVARIANCE = @SMatrix [1.0 0.0 0.0; 0.0 1.0 0.0; 0.0 0.0 0.05]
const EXPECTED_STATE = @SVector [18.169827258925427, 4.339708695271022, 0.2557219366745068]
const EXPECTED_COVARIANCE = @SMatrix [
    0.3270953723043491 0.1509754472729972 -0.022618166436367253;
    0.1509754472729972 0.07105284175378412 -0.010486015657880304;
    -0.022618166436367253 -0.010486015657880304 0.0016395600302299483
]

function load_trace()
    values = reinterpret(Float64, read(TRACE_PATH))
    length(values) == EPISODE_LENGTH * 4 || error("Gate B trace has the wrong length")
    reshape(copy(values), 4, EPISODE_LENGTH)
end

const TRACE = load_trace()

@inline function matmul!(output, left, right)
    @inbounds for column in axes(right, 2), row in axes(left, 1)
        total = 0.0
        for inner in axes(left, 2)
            total += left[row, inner] * right[inner, column]
        end
        output[row, column] = total
    end
end

@inline function matmul_right_transpose!(output, left, right)
    @inbounds for column in axes(right, 1), row in axes(left, 1)
        total = 0.0
        for inner in axes(left, 2)
            total += left[row, inner] * right[column, inner]
        end
        output[row, column] = total
    end
end

mutable struct Workspace
    state::MVector{3,Float64}
    covariance::MMatrix{3,3,Float64,9}
    predicted_state::MVector{3,Float64}
    motion_jacobian::MMatrix{3,3,Float64,9}
    control_jacobian::MMatrix{3,2,Float64,6}
    gp::MMatrix{3,3,Float64,9}
    predicted_covariance::MMatrix{3,3,Float64,9}
    vq::MMatrix{3,2,Float64,6}
    process_covariance::MMatrix{3,3,Float64,9}
    measurement_jacobian::MMatrix{2,3,Float64,6}
    hp::MMatrix{2,3,Float64,6}
    innovation_covariance::MMatrix{2,2,Float64,4}
    inverse_innovation::MMatrix{2,2,Float64,4}
    pht::MMatrix{3,2,Float64,6}
    gain::MMatrix{3,2,Float64,6}
    innovation::MVector{2,Float64}
    correction::MVector{3,Float64}
    kh::MMatrix{3,3,Float64,9}
    joseph_a::MMatrix{3,3,Float64,9}
    ap::MMatrix{3,3,Float64,9}
    corrected_covariance::MMatrix{3,3,Float64,9}
    kr::MMatrix{3,2,Float64,6}
    measurement_covariance::MMatrix{3,3,Float64,9}
end

function Workspace()
    Workspace(
        zeros(MVector{3,Float64}), zeros(MMatrix{3,3,Float64,9}),
        zeros(MVector{3,Float64}), zeros(MMatrix{3,3,Float64,9}),
        zeros(MMatrix{3,2,Float64,6}), zeros(MMatrix{3,3,Float64,9}),
        zeros(MMatrix{3,3,Float64,9}), zeros(MMatrix{3,2,Float64,6}),
        zeros(MMatrix{3,3,Float64,9}), zeros(MMatrix{2,3,Float64,6}),
        zeros(MMatrix{2,3,Float64,6}), zeros(MMatrix{2,2,Float64,4}),
        zeros(MMatrix{2,2,Float64,4}), zeros(MMatrix{3,2,Float64,6}),
        zeros(MMatrix{3,2,Float64,6}), zeros(MVector{2,Float64}),
        zeros(MVector{3,Float64}), zeros(MMatrix{3,3,Float64,9}),
        zeros(MMatrix{3,3,Float64,9}), zeros(MMatrix{3,3,Float64,9}),
        zeros(MMatrix{3,3,Float64,9}), zeros(MMatrix{3,2,Float64,6}),
        zeros(MMatrix{3,3,Float64,9}),
    )
end

function reset!(ws::Workspace)
    copyto!(ws.state, INITIAL_STATE)
    copyto!(ws.covariance, INITIAL_COVARIANCE)
end

@inline function step!(ws::Workspace, turn::Int)
    @inbounds begin
        velocity = TRACE[1, turn]
        angular_velocity = TRACE[2, turn]
        measured_range = TRACE[3, turn]
        measured_bearing = TRACE[4, turn]
        cosine = cos(ws.state[3])
        sine = sin(ws.state[3])
        g = ws.motion_jacobian
        g[1, 1], g[2, 1], g[3, 1] = 1.0, 0.0, 0.0
        g[1, 2], g[2, 2], g[3, 2] = 0.0, 1.0, 0.0
        g[1, 3], g[2, 3], g[3, 3] = -velocity * sine * DT, velocity * cosine * DT, 1.0
        v = ws.control_jacobian
        v[1, 1], v[2, 1], v[3, 1] = cosine * DT, sine * DT, 0.0
        v[1, 2], v[2, 2], v[3, 2] = 0.0, 0.0, DT
        ws.predicted_state[1] = ws.state[1] + velocity * cosine * DT
        ws.predicted_state[2] = ws.state[2] + velocity * sine * DT
        ws.predicted_state[3] = ws.state[3] + angular_velocity * DT

        matmul!(ws.gp, g, ws.covariance)
        matmul_right_transpose!(ws.predicted_covariance, ws.gp, g)
        matmul!(ws.vq, v, PROCESS_COVARIANCE)
        matmul_right_transpose!(ws.process_covariance, ws.vq, v)
        for index in eachindex(ws.predicted_covariance)
            ws.predicted_covariance[index] += ws.process_covariance[index]
        end

        delta_x = LANDMARK_X - ws.predicted_state[1]
        delta_y = LANDMARK_Y - ws.predicted_state[2]
        q = delta_x * delta_x + delta_y * delta_y
        q > 1.0e-12 || error("landmark distance")
        distance = sqrt(q)
        predicted_bearing = atan(delta_y, delta_x) - ws.predicted_state[3]
        h = ws.measurement_jacobian
        h[1, 1], h[2, 1] = -delta_x / distance, delta_y / q
        h[1, 2], h[2, 2] = -delta_y / distance, -delta_x / q
        h[1, 3], h[2, 3] = 0.0, -1.0
        matmul!(ws.hp, h, ws.predicted_covariance)
        matmul_right_transpose!(ws.innovation_covariance, ws.hp, h)
        for index in eachindex(ws.innovation_covariance)
            ws.innovation_covariance[index] += MEASUREMENT_COVARIANCE[index]
        end
        s = ws.innovation_covariance
        determinant = s[1, 1] * s[2, 2] - s[1, 2] * s[2, 1]
        abs(determinant) > 1.0e-12 || error("innovation determinant")
        inverse = ws.inverse_innovation
        inverse[1, 1], inverse[2, 1] = s[2, 2] / determinant, -s[2, 1] / determinant
        inverse[1, 2], inverse[2, 2] = -s[1, 2] / determinant, s[1, 1] / determinant
        matmul_right_transpose!(ws.pht, ws.predicted_covariance, h)
        matmul!(ws.gain, ws.pht, inverse)
        ws.innovation[1] = measured_range - distance
        ws.innovation[2] = measured_bearing - predicted_bearing
        matmul!(ws.correction, ws.gain, ws.innovation)
        for index in 1:3
            ws.state[index] = ws.predicted_state[index] + ws.correction[index]
        end

        matmul!(ws.kh, ws.gain, h)
        for column in 1:3, row in 1:3
            ws.joseph_a[row, column] = (row == column ? 1.0 : 0.0) - ws.kh[row, column]
        end
        matmul!(ws.ap, ws.joseph_a, ws.predicted_covariance)
        matmul_right_transpose!(ws.corrected_covariance, ws.ap, ws.joseph_a)
        matmul!(ws.kr, ws.gain, MEASUREMENT_COVARIANCE)
        matmul_right_transpose!(ws.measurement_covariance, ws.kr, ws.gain)
        for index in eachindex(ws.corrected_covariance)
            ws.corrected_covariance[index] += ws.measurement_covariance[index]
        end
        for column in 1:3, row in 1:(column - 1)
            symmetric = 0.5 * (ws.corrected_covariance[row, column] + ws.corrected_covariance[column, row])
            ws.corrected_covariance[row, column] = symmetric
            ws.corrected_covariance[column, row] = symmetric
        end
        copyto!(ws.covariance, ws.corrected_covariance)
        all(isfinite, ws.state) || error("non-finite state")
        all(isfinite, ws.covariance) || error("non-finite covariance")
        all(index -> ws.covariance[index, index] > 0.0, 1:3) || error("covariance diagonal")
    end
end

function run_episode!(ws::Workspace)
    @inbounds for turn in 1:EPISODE_LENGTH
        step!(ws, turn)
    end
end

function main(arguments)
    samples = 60
    index = 1
    while index <= length(arguments)
        if arguments[index] == "--samples"
            index += 1
            samples = parse(Int, arguments[index])
        else
            error("unknown argument: $(arguments[index])")
        end
        index += 1
    end
    samples > 0 || error("samples must be positive")
    ws = Workspace()
    reset!(ws)
    run_episode!(ws)
    maximum(abs, ws.state - EXPECTED_STATE) <= 1.0e-9 || error("fixed Julia state mismatch")
    maximum(abs, ws.covariance - EXPECTED_COVARIANCE) <= 1.0e-9 || error("fixed Julia covariance mismatch")
    for sample in 0:samples-1
        reset!(ws)
        result = @timed run_episode!(ws)
        println(
            "{\"lane\":\"julia-staticarrays\",\"sample\":$sample," *
            "\"turns\":$EPISODE_LENGTH,\"elapsed_ns\":$(round(Int, result.time * 1.0e9))," *
            "\"gc_ns\":$(round(Int, result.gctime * 1.0e9)),\"allocated_bytes\":$(result.bytes)}"
        )
    end
end

main(ARGS)
