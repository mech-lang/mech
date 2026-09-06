using Base.Threads

const F32 = Float32
const DT = F32(0.1)
const DT2 = DT * DT
const Q0 = F32(0.01)
const Q1 = F32(0.0025)
const R = F32(0.25)
const TOL = F32(0.0001)
const LIMIT = F32(3.4028235e38)

mutable struct SoAEKF
    x0::Vector{F32}
    x1::Vector{F32}
    x2::Vector{F32}
    p00::Vector{F32}
    p01::Vector{F32}
    p02::Vector{F32}
    p10::Vector{F32}
    p11::Vector{F32}
    p12::Vector{F32}
    p20::Vector{F32}
    p21::Vector{F32}
    p22::Vector{F32}
    v::Vector{F32}
    w::Vector{F32}
    z::Vector{F32}
    faults::Threads.Atomic{Int}
end

function make_state(v::Vector{F32}, w::Vector{F32}, z::Vector{F32})
    n = length(v)
    zeroes() = zeros(F32, n)
    e = SoAEKF(
        fill(F32(55), n), fill(F32(25), n), fill(F32(0.4), n),
        fill(F32(100), n), zeroes(), zeroes(), zeroes(), fill(F32(100), n),
        zeroes(), zeroes(), zeroes(), fill(F32(0.15), n), v, w, z, Threads.Atomic{Int}(0)
    )
    return e
end

function reset!(e::SoAEKF)
    fill!(e.x0, F32(55))
    fill!(e.x1, F32(25))
    fill!(e.x2, F32(0.4))
    fill!(e.p00, F32(100))
    fill!(e.p01, F32(0))
    fill!(e.p02, F32(0))
    fill!(e.p10, F32(0))
    fill!(e.p11, F32(100))
    fill!(e.p12, F32(0))
    fill!(e.p20, F32(0))
    fill!(e.p21, F32(0))
    fill!(e.p22, F32(0.15))
    e.faults[] = 0
end

@inline function finite(x::F32)
    return isfinite(x) && abs(x) <= LIMIT
end

@inline function valid_candidate(
    x0::F32, x1::F32, x2::F32,
    c00::F32, c01::F32, c02::F32,
    c10::F32, c11::F32, c12::F32,
    c20::F32, c21::F32, c22::F32,
)
    return finite(x0) && finite(x1) && finite(x2) &&
        finite(c00) && finite(c01) && finite(c02) &&
        finite(c10) && finite(c11) && finite(c12) &&
        finite(c20) && finite(c21) && finite(c22) &&
        c00 > 0 && c11 > 0 && c22 > 0 &&
        abs(c01 - c10) <= TOL && abs(c02 - c20) <= TOL &&
        abs(c12 - c21) <= TOL
end

