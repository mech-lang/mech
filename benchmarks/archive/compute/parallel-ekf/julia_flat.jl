# Fixed-shape Julia control with flat column-major buffers.  This is the
# closest Julia equivalent to the optimized Rust control: no Matrix objects,
# views, BLAS calls, or per-lane heap allocations are used in the hot path.
const DT = 0.1f0
const R = 0.25f0
const VALIDATE = length(ARGS) > 2 && lowercase(ARGS[3]) == "checked"
const SYMMETRY_TOLERANCE = 0.0001f0

@inline function valid_candidate(x1::Float32, x2::Float32, x3::Float32,
                                 covariance::NTuple{9,Float32})
    isfinite(x1) && isfinite(x2) && isfinite(x3) &&
    isfinite(covariance[1]) && isfinite(covariance[2]) && isfinite(covariance[3]) &&
    isfinite(covariance[4]) && isfinite(covariance[5]) && isfinite(covariance[6]) &&
    isfinite(covariance[7]) && isfinite(covariance[8]) && isfinite(covariance[9]) &&
    covariance[1] > 0.0f0 && covariance[5] > 0.0f0 && covariance[9] > 0.0f0 &&
    abs(covariance[4] - covariance[2]) <= SYMMETRY_TOLERANCE &&
    abs(covariance[7] - covariance[3]) <= SYMMETRY_TOLERANCE &&
    abs(covariance[8] - covariance[6]) <= SYMMETRY_TOLERANCE
end

@inline function mm33(a::NTuple{9,Float32}, b::NTuple{9,Float32})
    (a[1]*b[1] + a[4]*b[2] + a[7]*b[3],
     a[2]*b[1] + a[5]*b[2] + a[8]*b[3],
     a[3]*b[1] + a[6]*b[2] + a[9]*b[3],
     a[1]*b[4] + a[4]*b[5] + a[7]*b[6],
     a[2]*b[4] + a[5]*b[5] + a[8]*b[6],
     a[3]*b[4] + a[6]*b[5] + a[9]*b[6],
     a[1]*b[7] + a[4]*b[8] + a[7]*b[9],
     a[2]*b[7] + a[5]*b[8] + a[8]*b[9],
     a[3]*b[7] + a[6]*b[8] + a[9]*b[9])
end

@inline function mm32(a::NTuple{6,Float32}, b::NTuple{4,Float32})
    (a[1]*b[1] + a[4]*b[2],
     a[2]*b[1] + a[5]*b[2],
     a[3]*b[1] + a[6]*b[2],
     a[1]*b[3] + a[4]*b[4],
     a[2]*b[3] + a[5]*b[4],
     a[3]*b[3] + a[6]*b[4])
end

@inline function mm32x23(a::NTuple{6,Float32}, b::NTuple{6,Float32})
    (a[1]*b[1] + a[4]*b[2],
     a[2]*b[1] + a[5]*b[2],
     a[3]*b[1] + a[6]*b[2],
     a[1]*b[3] + a[4]*b[4],
     a[2]*b[3] + a[5]*b[4],
     a[3]*b[3] + a[6]*b[4],
     a[1]*b[5] + a[4]*b[6],
     a[2]*b[5] + a[5]*b[6],
     a[3]*b[5] + a[6]*b[6])
end

@inline function transpose33(a::NTuple{9,Float32})
    (a[1], a[4], a[7], a[2], a[5], a[8], a[3], a[6], a[9])
end

@inline function transpose32(a::NTuple{6,Float32})
    (a[1], a[4], a[2], a[5], a[3], a[6])
end

