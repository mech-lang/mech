"""Synchronous Julia/Metal EKF control.

The state is stored in structure-of-arrays form and one Metal thread owns one
filter.  The two kernels keep validation out of the unchecked launch path.
Checked launches write candidates into an unpublished alternate state, record
a fault summary for invalid lanes, and publish the alternate state only after
the host observes a completed turn with no faults.  Host synchronization and
fault observation remain inside the timed loop so this measures a synchronous
resident runtime boundary rather than a batched one-shot submission.
"""

using Metal

const DT = 0.1f0
const R = 0.25f0
const SYMMETRY_TOLERANCE = 0.0001f0
const FINITE_LIMIT = 3.402823466f38

@inline function ekf_unchecked!(
    x0, x1, x2,
    p00, p01, p02, p10, p11, p12, p20, p21, p22,
    velocity, angular_velocity, bearing, n,
)
    i = thread_position_in_grid().x
    if i <= n
        sx0 = x0[i]
        sx1 = x1[i]
        sx2 = x2[i]
        sp00 = p00[i]
        sp01 = p01[i]
        sp02 = p02[i]
        sp10 = p10[i]
        sp11 = p11[i]
        sp12 = p12[i]
        sp20 = p20[i]
        sp21 = p21[i]
        sp22 = p22[i]

        st = sin(sx2)
        ct = cos(sx2)
        d = velocity[i] * DT
        nx0 = sx0 + d * ct
        nx1 = sx1 + d * st
        nx2 = sx2 + angular_velocity[i] * DT
        f02 = -d * st
        f12 = d * ct

        ap0 = sp00 + f02 * sp20
        ap1 = sp01 + f02 * sp21
        ap2 = sp02 + f02 * sp22
        aq0 = sp10 + f12 * sp20
        aq1 = sp11 + f12 * sp21
        aq2 = sp12 + f12 * sp22
        q00 = ct * ct * 0.0001f0
        q01 = ct * st * 0.0001f0
        q11 = st * st * 0.0001f0
        q22 = 0.000025f0
        a00 = ap0 + ap2 * f02 + q00
        a01 = ap1 + ap2 * f12 + q01
        a02 = ap2
        a10 = aq0 + aq2 * f02 + q01
        a11 = aq1 + aq2 * f12 + q11
        a12 = aq2
        a20 = sp20 + sp22 * f02
        a21 = sp21 + sp22 * f12
        a22 = sp22 + q22

        dx = 140.0f0 - nx0
        dy = 12.0f0 - nx1
        rr = dx * dx + dy * dy
        raw = bearing[i] - (atan(dy, dx) - nx2)
        inn = atan(sin(raw), cos(raw))
        h0 = dy / rr
        h1 = -dx / rr
        h2 = -1.0f0
        ph0 = a00 * h0 + a01 * h1 + a02 * h2
        ph1 = a10 * h0 + a11 * h1 + a12 * h2
        ph2 = a20 * h0 + a21 * h1 + a22 * h2
        iv = h0 * ph0 + h1 * ph1 + h2 * ph2 + R
        k0 = ph0 / iv
        k1 = ph1 / iv
        k2 = ph2 / iv
        b00 = 1.0f0 - k0 * h0
        b01 = -k0 * h1
        b02 = -k0 * h2
        b10 = -k1 * h0
        b11 = 1.0f0 - k1 * h1
        b12 = -k1 * h2
        b20 = -k2 * h0
        b21 = -k2 * h1
        b22 = 1.0f0 - k2 * h2
        c00 = b00 * a00 + b01 * a10 + b02 * a20
        c01 = b00 * a01 + b01 * a11 + b02 * a21
        c02 = b00 * a02 + b01 * a12 + b02 * a22
        c10 = b10 * a00 + b11 * a10 + b12 * a20
        c11 = b10 * a01 + b11 * a11 + b12 * a21
        c12 = b10 * a02 + b11 * a12 + b12 * a22
        c20 = b20 * a00 + b21 * a10 + b22 * a20
        c21 = b20 * a01 + b21 * a11 + b22 * a21
        c22 = b20 * a02 + b21 * a12 + b22 * a22
        np00 = c00 * b00 + c01 * b01 + c02 * b02 + k0 * k0 * R
        np01 = c00 * b10 + c01 * b11 + c02 * b12 + k0 * k1 * R
        np02 = c00 * b20 + c01 * b21 + c02 * b22 + k0 * k2 * R
        np10 = c10 * b00 + c11 * b01 + c12 * b02 + k1 * k0 * R
        np11 = c10 * b10 + c11 * b11 + c12 * b12 + k1 * k1 * R
        np12 = c10 * b20 + c11 * b21 + c12 * b22 + k1 * k2 * R
        np20 = c20 * b00 + c21 * b01 + c22 * b02 + k2 * k0 * R
        np21 = c20 * b10 + c21 * b11 + c22 * b12 + k2 * k1 * R
        np22 = c20 * b20 + c21 * b21 + c22 * b22 + k2 * k2 * R

        x0[i] = nx0 + k0 * inn
        x1[i] = nx1 + k1 * inn
        x2[i] = nx2 + k2 * inn
        p00[i] = np00
        p01[i] = np01
        p02[i] = np02
        p10[i] = np10
        p11[i] = np11
        p12[i] = np12
        p20[i] = np20
        p21[i] = np21
        p22[i] = np22
    end
    return
