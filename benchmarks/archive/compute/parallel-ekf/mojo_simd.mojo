from max.algorithm import parallelize
from std.collections import List
from std.math import abs, atan2, cos, sin
from std.sys import argv
from std.time import monotonic
from std.utils.numerics import isfinite

comptime V = SIMD[.float32, 4]
comptime DT: Float32 = 0.1
comptime DT2: Float32 = 0.01
comptime Q0: Float32 = 0.01
comptime Q1: Float32 = 0.0025
comptime R: Float32 = 0.25
comptime TOL: Float32 = 0.0001
comptime ZERO: Float32 = 0.0
comptime ONE: Float32 = 1.0
comptime LANDMARK_X: Float32 = 140.0
comptime LANDMARK_Y: Float32 = 12.0
comptime LANES: Int = 4
comptime WORKERS: Int = 8

def valid_vec(
    x0: V, x1: V, x2: V,
    p00: V, p01: V, p02: V,
    p10: V, p11: V, p12: V,
    p20: V, p21: V, p22: V,
) -> SIMD[.bool, 4]:
    var ok = isfinite(x0)
    ok = ok & isfinite(x1)
    ok = ok & isfinite(x2)
    ok = ok & isfinite(p00)
    ok = ok & isfinite(p01)
    ok = ok & isfinite(p02)
    ok = ok & isfinite(p10)
    ok = ok & isfinite(p11)
    ok = ok & isfinite(p12)
    ok = ok & isfinite(p20)
    ok = ok & isfinite(p21)
    ok = ok & isfinite(p22)
    ok = ok & p00.gt(ZERO)
    ok = ok & p11.gt(ZERO)
    ok = ok & p22.gt(ZERO)
    ok = ok & (abs(p01 - p10)).le(TOL)
    ok = ok & (abs(p02 - p20)).le(TOL)
    ok = ok & (abs(p12 - p21)).le(TOL)
    return ok

