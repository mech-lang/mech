from max.gpu import global_idx
from max.gpu.host import DeviceBuffer, DeviceContext
from std.atomic import Atomic, Ordering
from std.collections import List
from std.math import cos, sin
from std.sys import argv, llvm_intrinsic
from std.time import monotonic

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
comptime BLOCK: Int = 64
# Use Metal's native atan2 intrinsic so the device executes the same exact
# function as the CPU lanes. Host-only libm entry points are not device-safe.
def abs_f32(x: Float32) -> Float32:
    return x if x >= ZERO else -x

def finite_f32(x: Float32) -> Bool:
    return x == x and x < Float32(3.4028235e38) and x > -Float32(3.4028235e38)

def atan2_device(y: Float32, x: Float32) -> Float32:
    return llvm_intrinsic["llvm.air.atan2.f32", Float32](y, x)

def valid_candidate(
    x0: Float32, x1: Float32, x2: Float32,
    p00: Float32, p01: Float32, p02: Float32,
    p10: Float32, p11: Float32, p12: Float32,
    p20: Float32, p21: Float32, p22: Float32,
) -> Bool:
    return (
        finite_f32(x0) and finite_f32(x1) and finite_f32(x2)
        and finite_f32(p00) and finite_f32(p01) and finite_f32(p02)
        and finite_f32(p10) and finite_f32(p11) and finite_f32(p12)
        and finite_f32(p20) and finite_f32(p21) and finite_f32(p22)
        and p00 > ZERO and p11 > ZERO and p22 > ZERO
        and abs_f32(p01 - p10) <= SYMMETRY_TOLERANCE
        and abs_f32(p02 - p20) <= SYMMETRY_TOLERANCE
        and abs_f32(p12 - p21) <= SYMMETRY_TOLERANCE
    )

def ekf_step(
    state0: Pointer[Float32, MutAnyOrigin],
    state1: Pointer[Float32, MutAnyOrigin],
    state2: Pointer[Float32, MutAnyOrigin],
    p00: Pointer[Float32, MutAnyOrigin],
    p01: Pointer[Float32, MutAnyOrigin],
    p02: Pointer[Float32, MutAnyOrigin],
    p10: Pointer[Float32, MutAnyOrigin],
    p11: Pointer[Float32, MutAnyOrigin],
    p12: Pointer[Float32, MutAnyOrigin],
    p20: Pointer[Float32, MutAnyOrigin],
    p21: Pointer[Float32, MutAnyOrigin],
    p22: Pointer[Float32, MutAnyOrigin],
    velocity: Pointer[Float32, MutAnyOrigin],
    angular_velocity: Pointer[Float32, MutAnyOrigin],
    bearing: Pointer[Float32, MutAnyOrigin],
    faults: Pointer[Int32, MutAnyOrigin],
    n: Int32,
    checked: Int32,
):
    var lane = global_idx.x
    if lane >= Int(n):
        return

    var theta = state2[unsafe_offset=lane]
    var st = sin(theta)
    var ct = cos(theta)
    var distance = velocity[unsafe_offset=lane] * DT
    var predicted_x0 = state0[unsafe_offset=lane] + distance * ct
    var predicted_x1 = state1[unsafe_offset=lane] + distance * st
    var predicted_x2 = theta + angular_velocity[unsafe_offset=lane] * DT
    var f02 = -distance * st
    var f12 = distance * ct
    var c00 = p00[unsafe_offset=lane]
    var c01 = p01[unsafe_offset=lane]
    var c02 = p02[unsafe_offset=lane]
    var c10 = p10[unsafe_offset=lane]
    var c11 = p11[unsafe_offset=lane]
    var c12 = p12[unsafe_offset=lane]
    var c20 = p20[unsafe_offset=lane]
    var c21 = p21[unsafe_offset=lane]
    var c22 = p22[unsafe_offset=lane]
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
    var predicted_bearing = atan2_device(dy, dx) - predicted_x2
    var raw_innovation = bearing[unsafe_offset=lane] - predicted_bearing
    var innovation = atan2_device(sin(raw_innovation), cos(raw_innovation))
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
    if checked != 0 and not valid_candidate(
        candidate_x0, candidate_x1, candidate_x2,
        candidate_p00, candidate_p01, candidate_p02,
        candidate_p10, candidate_p11, candidate_p12,
        candidate_p20, candidate_p21, candidate_p22,
    ):
        _ = Atomic[Int32, scope="device"].fetch_add[
            ordering=Ordering.RELAXED
        ](faults, Int32(1))
        return
    state0[unsafe_offset=lane] = candidate_x0
    state1[unsafe_offset=lane] = candidate_x1
    state2[unsafe_offset=lane] = candidate_x2
    p00[unsafe_offset=lane] = candidate_p00
    p01[unsafe_offset=lane] = candidate_p01
    p02[unsafe_offset=lane] = candidate_p02
    p10[unsafe_offset=lane] = candidate_p10
    p11[unsafe_offset=lane] = candidate_p11
    p12[unsafe_offset=lane] = candidate_p12
    p20[unsafe_offset=lane] = candidate_p20
    p21[unsafe_offset=lane] = candidate_p21
    p22[unsafe_offset=lane] = candidate_p22