end

@inline function ekf_checked!(
    x0r, x1r, x2r,
    p00r, p01r, p02r, p10r, p11r, p12r, p20r, p21r, p22r,
    x0w, x1w, x2w,
    p00w, p01w, p02w, p10w, p11w, p12w, p20w, p21w, p22w,
    velocity, angular_velocity, bearing, faults, n,
)
    i = thread_position_in_grid().x
    if i <= n
        sx0 = x0r[i]
        sx1 = x1r[i]
        sx2 = x2r[i]
        sp00 = p00r[i]
        sp01 = p01r[i]
        sp02 = p02r[i]
        sp10 = p10r[i]
        sp11 = p11r[i]
        sp12 = p12r[i]
        sp20 = p20r[i]
        sp21 = p21r[i]
        sp22 = p22r[i]

        st = sin(sx2)
        ct = cos(sx2)
        d = velocity[i] * DT
        nx0 = sx0 + d * ct
        nx1 = sx1 + d * st
        nx2 = sx2 + angular_velocity[i] * DT
        f02 = -d * st
        f12 = d * ct
        ap0 = sp00 + f02 * sp20
        ap1 = sp01 + f02 * sp21
        ap2 = sp02 + f02 * sp22
        aq0 = sp10 + f12 * sp20
        aq1 = sp11 + f12 * sp21
        aq2 = sp12 + f12 * sp22
        q00 = ct * ct * 0.0001f0
        q01 = ct * st * 0.0001f0
        q11 = st * st * 0.0001f0
        a00 = ap0 + ap2 * f02 + q00
        a01 = ap1 + ap2 * f12 + q01
        a02 = ap2
        a10 = aq0 + aq2 * f02 + q01
        a11 = aq1 + aq2 * f12 + q11
        a12 = aq2
        a20 = sp20 + sp22 * f02
        a21 = sp21 + sp22 * f12
        a22 = sp22 + 0.000025f0

        dx = 140.0f0 - nx0
        dy = 12.0f0 - nx1
        rr = dx * dx + dy * dy
        raw = bearing[i] - (atan(dy, dx) - nx2)
        inn = atan(sin(raw), cos(raw))
        h0 = dy / rr
        h1 = -dx / rr
        h2 = -1.0f0
        ph0 = a00 * h0 + a01 * h1 + a02 * h2
        ph1 = a10 * h0 + a11 * h1 + a12 * h2
        ph2 = a20 * h0 + a21 * h1 + a22 * h2
        iv = h0 * ph0 + h1 * ph1 + h2 * ph2 + R
        k0 = ph0 / iv
        k1 = ph1 / iv
        k2 = ph2 / iv
        b00 = 1.0f0 - k0 * h0
        b01 = -k0 * h1
        b02 = -k0 * h2
        b10 = -k1 * h0
        b11 = 1.0f0 - k1 * h1
        b12 = -k1 * h2
        b20 = -k2 * h0
        b21 = -k2 * h1
        b22 = 1.0f0 - k2 * h2
        c00 = b00 * a00 + b01 * a10 + b02 * a20
        c01 = b00 * a01 + b01 * a11 + b02 * a21
        c02 = b00 * a02 + b01 * a12 + b02 * a22
        c10 = b10 * a00 + b11 * a10 + b12 * a20
        c11 = b10 * a01 + b11 * a11 + b12 * a21
        c12 = b10 * a02 + b11 * a12 + b12 * a22
        c20 = b20 * a00 + b21 * a10 + b22 * a20
        c21 = b20 * a01 + b21 * a11 + b22 * a21
        c22 = b20 * a02 + b21 * a12 + b22 * a22
        np00 = c00 * b00 + c01 * b01 + c02 * b02 + k0 * k0 * R
        np01 = c00 * b10 + c01 * b11 + c02 * b12 + k0 * k1 * R
        np02 = c00 * b20 + c01 * b21 + c02 * b22 + k0 * k2 * R
        np10 = c10 * b00 + c11 * b01 + c12 * b02 + k1 * k0 * R
        np11 = c10 * b10 + c11 * b11 + c12 * b12 + k1 * k1 * R
        np12 = c10 * b20 + c11 * b21 + c12 * b22 + k1 * k2 * R
        np20 = c20 * b00 + c21 * b01 + c22 * b02 + k2 * k0 * R
        np21 = c20 * b10 + c21 * b11 + c22 * b12 + k2 * k1 * R
        np22 = c20 * b20 + c21 * b21 + c22 * b22 + k2 * k2 * R

        cx0 = nx0 + k0 * inn
        cx1 = nx1 + k1 * inn
        cx2 = nx2 + k2 * inn
        finite = abs(cx0) <= FINITE_LIMIT && abs(cx1) <= FINITE_LIMIT && abs(cx2) <= FINITE_LIMIT &&
                 abs(np00) <= FINITE_LIMIT && abs(np01) <= FINITE_LIMIT && abs(np02) <= FINITE_LIMIT &&
                 abs(np10) <= FINITE_LIMIT && abs(np11) <= FINITE_LIMIT && abs(np12) <= FINITE_LIMIT &&
                 abs(np20) <= FINITE_LIMIT && abs(np21) <= FINITE_LIMIT && abs(np22) <= FINITE_LIMIT
        positive = np00 > 0.0f0 && np11 > 0.0f0 && np22 > 0.0f0
        symmetric = abs(np01 - np10) <= SYMMETRY_TOLERANCE &&
                    abs(np02 - np20) <= SYMMETRY_TOLERANCE &&
                    abs(np12 - np21) <= SYMMETRY_TOLERANCE
        code = Int32(0)
        if !finite
            code = Int32(1)
        elseif !positive
            code = Int32(2)
        elseif !symmetric
            code = Int32(3)
        end
        if code == Int32(0)
            x0w[i] = cx0
            x1w[i] = cx1
            x2w[i] = cx2
            p00w[i] = np00
            p01w[i] = np01
            p02w[i] = np02
            p10w[i] = np10
            p11w[i] = np11
            p12w[i] = np12
            p20w[i] = np20
            p21w[i] = np21
            p22w[i] = np22
        else
            Metal.@atomic faults[1] += Int32(1)
            packed = (Int32(i) << 8) | code
            Metal.@atomic faults[2] = min(faults[2], packed)
        end
    end
    return