@inline function step!(e::SoAEKF, i::Int, checked::Bool)
    @inbounds begin
        theta = e.x2[i]
        st = sin(theta)
        ct = cos(theta)
        d = e.v[i] * DT
        x0 = e.x0[i] + d * ct
        x1 = e.x1[i] + d * st
        x2 = theta + e.w[i] * DT
        f02 = -d * st
        f12 = d * ct
        c00 = e.p00[i]
        c01 = e.p01[i]
        c02 = e.p02[i]
        c10 = e.p10[i]
        c11 = e.p11[i]
        c12 = e.p12[i]
        c20 = e.p20[i]
        c21 = e.p21[i]
        c22 = e.p22[i]
        ap00 = c00 + f02 * c20
        ap01 = c01 + f02 * c21
        ap02 = c02 + f02 * c22
        ap10 = c10 + f12 * c20
        ap11 = c11 + f12 * c21
        ap12 = c12 + f12 * c22
        process00 = ct * ct * DT2 * Q0
        process01 = ct * st * DT2 * Q0
        process11 = st * st * DT2 * Q0
        pp00 = ap00 + ap02 * f02 + process00
        pp01 = ap01 + ap02 * f12 + process01
        pp02 = ap02
        pp10 = ap10 + ap12 * f02 + process01
        pp11 = ap11 + ap12 * f12 + process11
        pp12 = ap12
        pp20 = c20 + c22 * f02
        pp21 = c21 + c22 * f12
        pp22 = c22 + DT2 * Q1
        dx = F32(140) - x0
        dy = F32(12) - x1
        q = dx * dx + dy * dy
        predicted_bearing = atan(dy, dx) - x2
        raw = e.z[i] - predicted_bearing
        innovation = atan(sin(raw), cos(raw))
        h0 = dy / q
        h1 = -dx / q
        h2 = F32(-1)
        pht0 = pp00 * h0 + pp01 * h1 + pp02 * h2
        pht1 = pp10 * h0 + pp11 * h1 + pp12 * h2
        pht2 = pp20 * h0 + pp21 * h1 + pp22 * h2
        variance = h0 * pht0 + h1 * pht1 + h2 * pht2 + R
        k0 = pht0 / variance
        k1 = pht1 / variance
        k2 = pht2 / variance
        nx0 = x0 + k0 * innovation
        nx1 = x1 + k1 * innovation
        nx2 = x2 + k2 * innovation
        a00 = F32(1) - k0 * h0
        a01 = -k0 * h1
        a02 = -k0 * h2
        a10 = -k1 * h0
        a11 = F32(1) - k1 * h1
        a12 = -k1 * h2
        a20 = -k2 * h0
        a21 = -k2 * h1
        a22 = F32(1) - k2 * h2
        b00 = a00 * pp00 + a01 * pp10 + a02 * pp20
        b01 = a00 * pp01 + a01 * pp11 + a02 * pp21
        b02 = a00 * pp02 + a01 * pp12 + a02 * pp22
        b10 = a10 * pp00 + a11 * pp10 + a12 * pp20
        b11 = a10 * pp01 + a11 * pp11 + a12 * pp21
        b12 = a10 * pp02 + a11 * pp12 + a12 * pp22
        b20 = a20 * pp00 + a21 * pp10 + a22 * pp20
        b21 = a20 * pp01 + a21 * pp11 + a22 * pp21
        b22 = a20 * pp02 + a21 * pp12 + a22 * pp22
        n00 = b00 * a00 + b01 * a01 + b02 * a02 + k0 * k0 * R
        n01 = b00 * a10 + b01 * a11 + b02 * a12 + k0 * k1 * R
        n02 = b00 * a20 + b01 * a21 + b02 * a22 + k0 * k2 * R
        n10 = b10 * a00 + b11 * a01 + b12 * a02 + k1 * k0 * R
        n11 = b10 * a10 + b11 * a11 + b12 * a12 + k1 * k1 * R
        n12 = b10 * a20 + b11 * a21 + b12 * a22 + k1 * k2 * R
        n20 = b20 * a00 + b21 * a01 + b22 * a02 + k2 * k0 * R
        n21 = b20 * a10 + b21 * a11 + b22 * a12 + k2 * k1 * R
        n22 = b20 * a20 + b21 * a21 + b22 * a22 + k2 * k2 * R
        if checked && !valid_candidate(nx0, nx1, nx2, n00, n01, n02, n10, n11, n12, n20, n21, n22)
            Threads.atomic_add!(e.faults, 1)
            return nothing
        end
        e.x0[i] = nx0
        e.x1[i] = nx1
        e.x2[i] = nx2
        e.p00[i] = n00
        e.p01[i] = n01
        e.p02[i] = n02
        e.p10[i] = n10
        e.p11[i] = n11
        e.p12[i] = n12
        e.p20[i] = n20
        e.p21[i] = n21
        e.p22[i] = n22
    end
    return nothing
end

function dispatch!(e::SoAEKF, turns::Int, checked::Bool)
    for _ in 1:turns
        if nthreads() == 1
            @inbounds for i in eachindex(e.v)
                step!(e, i, checked)
            end
        else
            @threads :static for i in eachindex(e.v)
                step!(e, i, checked)
            end
        end
    end
end

function inputs(instances::Int)
    phase = F32(2pi) .* F32.(0:(instances - 1)) ./ F32(instances)
    v = F32(1) .+ F32(0.05) .* sin.(phase .* F32(3))
    w = F32(0.015) .* (F32(1) .+ F32(0.1) .* sin.(phase .* F32(2)))
    z = F32(-0.55) .+ F32(0.01) .* sin.(phase .* F32(7)) .+ F32(0.005) .* sin.(phase .* F32(11))
    return v, w, z
end

function checksum(e::SoAEKF)
    return sum(Float64, e.x0) + sum(Float64, e.x1) + sum(Float64, e.x2) +
        sum(Float64, e.p00) + sum(Float64, e.p01) + sum(Float64, e.p02) +
        sum(Float64, e.p10) + sum(Float64, e.p11) + sum(Float64, e.p12) +
        sum(Float64, e.p20) + sum(Float64, e.p21) + sum(Float64, e.p22)
end

instances = max(1, length(ARGS) >= 1 ? parse(Int, ARGS[1]) : 10000)
turns = max(1, length(ARGS) >= 2 ? parse(Int, ARGS[2]) : 20)
checked = length(ARGS) < 3 || lowercase(ARGS[3]) == "checked"
v, w, z = inputs(instances)
e = make_state(v, w, z)
dispatch!(e, 2, checked)
reset!(e)
started = time_ns()
dispatch!(e, turns, checked)
elapsed = Float64(time_ns() - started) / Float64(1e9)
println("lane: Julia fixed-shape SoA fused kernel")
println("workload: resident EKF; ", instances, " filters x ", turns, " turns; explicit SoA f32")
println("timing: resident turn loop only; setup, compilation, warmup, and checksum excluded")
println("synchronization: per-turn CPU publication")
println("instances: ", instances)
println("turns: ", turns)
println("threads: ", nthreads())
println("validation: ", checked ? "checked" : "unchecked")
println("faults: ", e.faults[])
println("elapsed_s: ", elapsed)
println("throughput: ", Float64(instances * turns) / elapsed)
println("checksum: ", checksum(e))