@always_inline
def step_vec(
    mut state0: List[V], mut state1: List[V], mut state2: List[V],
    mut p00: List[V], mut p01: List[V], mut p02: List[V],
    mut p10: List[V], mut p11: List[V], mut p12: List[V],
    mut p20: List[V], mut p21: List[V], mut p22: List[V],
    velocity: List[V], angular_velocity: List[V], bearing: List[V],
    group: Int, checked: Bool,
) -> Int:
    var theta = state2[group]
    var st = sin(theta)
    var ct = cos(theta)
    var distance = velocity[group] * DT
    var predicted_x0 = state0[group] + distance * ct
    var predicted_x1 = state1[group] + distance * st
    var predicted_x2 = theta + angular_velocity[group] * DT
    var f02 = -distance * st
    var f12 = distance * ct
    var c00 = p00[group]
    var c01 = p01[group]
    var c02 = p02[group]
    var c10 = p10[group]
    var c11 = p11[group]
    var c12 = p12[group]
    var c20 = p20[group]
    var c21 = p21[group]
    var c22 = p22[group]
    var ap00 = c00 + f02 * c20
    var ap01 = c01 + f02 * c21
    var ap02 = c02 + f02 * c22
    var ap10 = c10 + f12 * c20
    var ap11 = c11 + f12 * c21
    var ap12 = c12 + f12 * c22
    var process00 = ct * ct * DT2 * Q0
    var process01 = ct * st * DT2 * Q0
    var process11 = st * st * DT2 * Q0
    var predicted_p00 = ap00 + ap02 * f02 + process00
    var predicted_p01 = ap01 + ap02 * f12 + process01
    var predicted_p02 = ap02
    var predicted_p10 = ap10 + ap12 * f02 + process01
    var predicted_p11 = ap11 + ap12 * f12 + process11
    var predicted_p12 = ap12
    var predicted_p20 = c20 + c22 * f02
    var predicted_p21 = c21 + c22 * f12
    var predicted_p22 = c22 + DT2 * Q1
    var dx = LANDMARK_X - predicted_x0
    var dy = LANDMARK_Y - predicted_x1
    var squared_range = dx * dx + dy * dy
    var predicted_bearing = atan2(dy, dx) - predicted_x2
    var raw_innovation = bearing[group] - predicted_bearing
    var innovation = atan2(sin(raw_innovation), cos(raw_innovation))
    var h0 = dy / squared_range
    var h1 = -dx / squared_range
    var h2: V = V(-ONE)
    var pht0 = predicted_p00 * h0 + predicted_p01 * h1 + predicted_p02 * h2
    var pht1 = predicted_p10 * h0 + predicted_p11 * h1 + predicted_p12 * h2
    var pht2 = predicted_p20 * h0 + predicted_p21 * h1 + predicted_p22 * h2
    var variance = h0 * pht0 + h1 * pht1 + h2 * pht2 + R
    var k0 = pht0 / variance
    var k1 = pht1 / variance
    var k2 = pht2 / variance
    var candidate_x0 = predicted_x0 + k0 * innovation
    var candidate_x1 = predicted_x1 + k1 * innovation
    var candidate_x2 = predicted_x2 + k2 * innovation
    var a00 = ONE - k0 * h0
    var a01 = -k0 * h1
    var a02 = -k0 * h2
    var a10 = -k1 * h0
    var a11 = ONE - k1 * h1
    var a12 = -k1 * h2
    var a20 = -k2 * h0
    var a21 = -k2 * h1
    var a22 = ONE - k2 * h2
    var b00 = a00 * predicted_p00 + a01 * predicted_p10 + a02 * predicted_p20
    var b01 = a00 * predicted_p01 + a01 * predicted_p11 + a02 * predicted_p21
    var b02 = a00 * predicted_p02 + a01 * predicted_p12 + a02 * predicted_p22
    var b10 = a10 * predicted_p00 + a11 * predicted_p10 + a12 * predicted_p20
    var b11 = a10 * predicted_p01 + a11 * predicted_p11 + a12 * predicted_p21
    var b12 = a10 * predicted_p02 + a11 * predicted_p12 + a12 * predicted_p22
    var b20 = a20 * predicted_p00 + a21 * predicted_p10 + a22 * predicted_p20
    var b21 = a20 * predicted_p01 + a21 * predicted_p11 + a22 * predicted_p21
    var b22 = a20 * predicted_p02 + a21 * predicted_p12 + a22 * predicted_p22
    var candidate_p00 = b00 * a00 + b01 * a01 + b02 * a02 + k0 * k0 * R
    var candidate_p01 = b00 * a10 + b01 * a11 + b02 * a12 + k0 * k1 * R
    var candidate_p02 = b00 * a20 + b01 * a21 + b02 * a22 + k0 * k2 * R
    var candidate_p10 = b10 * a00 + b11 * a01 + b12 * a02 + k1 * k0 * R
    var candidate_p11 = b10 * a10 + b11 * a11 + b12 * a12 + k1 * k1 * R
    var candidate_p12 = b10 * a20 + b11 * a21 + b12 * a22 + k1 * k2 * R
    var candidate_p20 = b20 * a00 + b21 * a01 + b22 * a02 + k2 * k0 * R
    var candidate_p21 = b20 * a10 + b21 * a11 + b22 * a12 + k2 * k1 * R
    var candidate_p22 = b20 * a20 + b21 * a21 + b22 * a22 + k2 * k2 * R
    if not checked:
        state0[group] = candidate_x0
        state1[group] = candidate_x1
        state2[group] = candidate_x2
        p00[group] = candidate_p00
        p01[group] = candidate_p01
        p02[group] = candidate_p02
        p10[group] = candidate_p10
        p11[group] = candidate_p11
        p12[group] = candidate_p12
        p20[group] = candidate_p20
        p21[group] = candidate_p21
        p22[group] = candidate_p22
        return 0
    var valid = valid_vec(
        candidate_x0, candidate_x1, candidate_x2,
        candidate_p00, candidate_p01, candidate_p02,
        candidate_p10, candidate_p11, candidate_p12,
        candidate_p20, candidate_p21, candidate_p22,
    )
    if valid.reduce_and()[0]:
        state0[group] = candidate_x0
        state1[group] = candidate_x1
        state2[group] = candidate_x2
        p00[group] = candidate_p00
        p01[group] = candidate_p01
        p02[group] = candidate_p02
        p10[group] = candidate_p10
        p11[group] = candidate_p11
        p12[group] = candidate_p12
        p20[group] = candidate_p20
        p21[group] = candidate_p21
        p22[group] = candidate_p22
        return 0
    var faults = 0
    var lane = 0
    while lane < LANES:
        if valid[lane]:
            state0[group][lane] = candidate_x0[lane]
            state1[group][lane] = candidate_x1[lane]
            state2[group][lane] = candidate_x2[lane]
            p00[group][lane] = candidate_p00[lane]
            p01[group][lane] = candidate_p01[lane]
            p02[group][lane] = candidate_p02[lane]
            p10[group][lane] = candidate_p10[lane]
            p11[group][lane] = candidate_p11[lane]
            p12[group][lane] = candidate_p12[lane]
            p20[group][lane] = candidate_p20[lane]
            p21[group][lane] = candidate_p21[lane]
            p22[group][lane] = candidate_p22[lane]
        else:
            faults += 1
        lane += 1
    return faults