end

instances = max(1, length(ARGS) > 0 ? parse(Int, ARGS[1]) : 100_000)
turns = max(1, length(ARGS) > 1 ? parse(Int, ARGS[2]) : 5)
mode = length(ARGS) > 2 ? lowercase(ARGS[3]) : "unchecked"
mode in ("checked", "unchecked") || error("mode must be checked or unchecked")
T = Float32
phase = T(2pi) .* T.(0:instances-1) ./ T(instances)
velocity = T.(1.0 .+ 0.05 .* sin.(phase .* 3.0))
angular_velocity = T.(0.015 .* (1.0 .+ 0.1 .* sin.(phase .* 2.0)))
bearing = T.(-0.55 .+ 0.01 .* sin.(phase .* 7.0) .+ 0.005 .* sin.(phase .* 11.0))
state = [fill(T(55), instances), fill(T(25), instances), fill(T(0.4), instances),
         fill(T(100), instances), fill(T(0), instances), fill(T(0), instances),
         fill(T(0), instances), fill(T(100), instances), fill(T(0), instances),
         fill(T(0), instances), fill(T(0), instances), fill(T(0.15), instances)]
device_state_a = map(MtlArray, state)
device_state_b = map(MtlArray, state)
device_velocity = MtlArray(velocity)
device_angular_velocity = MtlArray(angular_velocity)
device_bearing = MtlArray(bearing)
faults = MtlArray(Int32[0, typemax(Int32)])
fault_seed = Int32[0, typemax(Int32)]
n = Int32(instances)
groups = cld(instances, 256)
unchecked_args = (device_state_a..., device_velocity, device_angular_velocity, device_bearing, n)
checked_args_a = (device_state_a..., device_state_b..., device_velocity, device_angular_velocity, device_bearing, faults, n)
checked_args_b = (device_state_b..., device_state_a..., device_velocity, device_angular_velocity, device_bearing, faults, n)
published_group = 0

function dispatch!(count)
    global published_group
    for _ in 1:count
        if mode == "checked"
            copyto!(faults, fault_seed)
            args = published_group == 0 ? checked_args_a : checked_args_b
            @metal submit=true threads=256 groups=groups ekf_checked!(args...)
        else
            @metal submit=true threads=256 groups=groups ekf_unchecked!(unchecked_args...)
        end
        synchronize()
        if mode == "checked"
            fault_values = Array(faults)
            if fault_values[1] != 0
                return
            end
            published_group = 1 - published_group
        end
    end
end

# Compile and warm the device kernel before timing steady-state turns.
dispatch!(5)
for (target, source) in zip(device_state_a, state)
    copyto!(target, source)
end
for (target, source) in zip(device_state_b, state)
    copyto!(target, source)
end
published_group = 0
copyto!(faults, fault_seed)
synchronize()
started = time_ns()
dispatch!(turns)
elapsed = (time_ns() - started) / 1e9
published_state = mode == "checked" ? (published_group == 0 ? device_state_a : device_state_b) : device_state_a
host_state = map(Array, published_state)
checksum = sum(sum(Float64.(target)) for target in host_state)
fault_count = Int(Array(faults)[1])
fault_word = UInt32(Array(faults)[2])
println("lane: Julia Metal GPU, SoA resident")
println("instances: ", instances)
println("turns: ", turns)
println("elapsed_s: ", elapsed)
println("throughput: ", instances * turns / elapsed)
println("checksum: ", checksum)
println("validation: ", mode)
println("faults: ", fault_count)
println("fault_word: ", fault_word)
println("synchronization: per-turn Metal.synchronize")
