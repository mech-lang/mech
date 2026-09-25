"""Matched resident Metal EKF control for the IROS comparison.

Checked and unchecked modes use the same packed structure-of-arrays state,
the same two resident publication buffers, one GPU thread per filter, and one
submission plus synchronization per turn. Checked mode adds only candidate
validation and a compact two-word shared fault status.
"""

using Metal

const DT = 0.1f0
const R = 0.25f0
const SYMMETRY_TOLERANCE = 0.0001f0
const FINITE_LIMIT = 3.402823466f38
const COMPONENTS = 12
const THREADS = 64

@inline function candidate(state, velocity, angular_velocity, bearing, i)
    sx0 = state[i, 1]
    sx1 = state[i, 2]
    sx2 = state[i, 3]
    sp00 = state[i, 4]
    sp01 = state[i, 5]
    sp02 = state[i, 6]
    sp10 = state[i, 7]
    sp11 = state[i, 8]
    sp12 = state[i, 9]
    sp20 = state[i, 10]
    sp21 = state[i, 11]
    sp22 = state[i, 12]

    st, ct = sincos(sx2)
    d = velocity[i] * DT
    nx0 = muladd(d, ct, sx0)
    nx1 = muladd(d, st, sx1)
    nx2 = muladd(angular_velocity[i], DT, sx2)
    f02 = -d * st
    f12 = d * ct

    ap0 = muladd(f02, sp20, sp00)
    ap1 = muladd(f02, sp21, sp01)
    ap2 = muladd(f02, sp22, sp02)
    aq0 = muladd(f12, sp20, sp10)
    aq1 = muladd(f12, sp21, sp11)
    aq2 = muladd(f12, sp22, sp12)
    q00 = ct * ct * 0.0001f0
    q01 = ct * st * 0.0001f0
    q11 = st * st * 0.0001f0
    a00 = muladd(ap2, f02, ap0) + q00
    a01 = muladd(ap2, f12, ap1) + q01
    a02 = ap2
    a10 = muladd(aq2, f02, aq0) + q01
    a11 = muladd(aq2, f12, aq1) + q11
    a12 = aq2
    a20 = muladd(sp22, f02, sp20)
    a21 = muladd(sp22, f12, sp21)
    a22 = sp22 + 0.000025f0

    dx = 140.0f0 - nx0
    dy = 12.0f0 - nx1
    rr = muladd(dx, dx, dy * dy)
    raw = bearing[i] - (atan(dy, dx) - nx2)
    raw_sin, raw_cos = sincos(raw)
    innovation = atan(raw_sin, raw_cos)
    h0 = dy / rr
    h1 = -dx / rr
    h2 = -1.0f0
    ph0 = muladd(a00, h0, muladd(a01, h1, a02 * h2))
    ph1 = muladd(a10, h0, muladd(a11, h1, a12 * h2))
    ph2 = muladd(a20, h0, muladd(a21, h1, a22 * h2))
    variance = muladd(h0, ph0, muladd(h1, ph1, muladd(h2, ph2, R)))
    k0 = ph0 / variance
    k1 = ph1 / variance
    k2 = ph2 / variance
    b00 = 1.0f0 - k0 * h0
    b01 = -k0 * h1
    b02 = -k0 * h2
    b10 = -k1 * h0
    b11 = 1.0f0 - k1 * h1
    b12 = -k1 * h2
    b20 = -k2 * h0
    b21 = -k2 * h1
    b22 = 1.0f0 - k2 * h2
    c00 = muladd(b00, a00, muladd(b01, a10, b02 * a20))
    c01 = muladd(b00, a01, muladd(b01, a11, b02 * a21))
    c02 = muladd(b00, a02, muladd(b01, a12, b02 * a22))
    c10 = muladd(b10, a00, muladd(b11, a10, b12 * a20))
    c11 = muladd(b10, a01, muladd(b11, a11, b12 * a21))
    c12 = muladd(b10, a02, muladd(b11, a12, b12 * a22))
    c20 = muladd(b20, a00, muladd(b21, a10, b22 * a20))
    c21 = muladd(b20, a01, muladd(b21, a11, b22 * a21))
    c22 = muladd(b20, a02, muladd(b21, a12, b22 * a22))

    cx0 = muladd(k0, innovation, nx0)
    cx1 = muladd(k1, innovation, nx1)
    cx2 = muladd(k2, innovation, nx2)
    np00 = muladd(c00, b00, muladd(c01, b01, muladd(c02, b02, k0 * k0 * R)))
    np01 = muladd(c00, b10, muladd(c01, b11, muladd(c02, b12, k0 * k1 * R)))
    np02 = muladd(c00, b20, muladd(c01, b21, muladd(c02, b22, k0 * k2 * R)))
    np10 = muladd(c10, b00, muladd(c11, b01, muladd(c12, b02, k1 * k0 * R)))
    np11 = muladd(c10, b10, muladd(c11, b11, muladd(c12, b12, k1 * k1 * R)))
    np12 = muladd(c10, b20, muladd(c11, b21, muladd(c12, b22, k1 * k2 * R)))
    np20 = muladd(c20, b00, muladd(c21, b01, muladd(c22, b02, k2 * k0 * R)))
    np21 = muladd(c20, b10, muladd(c21, b11, muladd(c22, b12, k2 * k1 * R)))
    np22 = muladd(c20, b20, muladd(c21, b21, muladd(c22, b22, k2 * k2 * R)))
    return (cx0, cx1, cx2, np00, np01, np02, np10, np11, np12, np20, np21, np22)
end