def dispatch(
    mut state0: List[V], mut state1: List[V], mut state2: List[V],
    mut p00: List[V], mut p01: List[V], mut p02: List[V],
    mut p10: List[V], mut p11: List[V], mut p12: List[V],
    mut p20: List[V], mut p21: List[V], mut p22: List[V],
    velocity: List[V], angular_velocity: List[V], bearing: List[V],
    turns: Int, checked: Bool,
) -> Int:
    var groups = len(velocity)
    var chunk = (groups + WORKERS - 1) // WORKERS
    var worker_faults = List[Int](length=WORKERS, fill=0)
    var turn = 0
    while turn < turns:
        @always_inline
        def worker(worker_id: Int) {mut state0, mut state1, mut state2, mut p00, mut p01, mut p02, mut p10, mut p11, mut p12, mut p20, mut p21, mut p22, imm velocity, imm angular_velocity, imm bearing, mut worker_faults, imm groups, imm chunk, imm checked}:
            var start = worker_id * chunk
            var end = min(start + chunk, groups)
            var faults = 0
            var group = start
            while group < end:
                faults += step_vec(
                    state0, state1, state2,
                    p00, p01, p02, p10, p11, p12, p20, p21, p22,
                    velocity, angular_velocity, bearing, group, checked,
                )
                group += 1
            worker_faults[worker_id] = faults
        parallelize(worker, WORKERS, WORKERS)
        turn += 1
    var faults = 0
    for worker_id in range(WORKERS):
        faults += worker_faults[worker_id]
    return faults