def fill_inputs(
    mut velocity: List[Float32],
    mut angular_velocity: List[Float32],
    mut bearing: List[Float32],
):
    var denominator = Float32(len(velocity))
    var index = 0
    while index < len(velocity):
        var phase = Float32(6.2831855) * Float32(index) / denominator
        velocity[index] = ONE + Float32(0.05) * sin(phase * Float32(3.0))
        angular_velocity[index] = Float32(0.015) * (ONE + Float32(0.1) * sin(phase * Float32(2.0)))
        bearing[index] = Float32(-0.55) + Float32(0.01) * sin(phase * Float32(7.0)) + Float32(0.005) * sin(phase * Float32(11.0))
        index += 1

def fill_state(
    mut state0: List[Float32],
    mut state1: List[Float32],
    mut state2: List[Float32],
    mut p00: List[Float32],
    mut p01: List[Float32],
    mut p02: List[Float32],
    mut p10: List[Float32],
    mut p11: List[Float32],
    mut p12: List[Float32],
    mut p20: List[Float32],
    mut p21: List[Float32],
    mut p22: List[Float32],
    mut faults: List[Int32],
):
    var index = 0
    while index < len(state0):
        state0[index] = Float32(55.0)
        state1[index] = Float32(25.0)
        state2[index] = Float32(0.4)
        p00[index] = Float32(100.0)
        p01[index] = ZERO
        p02[index] = ZERO
        p10[index] = ZERO
        p11[index] = Float32(100.0)
        p12[index] = ZERO
        p20[index] = ZERO
        p21[index] = ZERO
        p22[index] = Float32(0.15)
        faults[index] = Int32(0)
        index += 1

def upload(ctx: DeviceContext, host: List[Float32]) raises -> DeviceBuffer[.float32]:
    var device = ctx.enqueue_create_buffer[.float32](len(host))
    var staging = ctx.enqueue_create_host_buffer[.float32](len(host))
    for index in range(len(host)):
        staging[index] = host[index]
    staging.enqueue_copy_to(device)
    return device