@inline function store_candidate!(output, i, value)
    output[i, 1] = value[1]
    output[i, 2] = value[2]
    output[i, 3] = value[3]
    output[i, 4] = value[4]
    output[i, 5] = value[5]
    output[i, 6] = value[6]
    output[i, 7] = value[7]
    output[i, 8] = value[8]
    output[i, 9] = value[9]
    output[i, 10] = value[10]
    output[i, 11] = value[11]
    output[i, 12] = value[12]
    return
end

@inline function ekf_unchecked!(input, output, velocity, angular_velocity, bearing, n)
    i = thread_position_in_grid().x
    if i <= n
        store_candidate!(output, i, candidate(input, velocity, angular_velocity, bearing, i))
    end
    return
end

@inline function ekf_checked!(input, output, velocity, angular_velocity, bearing, faults, n)
    i = thread_position_in_grid().x
    if i <= n
        value = candidate(input, velocity, angular_velocity, bearing, i)
        finite = abs(value[1]) <= FINITE_LIMIT && abs(value[2]) <= FINITE_LIMIT &&
                 abs(value[3]) <= FINITE_LIMIT && abs(value[4]) <= FINITE_LIMIT &&
                 abs(value[5]) <= FINITE_LIMIT && abs(value[6]) <= FINITE_LIMIT &&
                 abs(value[7]) <= FINITE_LIMIT && abs(value[8]) <= FINITE_LIMIT &&
                 abs(value[9]) <= FINITE_LIMIT && abs(value[10]) <= FINITE_LIMIT &&
                 abs(value[11]) <= FINITE_LIMIT && abs(value[12]) <= FINITE_LIMIT
        positive = value[4] > 0.0f0 && value[8] > 0.0f0 && value[12] > 0.0f0
        symmetric = abs(value[5] - value[7]) <= SYMMETRY_TOLERANCE &&
                    abs(value[6] - value[10]) <= SYMMETRY_TOLERANCE &&
                    abs(value[9] - value[11]) <= SYMMETRY_TOLERANCE
        code = !finite ? Int32(1) : (!positive ? Int32(2) : (!symmetric ? Int32(3) : Int32(0)))
        if code == Int32(0)
            store_candidate!(output, i, value)
        else
            Metal.@atomic faults[1] += Int32(1)
            packed = (Int32(i) << 8) | code
            Metal.@atomic faults[2] = min(faults[2], packed)
        end
    end
    return
end

instances = max(1, length(ARGS) > 0 ? parse(Int, ARGS[1]) : 500_000)
turns = max(1, length(ARGS) > 1 ? parse(Int, ARGS[2]) : 40)
mode = length(ARGS) > 2 ? lowercase(ARGS[3]) : "checked"
mode in ("checked", "unchecked") || error("mode must be checked or unchecked")

phase = Float32(2pi) .* Float32.(0:instances-1) ./ Float32(instances)
velocity = Float32.(1.0 .+ 0.05 .* sin.(phase .* 3.0))
angular_velocity = Float32.(0.015 .* (1.0 .+ 0.1 .* sin.(phase .* 2.0)))
bearing = Float32.(-0.55 .+ 0.01 .* sin.(phase .* 7.0) .+ 0.005 .* sin.(phase .* 11.0))
initial = Matrix{Float32}(undef, instances, COMPONENTS)
initial[:, 1] .= 55.0f0
initial[:, 2] .= 25.0f0
initial[:, 3] .= 0.4f0
initial[:, 4] .= 100.0f0
initial[:, 5:7] .= 0.0f0
initial[:, 8] .= 100.0f0
initial[:, 9:11] .= 0.0f0
initial[:, 12] .= 0.15f0

state = (MtlArray(initial), MtlArray(initial))
device_velocity = MtlArray(velocity)
device_angular_velocity = MtlArray(angular_velocity)
device_bearing = MtlArray(bearing)
faults = mtl(Int32[0, typemax(Int32)]; storage=Metal.SharedStorage)
fault_view = unsafe_wrap(Array, pointer(faults; storage=Metal.SharedStorage), size(faults))
n = Int32(instances)
groups = cld(instances, THREADS)
published_group = 1

function dispatch!(count)
    global published_group
    for _ in 1:count
        input = state[published_group]
        output_group = 3 - published_group
        output = state[output_group]
        if mode == "checked"
            fault_view[1] = Int32(0)
            fault_view[2] = typemax(Int32)
            @metal submit=true threads=THREADS groups=groups ekf_checked!(
                input, output, device_velocity, device_angular_velocity, device_bearing, faults, n)
        else
            @metal submit=true threads=THREADS groups=groups ekf_unchecked!(
                input, output, device_velocity, device_angular_velocity, device_bearing, n)
        end
        synchronize()
        if mode == "checked" && fault_view[1] != 0
            return
        end
        published_group = output_group
    end
end

dispatch!(5)
copyto!(state[1], initial)
copyto!(state[2], initial)
published_group = 1
fault_view[1] = Int32(0)
fault_view[2] = typemax(Int32)
synchronize()

started = time_ns()
dispatch!(turns)
elapsed = (time_ns() - started) / 1e9
host_state = Array(state[published_group])
checksum = sum(Float64.(host_state))

println("lane: Julia Metal GPU, matched packed SoA resident")
println("instances: ", instances)
println("turns: ", turns)
println("elapsed_s: ", elapsed)
println("throughput: ", instances * turns / elapsed)
println("checksum: ", checksum)
println("validation: ", mode)
println("faults: ", Int(fault_view[1]))
println("fault_word: ", UInt32(fault_view[2]))
println("threadgroup_size: ", THREADS)
println("publication: double-buffered in both modes")
println("fault_status: shared host-visible two-word buffer")
