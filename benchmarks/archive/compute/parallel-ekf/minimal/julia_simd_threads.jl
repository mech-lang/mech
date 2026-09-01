using SIMD
const V4 = Vec{4,Float32}
const DT = 0.1f0
const R = 0.25f0
const SYMMETRY_TOLERANCE = 0.0001f0
const VALIDATE = length(ARGS) > 2 && lowercase(ARGS[3]) == "checked"
const FUSED = length(ARGS) > 3 && lowercase(ARGS[4]) in ("fused", "batched", "unchecked-batched")
@inline function mm33(a::NTuple{9,T}, b::NTuple{9,T}) where {T}
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
@inline function mm32(a::NTuple{6,T}, b::NTuple{4,T}) where {T}
    (a[1]*b[1] + a[4]*b[2],
     a[2]*b[1] + a[5]*b[2],
     a[3]*b[1] + a[6]*b[2],
     a[1]*b[3] + a[4]*b[4],
     a[2]*b[3] + a[5]*b[4],
     a[3]*b[3] + a[6]*b[4])
end
@inline function mm32x23(a::NTuple{6,T}, b::NTuple{6,T}) where {T}
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
@inline function transpose33(a::NTuple{9,T}) where {T}
    (a[1], a[4], a[7], a[2], a[5], a[8], a[3], a[6], a[9])
end
@inline function transpose32(a::NTuple{6,T}) where {T}
    (a[1], a[4], a[2], a[5], a[3], a[6])
end
@inline function atan2_vec(y::V4, x::V4)
    V4((atan(y[1], x[1]), atan(y[2], x[2]), atan(y[3], x[3]), atan(y[4], x[4])))
end
@inline function valid_candidate(x1::V4, x2::V4, x3::V4, p::NTuple{9,V4})
    isfinite(x1) & isfinite(x2) & isfinite(x3) &
    isfinite(p[1]) & isfinite(p[2]) & isfinite(p[3]) &
    isfinite(p[4]) & isfinite(p[5]) & isfinite(p[6]) &
    isfinite(p[7]) & isfinite(p[8]) & isfinite(p[9]) &
    (p[1] > 0.0f0) & (p[5] > 0.0f0) & (p[9] > 0.0f0) &
    (abs(p[4]  - p[2]) <= SYMMETRY_TOLERANCE) &
    (abs(p[7]  - p[3]) <= SYMMETRY_TOLERANCE) &
    (abs(p[8]  - p[6]) <= SYMMETRY_TOLERANCE)
