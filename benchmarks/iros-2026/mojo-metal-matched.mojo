from max.gpu import global_idx
from max.gpu.host import DeviceBuffer, DeviceContext
from std.atomic import Atomic, Ordering
from std.collections import List
from std.math import cos, sin
from std.sys import argv, llvm_intrinsic
from std.time import monotonic

comptime COMPONENTS: Int = 12
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


def abs_f32(x: Float32) -> Float32:
    return x if x >= ZERO else -x


def finite_f32(x: Float32) -> Bool:
    return x == x and x < Float32(3.4028235e38) and x > -Float32(3.4028235e38)


def atan2_device(y: Float32, x: Float32) -> Float32:
    return llvm_intrinsic["llvm.air.fast_atan2", Float32](y, x)


def sin_device(x: Float32) -> Float32:
    return llvm_intrinsic["llvm.air.fast_sin", Float32](x)


def cos_device(x: Float32) -> Float32:
    return llvm_intrinsic["llvm.air.fast_cos", Float32](x)


def valid_candidate(
    x0: Float32,
    x1: Float32,
    x2: Float32,
    p00: Float32,
    p01: Float32,
    p02: Float32,
    p10: Float32,
    p11: Float32,
    p12: Float32,
    p20: Float32,
    p21: Float32,
    p22: Float32,
) -> Bool:
    return (
        finite_f32(x0)
        and finite_f32(x1)
        and finite_f32(x2)
        and finite_f32(p00)
        and finite_f32(p01)
        and finite_f32(p02)
        and finite_f32(p10)
        and finite_f32(p11)
        and finite_f32(p12)
        and finite_f32(p20)
        and finite_f32(p21)
        and finite_f32(p22)
        and p00 > ZERO
        and p11 > ZERO
        and p22 > ZERO
        and abs_f32(p01 - p10) <= SYMMETRY_TOLERANCE
        and abs_f32(p02 - p20) <= SYMMETRY_TOLERANCE
        and abs_f32(p12 - p21) <= SYMMETRY_TOLERANCE
    )


def ekf_step[
    checked: Bool
](
    state: Pointer[Float32, MutAnyOrigin],
    next_state: Pointer[Float32, MutAnyOrigin],
    velocity: Pointer[Float32, MutAnyOrigin],
    angular_velocity: Pointer[Float32, MutAnyOrigin],
    bearing: Pointer[Float32, MutAnyOrigin],
    faults: Pointer[Int32, MutAnyOrigin],
    n: Int32,
):
    var lane = global_idx.x
    if lane >= Int(n):
        return
    var stride = Int(n)

    var sx0 = state[unsafe_offset=lane]
    var sx1 = state[unsafe_offset=stride + lane]
    var sx2 = state[unsafe_offset=2 * stride + lane]
    var sp00 = state[unsafe_offset=3 * stride + lane]
    var sp01 = state[unsafe_offset=4 * stride + lane]
    var sp02 = state[unsafe_offset=5 * stride + lane]
    var sp10 = state[unsafe_offset=6 * stride + lane]
    var sp11 = state[unsafe_offset=7 * stride + lane]
    var sp12 = state[unsafe_offset=8 * stride + lane]
    var sp20 = state[unsafe_offset=9 * stride + lane]
    var sp21 = state[unsafe_offset=10 * stride + lane]
    var sp22 = state[unsafe_offset=11 * stride + lane]

    var st = sin_device(sx2)
    var ct = cos_device(sx2)
    var distance = velocity[unsafe_offset=lane] * DT
    var predicted_x0 = sx0 + distance * ct
    var predicted_x1 = sx1 + distance * st
    var predicted_x2 = sx2 + angular_velocity[unsafe_offset=lane] * DT
    var f02 = -distance * st
    var f12 = distance * ct
    var ap00 = sp00 + f02 * sp20
    var ap01 = sp01 + f02 * sp21
    var ap02 = sp02 + f02 * sp22
    var ap10 = sp10 + f12 * sp20
    var ap11 = sp11 + f12 * sp21
    var ap12 = sp12 + f12 * sp22
    var process00 = ct * ct * Float32(0.0001)
    var process01 = ct * st * Float32(0.0001)
    var process11 = st * st * Float32(0.0001)
    var predicted_p00 = ap00 + ap02 * f02 + process00
    var predicted_p01 = ap01 + ap02 * f12 + process01
    var predicted_p02 = ap02
    var predicted_p10 = ap10 + ap12 * f02 + process01
    var predicted_p11 = ap11 + ap12 * f12 + process11
    var predicted_p12 = ap12
    var predicted_p20 = sp20 + sp22 * f02
    var predicted_p21 = sp21 + sp22 * f12
    var predicted_p22 = sp22 + DT2 * Q1
    var dx = LANDMARK_X - predicted_x0
    var dy = LANDMARK_Y - predicted_x1
    var squared_range = dx * dx + dy * dy
    var predicted_bearing = atan2_device(dy, dx) - predicted_x2
    var raw_innovation = bearing[unsafe_offset=lane] - predicted_bearing
    var innovation = atan2_device(
        sin_device(raw_innovation), cos_device(raw_innovation)
    )
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
        candidate_x0,
        candidate_x1,
        candidate_x2,
        candidate_p00,
        candidate_p01,
        candidate_p02,
        candidate_p10,
        candidate_p11,
        candidate_p12,
        candidate_p20,
        candidate_p21,
        candidate_p22,
    ):
        _ = Atomic[Int32, scope="device"].fetch_add[ordering=Ordering.RELAXED](
            faults, Int32(1)
        )
        Atomic[Int32, scope="device"].min[ordering=Ordering.RELAXED](
            faults.unsafe_offset(1), Int32((lane << 8) | 1)
        )
        return

    next_state[unsafe_offset=lane] = candidate_x0
    next_state[unsafe_offset=stride + lane] = candidate_x1
    next_state[unsafe_offset=2 * stride + lane] = candidate_x2
    next_state[unsafe_offset=3 * stride + lane] = candidate_p00
    next_state[unsafe_offset=4 * stride + lane] = candidate_p01
    next_state[unsafe_offset=5 * stride + lane] = candidate_p02
    next_state[unsafe_offset=6 * stride + lane] = candidate_p10
    next_state[unsafe_offset=7 * stride + lane] = candidate_p11
    next_state[unsafe_offset=8 * stride + lane] = candidate_p12
    next_state[unsafe_offset=9 * stride + lane] = candidate_p20
    next_state[unsafe_offset=10 * stride + lane] = candidate_p21
    next_state[unsafe_offset=11 * stride + lane] = candidate_p22


