from std.collections import List
from std.math import abs, atan2, cos, sin
from std.sys import argv
from std.time import monotonic
from std.utils.numerics import isfinite
from max.algorithm import parallelize

comptime DT: Float32 = 0.1
comptime DT2: Float32 = 0.01
comptime Q0: Float32 = 0.01
comptime Q1: Float32 = 0.0025
comptime R: Float32 = 0.25
comptime SYMMETRY_TOLERANCE: Float32 = 0.0001
comptime ZERO: Float32 = 0.0
comptime ONE: Float32 = 1.0
comptime LANDMARK_X: Float32 = 140.0
comptime LANDMARK_Y: Float32 = 12.0
comptime WORKERS: Int = 8

def reset_state(mut state: List[Float32]):
    var i = 0
    while i < len(state):
        if i % 3 == 0:
            state[i] = Float32(55.0)
        elif i % 3 == 1:
            state[i] = Float32(25.0)
        else:
            state[i] = Float32(0.4)
        i += 1

def reset_covariance(mut covariance: List[Float32]):
    var i = 0
    while i < len(covariance):
        var slot = i % 9
        if slot == 0 or slot == 4:
            covariance[i] = Float32(100.0)
        elif slot == 8:
            covariance[i] = Float32(0.15)
        else:
            covariance[i] = Float32(0.0)
        i += 1

def valid_candidate(
    x0: Float32, x1: Float32, x2: Float32,
    p00: Float32, p01: Float32, p02: Float32,
    p10: Float32, p11: Float32, p12: Float32,
    p20: Float32, p21: Float32, p22: Float32,
) -> Bool:
    return (
        isfinite(x0) and isfinite(x1) and isfinite(x2)
        and isfinite(p00) and isfinite(p01) and isfinite(p02)
        and isfinite(p10) and isfinite(p11) and isfinite(p12)
        and isfinite(p20) and isfinite(p21) and isfinite(p22)
        and p00 > ZERO and p11 > ZERO and p22 > ZERO
        and abs(p01 - p10) <= SYMMETRY_TOLERANCE
        and abs(p02 - p20) <= SYMMETRY_TOLERANCE
        and abs(p12 - p21) <= SYMMETRY_TOLERANCE
    )

@always_inline
def step(
    mut state: List[Float32], mut covariance: List[Float32],
    lane: Int, velocity: Float32, angular_velocity: Float32,
    bearing: Float32, checked: Bool,
) -> Bool:
    var state_base = lane * 3
    var covariance_base = lane * 9
    var theta = state[state_base + 2]
    var st = sin(theta)
    var ct = cos(theta)
    var distance = velocity * DT
    var predicted_x0 = state[state_base] + distance * ct
    var predicted_x1 = state[state_base + 1] + distance * st
    var predicted_x2 = theta + angular_velocity * DT
    var f02 = -distance * st
    var f12 = distance * ct
    var p00 = covariance[covariance_base]
    var p01 = covariance[covariance_base + 1]
    var p02 = covariance[covariance_base + 2]
    var p10 = covariance[covariance_base + 3]
    var p11 = covariance[covariance_base + 4]
    var p12 = covariance[covariance_base + 5]
    var p20 = covariance[covariance_base + 6]
    var p21 = covariance[covariance_base + 7]
    var p22 = covariance[covariance_base + 8]
    var ap00 = p00 + f02 * p20
    var ap01 = p01 + f02 * p21
    var ap02 = p02 + f02 * p22
    var ap10 = p10 + f12 * p20
    var ap11 = p11 + f12 * p21
    var ap12 = p12 + f12 * p22
    var process00 = ct * ct * DT2 * Q0
    var process01 = ct * st * DT2 * Q0
    var process11 = st * st * DT2 * Q0
    var predicted_p00 = ap00 + ap02 * f02 + process00
    var predicted_p01 = ap01 + ap02 * f12 + process01
    var predicted_p02 = ap02
    var predicted_p10 = ap10 + ap12 * f02 + process01
    var predicted_p11 = ap11 + ap12 * f12 + process11
    var predicted_p12 = ap12
    var predicted_p20 = p20 + p22 * f02
    var predicted_p21 = p21 + p22 * f12
    var predicted_p22 = p22 + DT2 * Q1
    var dx = LANDMARK_X - predicted_x0
    var dy = LANDMARK_Y - predicted_x1
    var squared_range = dx * dx + dy * dy
    var predicted_bearing = atan2(dy, dx) - predicted_x2
    var raw_innovation = bearing - predicted_bearing
    var innovation = atan2(sin(raw_innovation), cos(raw_innovation))
    var h0 = dy / squared_range
    var h1 = -dx / squared_range
    var h2: Float32 = -ONE
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
    if checked and not valid_candidate(
        candidate_x0, candidate_x1, candidate_x2,
        candidate_p00, candidate_p01, candidate_p02,
        candidate_p10, candidate_p11, candidate_p12,
        candidate_p20, candidate_p21, candidate_p22,
    ):
        return False
    state[state_base] = candidate_x0
    state[state_base + 1] = candidate_x1
    state[state_base + 2] = candidate_x2
    covariance[covariance_base] = candidate_p00
    covariance[covariance_base + 1] = candidate_p01
    covariance[covariance_base + 2] = candidate_p02
    covariance[covariance_base + 3] = candidate_p10
    covariance[covariance_base + 4] = candidate_p11
    covariance[covariance_base + 5] = candidate_p12
    covariance[covariance_base + 6] = candidate_p20
    covariance[covariance_base + 7] = candidate_p21
    covariance[covariance_base + 8] = candidate_p22
    return True