end
@inline function step_group!(x1::Vector{V4}, x2::Vector{V4}, x3::Vector{V4},
                             covariance::NTuple{9,Vector{V4}}, group::Int,
                             velocity::Vector{V4}, angular_velocity::Vector{V4},
                             bearing::Vector{V4})
    theta = x3[group]
    sin_theta = sin(theta)
    cos_theta = cos(theta)
    distance = velocity[group] * DT
    predicted_state_1 = x1[group] + distance * cos_theta
    predicted_state_2 = x2[group] + distance * sin_theta
    predicted_state_3 = theta + angular_velocity[group] * DT
    f = (V4(1), V4(0), V4(0),
         V4(0), V4(1), V4(0),
          -distance * sin_theta, distance * cos_theta, V4(1))
    ft = transpose33(f)
    g = (cos_theta * DT, sin_theta * DT, V4(0), V4(0), V4(0), V4(DT))
    gt = transpose32(g)
    p = ntuple(i -> covariance[i][group], 9)
    predicted_p0 = mm33(mm33(f, p), ft)
    process_p = mm32x23(mm32(g, (V4(0.01f0), V4(0), V4(0), V4(0.0025f0))), gt)
    predicted_p = ntuple(i -> predicted_p0[i] + process_p[i], 9)
    delta_x = V4(140)  - predicted_state_1
    delta_y = V4(12)  - predicted_state_2
    squared_range = delta_x * delta_x + delta_y * delta_y
    predicted_bearing = atan2_vec(delta_y, delta_x)  - predicted_state_3
    raw_innovation = bearing[group]  - predicted_bearing
    innovation = atan2_vec(sin(raw_innovation), cos(raw_innovation))
    h0 = delta_y / squared_range
    h1 =  -delta_x / squared_range
    h2 = V4(-1)
    pht0 = predicted_p[1]*h0 + predicted_p[4]*h1 + predicted_p[7]*h2
    pht1 = predicted_p[2]*h0 + predicted_p[5]*h1 + predicted_p[8]*h2
    pht2 = predicted_p[3]*h0 + predicted_p[6]*h1 + predicted_p[9]*h2
    variance = h0*pht0 + h1*pht1 + h2*pht2 + R
    k0 = pht0 / variance
    k1 = pht1 / variance
    k2 = pht2 / variance
    a = (V4(1)  - k0*h0,  -k1*h0,  -k2*h0,
          -k0*h1, V4(1)  - k1*h1,  -k2*h1,
          -k0*h2,  -k1*h2, V4(1)  - k2*h2)
    corrected_p = mm33(mm33(a, predicted_p), transpose33(a))
    candidate_p = (corrected_p[1] + k0*k0*R,
                   corrected_p[2] + k1*k0*R,
                   corrected_p[3] + k2*k0*R,
                   corrected_p[4] + k0*k1*R,
                   corrected_p[5] + k1*k1*R,
                   corrected_p[6] + k2*k1*R,
                   corrected_p[7] + k0*k2*R,
                   corrected_p[8] + k1*k2*R,
                   corrected_p[9] + k2*k2*R)
    candidate_x1 = predicted_state_1 + k0*innovation
    candidate_x2 = predicted_state_2 + k1*innovation
    candidate_x3 = predicted_state_3 + k2*innovation
    if VALIDATE
        valid = valid_candidate(candidate_x1, candidate_x2, candidate_x3, candidate_p)
        if valid[1] & valid[2] & valid[3] & valid[4]
            x1[group] = candidate_x1
            x2[group] = candidate_x2
            x3[group] = candidate_x3
            for i in 1:9
                covariance[i][group] = candidate_p[i]
            end
            return 0
        end
        old_p = p
        x1[group] = vifelse(valid, candidate_x1, x1[group])
        x2[group] = vifelse(valid, candidate_x2, x2[group])
        x3[group] = vifelse(valid, candidate_x3, x3[group])
        for i in 1:9
            covariance[i][group] = vifelse(valid, candidate_p[i], old_p[i])
        end
        return Int(!valid[1]) + Int(!valid[2]) + Int(!valid[3]) + Int(!valid[4])
    else
        x1[group] = candidate_x1
        x2[group] = candidate_x2
        x3[group] = candidate_x3
        for i in 1:9
            covariance[i][group] = candidate_p[i]
        end
        return 0
    end
end
@inline function pack4(values::Vector{Float32})
    n = length(values)
    n % 4 == 0 || error("Julia fixed-shape SIMD requires an instance count divisible by four")
    [V4((values[i], values[i+1], values[i+2], values[i+3])) for i in 1:4:n]
end
function checksum(values::Vector{V4})
    total = 0.0
    @inbounds for value in values
        total += Float64(value[1]) + Float64(value[2]) + Float64(value[3]) + Float64(value[4])
    end
    total
end
function dispatch!(x1, x2, x3, covariance, velocity, angular_velocity, bearing, turns)
    if Threads.nthreads() == 1
        faults = 0
        @inbounds for _ in 1:turns
            if VALIDATE
                for group in eachindex(velocity)
                    faults += step_group!(x1, x2, x3, covariance, group, velocity, angular_velocity, bearing)
                end
            else
                @simd for group in eachindex(velocity)
                    step_group!(x1, x2, x3, covariance, group, velocity, angular_velocity, bearing)
                end
            end
        end
        return faults
    end

    # Independent groups run in parallel; each turn still synchronizes before
    # the next state publication, matching Mech's worker runtime.
    faults_by_thread = zeros(Int, Threads.maxthreadid())
    @inbounds for _ in 1:turns
        Threads.@threads :static for group in eachindex(velocity)
            if VALIDATE
                faults_by_thread[Threads.threadid()] +=
                    step_group!(x1, x2, x3, covariance, group, velocity, angular_velocity, bearing)
            else
                step_group!(x1, x2, x3, covariance, group, velocity, angular_velocity, bearing)
            end
        end
    end
    sum(faults_by_thread)