def fill_inputs(
    mut velocity: List[Float32],
    mut angular_velocity: List[Float32],
    mut bearing: List[Float32],
):
    var denominator = Float32(len(velocity))
    for index in range(len(velocity)):
        var phase = Float32(6.2831855) * Float32(index) / denominator
        velocity[index] = ONE + Float32(0.05) * sin(phase * Float32(3.0))
        angular_velocity[index] = Float32(0.015) * (
            ONE + Float32(0.1) * sin(phase * Float32(2.0))
        )
        bearing[index] = (
            Float32(-0.55)
            + Float32(0.01) * sin(phase * Float32(7.0))
            + Float32(0.005) * sin(phase * Float32(11.0))
        )


def fill_state(mut state: List[Float32], instances: Int):
    for component in range(COMPONENTS):
        var value = ZERO
        if component == 0:
            value = Float32(55.0)
        elif component == 1:
            value = Float32(25.0)
        elif component == 2:
            value = Float32(0.4)
        elif component == 3 or component == 7:
            value = Float32(100.0)
        elif component == 11:
            value = Float32(0.15)
        for lane in range(instances):
            state[component * instances + lane] = value


def upload(
    ctx: DeviceContext, host: List[Float32]
) raises -> DeviceBuffer[.float32]:
    var device = ctx.enqueue_create_buffer[.float32](len(host))
    var staging = ctx.enqueue_create_host_buffer[.float32](len(host))
    for index in range(len(host)):
        staging[index] = host[index]
    staging.enqueue_copy_to(device)
    return device