def run_case(instances: Int, turns: Int, checked: Bool, sync_per_turn: Bool) raises:
    var velocity_host = List[Float32](length=instances, fill=ZERO)
    var angular_host = List[Float32](length=instances, fill=ZERO)
    var bearing_host = List[Float32](length=instances, fill=ZERO)
    fill_inputs(velocity_host, angular_host, bearing_host)
    var state0_host = List[Float32](length=instances, fill=ZERO)
    var state1_host = List[Float32](length=instances, fill=ZERO)
    var state2_host = List[Float32](length=instances, fill=ZERO)
    var p00_host = List[Float32](length=instances, fill=ZERO)
    var p01_host = List[Float32](length=instances, fill=ZERO)
    var p02_host = List[Float32](length=instances, fill=ZERO)
    var p10_host = List[Float32](length=instances, fill=ZERO)
    var p11_host = List[Float32](length=instances, fill=ZERO)
    var p12_host = List[Float32](length=instances, fill=ZERO)
    var p20_host = List[Float32](length=instances, fill=ZERO)
    var p21_host = List[Float32](length=instances, fill=ZERO)
    var p22_host = List[Float32](length=instances, fill=ZERO)
    var faults_host = List[Int32](length=instances, fill=0)
    fill_state(
        state0_host, state1_host, state2_host,
        p00_host, p01_host, p02_host, p10_host, p11_host, p12_host,
        p20_host, p21_host, p22_host, faults_host,
    )

    var ctx = DeviceContext(api="metal")
    var velocity = upload(ctx, velocity_host)
    var angular_velocity = upload(ctx, angular_host)
    var bearing = upload(ctx, bearing_host)
    var state0 = upload(ctx, state0_host)
    var state1 = upload(ctx, state1_host)
    var state2 = upload(ctx, state2_host)
    var p00 = upload(ctx, p00_host)
    var p01 = upload(ctx, p01_host)
    var p02 = upload(ctx, p02_host)
    var p10 = upload(ctx, p10_host)
    var p11 = upload(ctx, p11_host)
    var p12 = upload(ctx, p12_host)
    var p20 = upload(ctx, p20_host)
    var p21 = upload(ctx, p21_host)
    var p22 = upload(ctx, p22_host)
    var faults = ctx.enqueue_create_buffer[.int32](1)
    var kernel = ctx.compile_function[ekf_step]()
    var grid = (instances + BLOCK - 1) // BLOCK
    var enqueue = lambda() raises:
        ctx.enqueue_function(
            kernel,
            state0, state1, state2,
            p00, p01, p02,
            p10, p11, p12,
            p20, p21, p22,
            velocity, angular_velocity, bearing,
            faults, Int32(instances), Int32(checked),
            grid_dim=grid, block_dim=BLOCK,
        )
    var warmup = 5
    var turn = 0
    while turn < warmup:
        enqueue()
        turn += 1
    ctx.synchronize()
    # Allocate a fresh resident state so the timed region excludes reset copies.
    var state0_t = upload(ctx, state0_host)
    var state1_t = upload(ctx, state1_host)
    var state2_t = upload(ctx, state2_host)
    var p00_t = upload(ctx, p00_host)
    var p01_t = upload(ctx, p01_host)
    var p02_t = upload(ctx, p02_host)
    var p10_t = upload(ctx, p10_host)
    var p11_t = upload(ctx, p11_host)
    var p12_t = upload(ctx, p12_host)
    var p20_t = upload(ctx, p20_host)
    var p21_t = upload(ctx, p21_host)
    var p22_t = upload(ctx, p22_host)
    # One device-wide counter is enough: each invalid lane increments it, and
    # the host observes the count at the publication boundary.
    var faults_t = ctx.enqueue_create_buffer[.int32](1)
    ctx.enqueue_memset(faults_t, Int32(0))
    var enqueue_t = lambda() raises:
        ctx.enqueue_function(
            kernel,
            state0_t, state1_t, state2_t,
            p00_t, p01_t, p02_t,
            p10_t, p11_t, p12_t,
            p20_t, p21_t, p22_t,
            velocity, angular_velocity, bearing,
            faults_t, Int32(instances), Int32(checked),
            grid_dim=grid, block_dim=BLOCK,
        )
    ctx.synchronize()
    var started = monotonic()
    turn = 0
    var observed_faults = 0
    while turn < turns:
        if checked:
            ctx.enqueue_memset(faults_t, Int32(0))
        enqueue_t()
        if sync_per_turn:
            ctx.synchronize()
            if checked:
                with faults_t.map_to_host() as fault_host:
                    observed_faults += Int(fault_host[0])
        turn += 1
    if not sync_per_turn:
        ctx.synchronize()
    var elapsed_s = Float64(monotonic() - started) / 1000000000.0
    var throughput = Float64(instances * turns) / elapsed_s
    var checksum: Float64 = 0.0
    with state0_t.map_to_host() as h:
        turn = 0
        while turn < instances:
            checksum += Float64(h[turn])
            turn += 1
    with state1_t.map_to_host() as h:
        turn = 0
        while turn < instances:
            checksum += Float64(h[turn])
            turn += 1
    with state2_t.map_to_host() as h:
        turn = 0
        while turn < instances:
            checksum += Float64(h[turn])
            turn += 1
    with p00_t.map_to_host() as h:
        for index in range(instances):
            checksum += Float64(h[index])
    with p01_t.map_to_host() as h:
        for index in range(instances):
            checksum += Float64(h[index])
    with p02_t.map_to_host() as h:
        for index in range(instances):
            checksum += Float64(h[index])
    with p10_t.map_to_host() as h:
        for index in range(instances):
            checksum += Float64(h[index])
    with p11_t.map_to_host() as h:
        for index in range(instances):
            checksum += Float64(h[index])
    with p12_t.map_to_host() as h:
        for index in range(instances):
            checksum += Float64(h[index])
    with p20_t.map_to_host() as h:
        for index in range(instances):
            checksum += Float64(h[index])
    with p21_t.map_to_host() as h:
        for index in range(instances):
            checksum += Float64(h[index])
    with p22_t.map_to_host() as h:
        for index in range(instances):
            checksum += Float64(h[index])
    var fault_count: Int
    with faults_t.map_to_host() as h:
        fault_count = Int(h[0])
    print("lane: Mojo native Metal resident kernel")
    print("backend: metal")
    print("workload: resident EKF;", instances, "filters x", turns, "turns; one device kernel per turn")
    print("instances:", instances)
    print("turns:", turns)
    print("timing: resident dispatch loop only; setup, compilation, warmup, and final readback excluded")
    print("synchronization:", "per-turn" if sync_per_turn else "deferred-until-end")
    print("validation:", "checked" if checked else "unchecked")
    print("faults:", fault_count)
    print("faults observed at publication boundaries:", observed_faults)
    print("elapsed_s:", elapsed_s)
    print("throughput:", throughput)
    print("checksum:", checksum)

def main() raises:
    var instances = 10000
    var turns = 40
    var checked = False
    var sync_per_turn = True
    if len(argv()) > 1:
        instances = Int(argv()[1])
    if len(argv()) > 2:
        turns = Int(argv()[2])
    if len(argv()) > 3:
        checked = argv()[3] == "checked"
    if len(argv()) > 4:
        sync_per_turn = argv()[4] != "deferred"
    if instances < 1:
        instances = 1
    if turns < 1:
        turns = 1
    run_case(instances, turns, checked, sync_per_turn)