def main() raises:
    var instances = 10000
    var turns = 20
    var checked = False
    if len(argv()) > 1:
        instances = Int(argv()[1])
    if len(argv()) > 2:
        turns = Int(argv()[2])
    if len(argv()) > 3:
        checked = argv()[3] == "checked"
    if instances < 1:
        instances = 1
    if turns < 1:
        turns = 1
    var groups = (instances + LANES - 1) // LANES
    var velocity_scalar = List[Float32](length=instances, fill=ZERO)
    var angular_scalar = List[Float32](length=instances, fill=ZERO)
    var bearing_scalar = List[Float32](length=instances, fill=ZERO)
    var denominator = Float32(instances)
    for index in range(instances):
        var phase = Float32(6.2831855) * Float32(index) / denominator
        velocity_scalar[index] = ONE + Float32(0.05) * sin(phase * Float32(3.0))
        angular_scalar[index] = Float32(0.015) * (ONE + Float32(0.1) * sin(phase * Float32(2.0)))
        bearing_scalar[index] = Float32(-0.55) + Float32(0.01) * sin(phase * Float32(7.0)) + Float32(0.005) * sin(phase * Float32(11.0))
    var velocity = List[V](length=groups, fill=V(ZERO))
    var angular_velocity = List[V](length=groups, fill=V(ZERO))
    var bearing = List[V](length=groups, fill=V(ZERO))
    for group in range(groups):
        var base = group * LANES
        var v0 = velocity_scalar[base]
        var v1 = ZERO
        var v2 = ZERO
        var v3 = ZERO
        var w0 = angular_scalar[base]
        var w1 = ZERO
        var w2 = ZERO
        var w3 = ZERO
        var b0 = bearing_scalar[base]
        var b1 = ZERO
        var b2 = ZERO
        var b3 = ZERO
        if base + 1 < instances:
            v1 = velocity_scalar[base + 1]
            w1 = angular_scalar[base + 1]
            b1 = bearing_scalar[base + 1]
        if base + 2 < instances:
            v2 = velocity_scalar[base + 2]
            w2 = angular_scalar[base + 2]
            b2 = bearing_scalar[base + 2]
        if base + 3 < instances:
            v3 = velocity_scalar[base + 3]
            w3 = angular_scalar[base + 3]
            b3 = bearing_scalar[base + 3]
        velocity[group] = V(v0, v1, v2, v3)
        angular_velocity[group] = V(w0, w1, w2, w3)
        bearing[group] = V(b0, b1, b2, b3)
    var state0 = List[V](length=groups, fill=V(Float32(55.0)))
    var state1 = List[V](length=groups, fill=V(Float32(25.0)))
    var state2 = List[V](length=groups, fill=V(Float32(0.4)))
    var p00 = List[V](length=groups, fill=V(Float32(100.0)))
    var p01 = List[V](length=groups, fill=V(ZERO))
    var p02 = List[V](length=groups, fill=V(ZERO))
    var p10 = List[V](length=groups, fill=V(ZERO))
    var p11 = List[V](length=groups, fill=V(Float32(100.0)))
    var p12 = List[V](length=groups, fill=V(ZERO))
    var p20 = List[V](length=groups, fill=V(ZERO))
    var p21 = List[V](length=groups, fill=V(ZERO))
    var p22 = List[V](length=groups, fill=V(Float32(0.15)))
    _ = dispatch(state0, state1, state2, p00, p01, p02, p10, p11, p12, p20, p21, p22, velocity, angular_velocity, bearing, 2, False)
    state0 = List[V](length=groups, fill=V(Float32(55.0)))
    state1 = List[V](length=groups, fill=V(Float32(25.0)))
    state2 = List[V](length=groups, fill=V(Float32(0.4)))
    p00 = List[V](length=groups, fill=V(Float32(100.0)))
    p01 = List[V](length=groups, fill=V(ZERO))
    p02 = List[V](length=groups, fill=V(ZERO))
    p10 = List[V](length=groups, fill=V(ZERO))
    p11 = List[V](length=groups, fill=V(Float32(100.0)))
    p12 = List[V](length=groups, fill=V(ZERO))
    p20 = List[V](length=groups, fill=V(ZERO))
    p21 = List[V](length=groups, fill=V(ZERO))
    p22 = List[V](length=groups, fill=V(Float32(0.15)))
    var started = monotonic()
    var faults = dispatch(state0, state1, state2, p00, p01, p02, p10, p11, p12, p20, p21, p22, velocity, angular_velocity, bearing, turns, checked)
    var elapsed_s = Float64(monotonic() - started) / 1000000000.0
    var checksum: Float64 = 0.0
    for group in range(groups):
        for lane in range(LANES):
            checksum += Float64(state0[group][lane]) + Float64(state1[group][lane]) + Float64(state2[group][lane])
            checksum += Float64(p00[group][lane]) + Float64(p01[group][lane]) + Float64(p02[group][lane])
            checksum += Float64(p10[group][lane]) + Float64(p11[group][lane]) + Float64(p12[group][lane])
            checksum += Float64(p20[group][lane]) + Float64(p21[group][lane]) + Float64(p22[group][lane])
    print("lane: Mojo fused SIMD-4, 8 workers")
    print("workload: resident EKF;", instances, "filters x", turns, "turns; SIMD-4; 8 CPU workers")
    print("instances:", instances)
    print("turns:", turns)
    print("timing: resident turn loop only; setup, warmup, and checksum excluded")
    print("synchronization: per-turn CPU publication")
    print("validation:", "checked" if checked else "unchecked")
    print("faults:", faults)
    print("elapsed_s:", elapsed_s)
    print("throughput:", Float64(instances * turns) / elapsed_s)
    print("checksum:", checksum)