def run_case(
    instances: Int, turns: Int, checked: Bool, block: Int, inject_fault: Bool
) raises:
    var velocity_host = List[Float32](length=instances, fill=ZERO)
    var angular_host = List[Float32](length=instances, fill=ZERO)
    var bearing_host = List[Float32](length=instances, fill=ZERO)
    fill_inputs(velocity_host, angular_host, bearing_host)
    if inject_fault:
        velocity_host[0] = Float32.MAX
    var state_host = List[Float32](length=COMPONENTS * instances, fill=ZERO)
    fill_state(state_host, instances)

    var ctx = DeviceContext(api="metal")
    var velocity = upload(ctx, velocity_host)
    var angular_velocity = upload(ctx, angular_host)
    var bearing = upload(ctx, bearing_host)
    var state_a = upload(ctx, state_host)
    var state_b = upload(ctx, state_host)
    var faults = ctx.enqueue_create_buffer[.int32](2)
    var checked_kernel = ctx.compile_function[ekf_step[True]]()
    var unchecked_kernel = ctx.compile_function[ekf_step[False]]()
    var grid = (instances + block - 1) // block
    ctx.synchronize()
    var fault_host = faults.unsafe_host_ptr()

    var source_is_a = True
    for _ in range(5):
        if checked:
            fault_host[] = Int32(0)
            fault_host[unsafe_offset=1] = Int32.MAX
        if source_is_a:
            if checked:
                ctx.enqueue_function(
                    checked_kernel,
                    state_a,
                    state_b,
                    velocity,
                    angular_velocity,
                    bearing,
                    faults,
                    Int32(instances),
                    grid_dim=grid,
                    block_dim=block,
                )
            else:
                ctx.enqueue_function(
                    unchecked_kernel,
                    state_a,
                    state_b,
                    velocity,
                    angular_velocity,
                    bearing,
                    faults,
                    Int32(instances),
                    grid_dim=grid,
                    block_dim=block,
                )
        else:
            if checked:
                ctx.enqueue_function(
                    checked_kernel,
                    state_b,
                    state_a,
                    velocity,
                    angular_velocity,
                    bearing,
                    faults,
                    Int32(instances),
                    grid_dim=grid,
                    block_dim=block,
                )
            else:
                ctx.enqueue_function(
                    unchecked_kernel,
                    state_b,
                    state_a,
                    velocity,
                    angular_velocity,
                    bearing,
                    faults,
                    Int32(instances),
                    grid_dim=grid,
                    block_dim=block,
                )
        ctx.synchronize()
        if not checked or fault_host[] == 0:
            source_is_a = not source_is_a

    # Reset both resident state buffers outside the timed region.
    state_a = upload(ctx, state_host)
    state_b = upload(ctx, state_host)
    source_is_a = True
    ctx.synchronize()

    var observed_faults = 0
    var started = monotonic()
    for _ in range(turns):
        if checked:
            fault_host[] = Int32(0)
            fault_host[unsafe_offset=1] = Int32.MAX
        if source_is_a:
            if checked:
                ctx.enqueue_function(
                    checked_kernel,
                    state_a,
                    state_b,
                    velocity,
                    angular_velocity,
                    bearing,
                    faults,
                    Int32(instances),
                    grid_dim=grid,
                    block_dim=block,
                )
            else:
                ctx.enqueue_function(
                    unchecked_kernel,
                    state_a,
                    state_b,
                    velocity,
                    angular_velocity,
                    bearing,
                    faults,
                    Int32(instances),
                    grid_dim=grid,
                    block_dim=block,
                )
        else:
            if checked:
                ctx.enqueue_function(
                    checked_kernel,
                    state_b,
                    state_a,
                    velocity,
                    angular_velocity,
                    bearing,
                    faults,
                    Int32(instances),
                    grid_dim=grid,
                    block_dim=block,
                )
            else:
                ctx.enqueue_function(
                    unchecked_kernel,
                    state_b,
                    state_a,
                    velocity,
                    angular_velocity,
                    bearing,
                    faults,
                    Int32(instances),
                    grid_dim=grid,
                    block_dim=block,
                )
        ctx.synchronize()
        var turn_faults = 0
        if checked:
            turn_faults = Int(fault_host[])
            observed_faults += turn_faults
        if turn_faults == 0:
            source_is_a = not source_is_a
    var elapsed_s = Float64(monotonic() - started) / 1000000000.0
    var throughput = Float64(instances * turns) / elapsed_s / 1000000.0

    var checksum: Float64 = 0.0
    if source_is_a:
        with state_a.map_to_host() as state:
            for index in range(COMPONENTS * instances):
                checksum += Float64(state[index])
    else:
        with state_b.map_to_host() as state:
            for index in range(COMPONENTS * instances):
                checksum += Float64(state[index])

    print("implementation: Mojo native Metal matched")
    print("mode:", "checked" if checked else "unchecked")
    print("instances:", instances)
    print("turns:", turns)
    print("threadgroup_size:", block)
    print("elapsed_s:", elapsed_s)
    print("throughput_million_ekf_turns_per_second:", throughput)
    print("checksum:", checksum)
    print("faults:", observed_faults)
    print("fault_word:", UInt32(fault_host[unsafe_offset=1]))
    print("resident_bytes:", (2 * COMPONENTS + 3) * instances * 4 + 8)


def main() raises:
    var instances = 500000
    var turns = 40
    var checked = True
    var block = 64
    var inject_fault = False
    if len(argv()) > 1:
        instances = Int(argv()[1])
    if len(argv()) > 2:
        turns = Int(argv()[2])
    if len(argv()) > 3:
        checked = argv()[3] == "checked"
    if len(argv()) > 4:
        block = Int(argv()[4])
    if len(argv()) > 5:
        inject_fault = argv()[5] == "fault"
    if instances < 1:
        instances = 1
    if turns < 1:
        turns = 1
    if block < 1:
        block = 1
    run_case(instances, turns, checked, block, inject_fault)