@inline function step!(state::Vector{Float32}, covariance::Vector{Float32}, lane::Int,
                       velocity::Vector{Float32}, angular_velocity::Vector{Float32},
                       bearing::Vector{Float32})
    @inbounds begin
        state_offset = (lane - 1) * 3
        covariance_offset = (lane - 1) * 9
        theta = state[state_offset + 3]
        sin_theta, cos_theta = sincos(theta)
        distance = velocity[lane] * DT
        predicted_state_1 = state[state_offset + 1] + distance * cos_theta
        predicted_state_2 = state[state_offset + 2] + distance * sin_theta
        predicted_state_3 = theta + angular_velocity[lane] * DT

        f = (1.0f0, 0.0f0, 0.0f0,
             0.0f0, 1.0f0, 0.0f0,
             -distance * sin_theta, distance * cos_theta, 1.0f0)
        ft = transpose33(f)
        g = (cos_theta * DT, sin_theta * DT, 0.0f0,
             0.0f0, 0.0f0, DT)
        gt = transpose32(g)
        p = (covariance[covariance_offset + 1], covariance[covariance_offset + 2], covariance[covariance_offset + 3],
             covariance[covariance_offset + 4], covariance[covariance_offset + 5], covariance[covariance_offset + 6],
             covariance[covariance_offset + 7], covariance[covariance_offset + 8], covariance[covariance_offset + 9])
        predicted_p = mm33(mm33(f, p), ft)
        process_p = mm32x23(mm32(g, (0.01f0, 0.0f0, 0.0f0, 0.0025f0)), gt)
        predicted_p = (predicted_p[1] + process_p[1], predicted_p[2] + process_p[2], predicted_p[3] + process_p[3],
                       predicted_p[4] + process_p[4], predicted_p[5] + process_p[5], predicted_p[6] + process_p[6],
                       predicted_p[7] + process_p[7], predicted_p[8] + process_p[8], predicted_p[9] + process_p[9])

        delta_x = 140.0f0 - predicted_state_1
        delta_y = 12.0f0 - predicted_state_2
        squared_range = delta_x * delta_x + delta_y * delta_y
        predicted_bearing = atan(delta_y, delta_x) - predicted_state_3
        raw_innovation = bearing[lane] - predicted_bearing
        innovation = atan(sin(raw_innovation), cos(raw_innovation))
        h0 = delta_y / squared_range
        h1 = -delta_x / squared_range
        h2 = -1.0f0
        pht0 = predicted_p[1]*h0 + predicted_p[4]*h1 + predicted_p[7]*h2
        pht1 = predicted_p[2]*h0 + predicted_p[5]*h1 + predicted_p[8]*h2
        pht2 = predicted_p[3]*h0 + predicted_p[6]*h1 + predicted_p[9]*h2
        variance = h0*pht0 + h1*pht1 + h2*pht2 + R
        k0 = pht0 / variance
        k1 = pht1 / variance
        k2 = pht2 / variance
        a = (1.0f0-k0*h0, -k1*h0, -k2*h0,
             -k0*h1, 1.0f0-k1*h1, -k2*h1,
             -k0*h2, -k1*h2, 1.0f0-k2*h2)
        corrected_p = mm33(mm33(a, predicted_p), transpose33(a))
        candidate_covariance = (corrected_p[1] + k0*k0*R,
                                corrected_p[2] + k1*k0*R,
                                corrected_p[3] + k2*k0*R,
                                corrected_p[4] + k0*k1*R,
                                corrected_p[5] + k1*k1*R,
                                corrected_p[6] + k2*k1*R,
                                corrected_p[7] + k0*k2*R,
                                corrected_p[8] + k1*k2*R,
                                corrected_p[9] + k2*k2*R)
        candidate_state_1 = predicted_state_1 + k0*innovation
        candidate_state_2 = predicted_state_2 + k1*innovation
        candidate_state_3 = predicted_state_3 + k2*innovation
        if !VALIDATE || valid_candidate(candidate_state_1, candidate_state_2, candidate_state_3,
                                        candidate_covariance)
            state[state_offset + 1] = candidate_state_1
            state[state_offset + 2] = candidate_state_2
            state[state_offset + 3] = candidate_state_3
            covariance[covariance_offset + 1] = candidate_covariance[1]
            covariance[covariance_offset + 2] = candidate_covariance[2]
            covariance[covariance_offset + 3] = candidate_covariance[3]
            covariance[covariance_offset + 4] = candidate_covariance[4]
            covariance[covariance_offset + 5] = candidate_covariance[5]
            covariance[covariance_offset + 6] = candidate_covariance[6]
            covariance[covariance_offset + 7] = candidate_covariance[7]
            covariance[covariance_offset + 8] = candidate_covariance[8]
            covariance[covariance_offset + 9] = candidate_covariance[9]
            return true
        end
    end
    false
end

@inline function dispatch!(state, covariance, velocity, angular_velocity, bearing, turns)
    if VALIDATE
        faults = 0
        @inbounds for _ in 1:turns
            for lane in eachindex(velocity)
                faults += !step!(state, covariance, lane, velocity, angular_velocity, bearing)
            end
        end
        return faults
    else
        @inbounds for _ in 1:turns
            @simd for lane in eachindex(velocity)
                step!(state, covariance, lane, velocity, angular_velocity, bearing)
            end
        end
        return 0
    end
end

instances = max(1, length(ARGS) > 0 ? parse(Int, ARGS[1]) : 10_000)
turns = max(1, length(ARGS) > 1 ? parse(Int, ARGS[2]) : 5)
phase = Float32(2pi) .* Float32.(0:instances-1) ./ Float32(instances)
velocity = 1.0f0 .+ 0.05f0 .* sin.(phase .* 3.0f0)
angular_velocity = 0.015f0 .* (1.0f0 .+ 0.1f0 .* sin.(phase .* 2.0f0))
bearing = -0.55f0 .+ 0.01f0 .* sin.(phase .* 7.0f0) .+ 0.005f0 .* sin.(phase .* 11.0f0)
state = repeat(Float32[55, 25, 0.4], instances)
covariance = repeat(Float32[100, 0, 0, 0, 100, 0, 0, 0, 0.15], instances)
dispatch!(state, covariance, velocity, angular_velocity, bearing, 5)
state .= repeat(Float32[55, 25, 0.4], instances)
covariance .= repeat(Float32[100, 0, 0, 0, 100, 0, 0, 0, 0.15], instances)
started = time_ns()
faults = dispatch!(state, covariance, velocity, angular_velocity, bearing, turns)
elapsed = (time_ns() - started) / 1e9
println("lane: Julia fixed-shape flat tuples")
println("instances: ", instances)
println("turns: ", turns)
println("elapsed_s: ", elapsed)
println("throughput: ", instances * turns / elapsed)
println("checksum: ", sum(Float64, state) + sum(Float64, covariance))
println("validation: ", VALIDATE ? "checked" : "unchecked")
println("faults: ", faults)
