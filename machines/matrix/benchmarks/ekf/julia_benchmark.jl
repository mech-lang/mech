#!/usr/bin/env julia

using LinearAlgebra
using Statistics

const SAMPLE_COUNT = 9
const INPUT_PERIOD = 4_096
const TARGET_SAMPLE_SECONDS = 0.075
const WARMUP_SECONDS = 0.25
const DT = 0.1
const LANDMARK_X = 140.0
const LANDMARK_Y = 12.0
const MEASUREMENT_NOISE = 0.25

wrap_angle(angle) = atan(sin(angle), cos(angle))

function input_samples()
    truth = [45.0, 15.0, 0.0]
    samples = Vector{NTuple{3,Float64}}(undef, INPUT_PERIOD)
    base_angular_velocity = 2pi / (INPUT_PERIOD * DT)
    for index in 0:INPUT_PERIOD-1
        phase = 2pi * index / INPUT_PERIOD
        linear_velocity = 1.0 + 0.05 * sin(phase * 3.0)
        angular_velocity = base_angular_velocity * (1.0 + 0.1 * cos(phase * 2.0))
        truth[1] += linear_velocity * cos(truth[3]) * DT
        truth[2] += linear_velocity * sin(truth[3]) * DT
        truth[3] = wrap_angle(truth[3] + angular_velocity * DT)
        noise = 0.01 * sin(phase * 7.0) + 0.005 * cos(phase * 11.0)
        bearing = wrap_angle(atan(LANDMARK_Y - truth[2], LANDMARK_X - truth[1]) - truth[3] + noise)
        samples[index + 1] = (linear_velocity, angular_velocity, bearing)
    end
    return samples
end

mutable struct Ekf
    state::Vector{Float64}
    covariance::Matrix{Float64}
end

Ekf() = Ekf([55.0, 25.0, 0.4], Diagonal([100.0, 100.0, 0.15]) |> Matrix)

function turn!(filter::Ekf, sample)
    linear_velocity, angular_velocity, bearing = sample
    theta = filter.state[3]
    sin_theta, cos_theta = sincos(theta)
    distance = linear_velocity * DT
    predicted_state = filter.state + [distance * cos_theta, distance * sin_theta, angular_velocity * DT]
    motion_jacobian = [
        1.0 0.0 -distance * sin_theta
        0.0 1.0  distance * cos_theta
        0.0 0.0  1.0
    ]
    control_jacobian = [cos_theta * DT 0.0; sin_theta * DT 0.0; 0.0 DT]
    process_noise = Diagonal([0.01, 0.0025])
    predicted_covariance = motion_jacobian * filter.covariance * transpose(motion_jacobian) +
                           control_jacobian * process_noise * transpose(control_jacobian)

    delta_x = LANDMARK_X - predicted_state[1]
    delta_y = LANDMARK_Y - predicted_state[2]
    squared_range = delta_x * delta_x + delta_y * delta_y
    predicted_bearing = atan(delta_y, delta_x) - predicted_state[3]
    innovation = wrap_angle(bearing - predicted_bearing)
    observation_jacobian = reshape([delta_y / squared_range, -delta_x / squared_range, -1.0], 1, 3)
    innovation_variance = (observation_jacobian * predicted_covariance * transpose(observation_jacobian))[1] + MEASUREMENT_NOISE
    gain = predicted_covariance * transpose(observation_jacobian) / innovation_variance

    filter.state = predicted_state + vec(gain) * innovation
    correction = Matrix{Float64}(I, 3, 3) - gain * observation_jacobian
    filter.covariance = correction * predicted_covariance * transpose(correction) +
                        gain * transpose(gain) * MEASUREMENT_NOISE
end

check(filter) = sum(filter.state) + tr(filter.covariance)

function validate(samples)
    filter = Ekf()
    for index in 1:256
        turn!(filter, samples[index])
    end
    expected_state = [80.35682278971947, 24.769631959523576, 0.34224350117788876]
    expected_covariance = [
        100.24018212103414 -2.34499352078198 -0.23555533772422113
        -2.3449935207819896 46.240533034085935 -0.716187029947414
        -0.23555533772422071 -0.7161870299474145 0.014716073850468109
    ]
    @assert maximum(abs.(filter.state - expected_state)) < 1.0e-9
    @assert maximum(abs.(filter.covariance - expected_covariance)) < 1.0e-9
end

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

samples = input_samples()
validate(samples)
filter = Ekf()
sample_index = Ref(1)
function operation()
    turn!(filter, samples[sample_index[]])
    sample_index[] = mod1(sample_index[] + 1, length(samples))
end
result = measure(operation)
@assert isfinite(check(filter))
median_ms, minimum_ms, maximum_ms, iterations = result
println("runtime,operation,median_ms,min_ms,max_ms,batch_iterations,check")
println("julia-loop,ekf,$(round(median_ms, digits=9)),$(round(minimum_ms, digits=9)),$(round(maximum_ms, digits=9)),$iterations,$(round(check(filter), digits=12))")