def dispatch(
    mut state: List[Float32], mut covariance: List[Float32],
    velocity: List[Float32], angular_velocity: List[Float32],
    bearing: List[Float32], turns: Int, checked: Bool,
) -> Int:
    var worker_faults = List[Int](length=WORKERS, fill=0)
    var chunk_size = (len(velocity) + WORKERS - 1) // WORKERS
    var turn = 0
    while turn < turns:
        @always_inline
        def worker(worker_id: Int) {mut state, mut covariance, mut worker_faults, imm velocity, imm angular_velocity, imm bearing, imm checked, imm chunk_size}:
            var start = worker_id * chunk_size
            var end = min(start + chunk_size, len(velocity))
            var local_faults = 0
            var lane = start
            while lane < end:
                if not step(state, covariance, lane, velocity[lane], angular_velocity[lane], bearing[lane], checked):
                    local_faults += 1
                lane += 1
            worker_faults[worker_id] = local_faults

        parallelize(worker, WORKERS, WORKERS)
        turn += 1
    var faults = 0
    var worker_id = 0
    while worker_id < WORKERS:
        faults += worker_faults[worker_id]
        worker_id += 1
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
    var velocity = List[Float32](length=instances, fill=ZERO)
    var angular_velocity = List[Float32](length=instances, fill=ZERO)
    var bearing = List[Float32](length=instances, fill=ZERO)
    var state = List[Float32](length=instances * 3, fill=ZERO)
    var covariance = List[Float32](length=instances * 9, fill=ZERO)
    var denominator = Float32(instances)
    var index = 0
    while index < instances:
        var phase = Float32(6.2831855) * Float32(index) / denominator
        velocity[index] = ONE + Float32(0.05) * sin(phase * Float32(3.0))
        angular_velocity[index] = Float32(0.015) * (ONE + Float32(0.1) * sin(phase * Float32(2.0)))
        bearing[index] = Float32(-0.55) + Float32(0.01) * sin(phase * Float32(7.0)) + Float32(0.005) * sin(phase * Float32(11.0))
        index += 1
    reset_state(state)
    reset_covariance(covariance)
    _ = dispatch(state, covariance, velocity, angular_velocity, bearing, 2, False)
    reset_state(state)
    reset_covariance(covariance)
    var started = monotonic()
    var faults = dispatch(state, covariance, velocity, angular_velocity, bearing, turns, checked)
    var finished = monotonic()
    var elapsed_s = Float64(finished - started) / 1000000000.0
    var throughput = Float64(instances * turns) / elapsed_s
    var checksum: Float64 = 0.0
    index = 0
    while index < len(state):
        checksum += Float64(state[index])
        index += 1
    index = 0
    while index < len(covariance):
        checksum += Float64(covariance[index])
        index += 1
    print("lane: Mojo fused fixed-shape, 8 workers")
    print("workload: resident EKF;", instances, "filters x", turns, "turns; flat f32; 8 CPU workers")
    print("instances:", instances)
    print("turns:", turns)
    print("timing: resident turn loop only; setup, warmup, and checksum excluded")
    print("synchronization: per-turn CPU publication")
    print("validation:", "checked" if checked else "unchecked")
    print("faults:", faults)
    print("elapsed_s:", elapsed_s)
    print("throughput:", throughput)
    print("checksum:", checksum)
