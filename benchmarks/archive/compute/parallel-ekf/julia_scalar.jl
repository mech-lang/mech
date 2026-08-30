using LinearAlgebra

BLAS.set_num_threads(1)
const VALIDATE = length(ARGS) > 2 && lowercase(ARGS[3]) == "checked"
const SYMMETRY_TOLERANCE = 0.0001f0

mutable struct Scratch
    f::Matrix{Float32}; g::Matrix{Float32}; left::Matrix{Float32}
    predicted_p::Matrix{Float32}; process_left::Matrix{Float32}; process_p::Matrix{Float32}
    pht::Vector{Float32}; a::Matrix{Float32}; ap::Matrix{Float32}; corrected_p::Matrix{Float32}
end

Scratch() = Scratch(zeros(Float32,3,3), zeros(Float32,3,2), zeros(Float32,3,3),
    zeros(Float32,3,3), zeros(Float32,3,2), zeros(Float32,3,3), zeros(Float32,3),
    zeros(Float32,3,3), zeros(Float32,3,3), zeros(Float32,3,3))
const Q = Float32[0.01 0; 0 0.0025]

@inline function valid_candidate(x0::Float32, x1::Float32, x2::Float32, p::Matrix{Float32})
    isfinite(x0) && isfinite(x1) && isfinite(x2) &&
    isfinite(p[1,1]) && isfinite(p[2,1]) && isfinite(p[3,1]) &&
    isfinite(p[1,2]) && isfinite(p[2,2]) && isfinite(p[3,2]) &&
    isfinite(p[1,3]) && isfinite(p[2,3]) && isfinite(p[3,3]) &&
    p[1,1] > 0.0f0 && p[2,2] > 0.0f0 && p[3,3] > 0.0f0 &&
    abs(p[1,2] - p[2,1]) <= SYMMETRY_TOLERANCE &&
    abs(p[1,3] - p[3,1]) <= SYMMETRY_TOLERANCE &&
    abs(p[2,3] - p[3,2]) <= SYMMETRY_TOLERANCE
end

Base.@inline function step!(state, covariance, lane, velocity, angular_velocity, bearing, s)
    dt = 0.1f0
    st = state[3,lane]
    sin_theta, cos_theta = sincos(st)
    distance = velocity[lane] * dt
    x0 = state[1,lane] + distance * cos_theta
    x1 = state[2,lane] + distance * sin_theta
    x2 = st + angular_velocity[lane] * dt
    s.f[1,1]=1f0; s.f[2,1]=0f0; s.f[3,1]=0f0
    s.f[1,2]=0f0; s.f[2,2]=1f0; s.f[3,2]=0f0
    s.f[1,3]=-distance*sin_theta; s.f[2,3]=distance*cos_theta; s.f[3,3]=1f0
    s.g[1,1]=cos_theta*dt; s.g[2,1]=sin_theta*dt; s.g[3,1]=0f0
    s.g[1,2]=0f0; s.g[2,2]=0f0; s.g[3,2]=dt
    mul!(s.left, s.f, view(covariance,:,:,lane))
    mul!(s.predicted_p, s.left, transpose(s.f))
    mul!(s.process_left, s.g, Q)
    mul!(s.process_p, s.process_left, transpose(s.g))
    s.predicted_p .+= s.process_p
    dx = 140f0 - x0; dy = 12f0 - x1; q = dx*dx + dy*dy
    predicted_bearing = atan(dy,dx) - x2
    raw = bearing[lane] - predicted_bearing
    innovation = atan(sin(raw), cos(raw))
    h0 = dy/q; h1 = -dx/q; h2 = -1f0
    for row in 1:3
        s.pht[row] = s.predicted_p[row,1]*h0 + s.predicted_p[row,2]*h1 + s.predicted_p[row,3]*h2
    end
    variance = h0*s.pht[1] + h1*s.pht[2] + h2*s.pht[3] + 0.25f0
    k0=s.pht[1]/variance; k1=s.pht[2]/variance; k2=s.pht[3]/variance
    candidate_state = (x0+k0*innovation, x1+k1*innovation, x2+k2*innovation)
    k=(k0,k1,k2); h=(h0,h1,h2)
    for column in 1:3, row in 1:3
        s.a[row,column] = (row == column ? 1f0 : 0f0) - k[row]*h[column]
    end
    mul!(s.ap, s.a, s.predicted_p); mul!(s.corrected_p, s.ap, transpose(s.a))
    for column in 1:3, row in 1:3
        s.corrected_p[row,column] += k[row]*k[column]*0.25f0
    end
    if VALIDATE && !valid_candidate(candidate_state[1], candidate_state[2], candidate_state[3], s.corrected_p)
        return false
    end
    state[1,lane] = candidate_state[1]
    state[2,lane] = candidate_state[2]
    state[3,lane] = candidate_state[3]
    for column in 1:3, row in 1:3
        covariance[row,column,lane] = s.corrected_p[row,column]
    end
    true
end

function dispatch!(state,covariance,velocity,angular_velocity,bearing,turns,s)
    faults = 0
    for _ in 1:turns, lane in eachindex(velocity)
        faults += !step!(state,covariance,lane,velocity,angular_velocity,bearing,s)
    end
    faults
end

instances = max(1, length(ARGS)>0 ? parse(Int,ARGS[1]) : 10000)
turns = max(1, length(ARGS)>1 ? parse(Int,ARGS[2]) : 5)
phase = Float32(2pi) .* Float32.(0:instances-1) ./ Float32(instances)
velocity = 1f0 .+ 0.05f0 .* sin.(phase .* 3f0)
angular_velocity = 0.015f0 .* (1f0 .+ 0.1f0 .* sin.(phase .* 2f0))
bearing = -0.55f0 .+ 0.01f0 .* sin.(phase .* 7f0) .+ 0.005f0 .* sin.(phase .* 11f0)
state = repeat(reshape(Float32[55,25,0.4],3,1),1,instances)
covariance = repeat(reshape(Float32[100,0,0,0,100,0,0,0,0.15],3,3,1),1,1,instances)
s = Scratch(); dispatch!(state,covariance,velocity,angular_velocity,bearing,5,s)
state .= reshape(Float32[55,25,0.4],3,1)
covariance .= reshape(Float32[100,0,0,0,100,0,0,0,0.15],3,3,1)
started = time_ns(); faults = dispatch!(state,covariance,velocity,angular_velocity,bearing,turns,s)
elapsed = (time_ns()-started)/1e9
println("lane: Julia scalar outer loop")
println("instances: ",instances); println("turns: ",turns); println("elapsed_s: ",elapsed)
println("throughput: ",instances*turns/elapsed); println("checksum: ",sum(Float64,state)+sum(Float64,covariance))
println("validation: ", VALIDATE ? "checked" : "unchecked")
println("faults: ", faults)