end

function dispatch_fused!(x1, x2, x3, covariance, velocity, angular_velocity, bearing, turns)
    # Each worker owns a disjoint set of independent filters and keeps its
    # turn loop local. This removes the per-turn thread barrier while keeping
    # validation and rollback inside each filter when requested.
    faults_by_thread = zeros(Int, Threads.maxthreadid())
    if Threads.nthreads() == 1
        faults = 0
        @inbounds for group in eachindex(velocity)
            for _ in 1:turns
                faults += step_group!(x1, x2, x3, covariance, group, velocity, angular_velocity, bearing)
            end
        end
        return faults
    end
    @inbounds Threads.@threads :static for group in eachindex(velocity)
        local_faults = 0
        for _ in 1:turns
            if VALIDATE
                local_faults += step_group!(x1, x2, x3, covariance, group, velocity, angular_velocity, bearing)
            else
                step_group!(x1, x2, x3, covariance, group, velocity, angular_velocity, bearing)
            end
        end
        faults_by_thread[Threads.threadid()] += local_faults
    end
    sum(faults_by_thread)
end

dispatch_mode!(args...) = FUSED ? dispatch_fused!(args...) : dispatch!(args...)
instances = max(1, length(ARGS) > 0 ? parse(Int, ARGS[1]) : 10_000)
turns = max(1, length(ARGS) > 1 ? parse(Int, ARGS[2]) : 5)
instances % 4 == 0 || error("instance count must be divisible by four")
phase = Float32(2pi) .* Float32.(0:instances-1) ./ Float32(instances)
velocity = pack4(1.0f0 .+ 0.05f0 .* sin.(phase .* 3.0f0))
angular_velocity = pack4(0.015f0 .* (1.0f0 .+ 0.1f0 .* sin.(phase .* 2.0f0)))
bearing = pack4(-0.55f0 .+ 0.01f0 .* sin.(phase .* 7.0f0) .+ 0.005f0 .* sin.(phase .* 11.0f0))
groups = length(velocity)
x1 = fill(V4(55), groups)
x2 = fill(V4(25), groups)
x3 = fill(V4(0.4f0), groups)
covariance = (fill(V4(100), groups), fill(V4(0), groups), fill(V4(0), groups),
             fill(V4(0), groups), fill(V4(100), groups), fill(V4(0), groups),
             fill(V4(0), groups), fill(V4(0), groups), fill(V4(0.15f0), groups))
dispatch_mode!(x1, x2, x3, covariance, velocity, angular_velocity, bearing, 5)
fill!(x1, V4(55)); fill!(x2, V4(25)); fill!(x3, V4(0.4f0))
for i in eachindex(covariance)
    fill!(covariance[i], (i == 1 || i == 5) ? V4(100) : (i == 9 ? V4(0.15f0) : V4(0)))
end
started = time_ns()
faults = dispatch_mode!(x1, x2, x3, covariance, velocity, angular_velocity, bearing, turns)
elapsed = (time_ns() - started) / 1e9
println("lane: Julia fixed-shape SIMD (SIMD.jl Vec V4)")
println("instances: ", instances)
println("turns: ", turns)
println("threads: ", Threads.nthreads())
println("elapsed_s: ", elapsed)
println("throughput: ", instances * turns / elapsed)
println("checksum: ", checksum(x1) + checksum(x2) + checksum(x3) + sum(checksum, covariance))
println("validation: ", VALIDATE ? "checked" : "unchecked")
println("synchronization: ", FUSED ? "once after fused block" : "per-turn")
println("faults: ", faults)
