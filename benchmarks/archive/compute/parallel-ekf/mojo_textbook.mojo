from std.collections import List
from std.math import abs, atan2, cos, sin
from std.sys import argv
from std.time import monotonic
from std.utils.numerics import isfinite

comptime DT: Float32 = 0.1
comptime Q0: Float32 = 0.01
comptime Q1: Float32 = 0.0025
comptime R: Float32 = 0.25
comptime SYMMETRY_TOLERANCE: Float32 = 0.0001
comptime ZERO: Float32 = 0.0
comptime ONE: Float32 = 1.0
comptime LANDMARK_X: Float32 = 140.0
comptime LANDMARK_Y: Float32 = 12.0

# Matrices are row-major. The helpers deliberately retain the textbook
# expression shape instead of expanding the EKF into scalar temporaries.
def vec3(x0: Float32, x1: Float32, x2: Float32) -> List[Float32]:
    var x = List[Float32](length=3, fill=ZERO)
    x[0] = x0
    x[1] = x1
    x[2] = x2
    return x^

def mat3_zero() -> List[Float32]:
    return List[Float32](length=9, fill=ZERO)

def mat3_identity() -> List[Float32]:
    var a = mat3_zero()
    a[0] = ONE
    a[4] = ONE
    a[8] = ONE
    return a^

def mat2_process() -> List[Float32]:
    var q = List[Float32](length=4, fill=ZERO)
    q[0] = Q0
    q[3] = Q1
    return q^

def mat3_transpose(a: List[Float32]) -> List[Float32]:
    var out = mat3_zero()
    for i in range(3):
        for j in range(3):
            out[i * 3 + j] = a[j * 3 + i]
    return out^

def mat3_mul(a: List[Float32], b: List[Float32]) -> List[Float32]:
    var out = mat3_zero()
    for i in range(3):
        for j in range(3):
            var value = ZERO
            for k in range(3):
                value += a[i * 3 + k] * b[k * 3 + j]
            out[i * 3 + j] = value
    return out^

def mat3_add(a: List[Float32], b: List[Float32]) -> List[Float32]:
    var out = mat3_zero()
    for i in range(9):
        out[i] = a[i] + b[i]
    return out^

def mat3_sub(a: List[Float32], b: List[Float32]) -> List[Float32]:
    var out = mat3_zero()
    for i in range(9):
        out[i] = a[i] - b[i]
    return out^

def mat3_scale(a: List[Float32], scale: Float32) -> List[Float32]:
    var out = mat3_zero()
    for i in range(9):
        out[i] = a[i] * scale
    return out^

def mat3_vec(a: List[Float32], x: List[Float32]) -> List[Float32]:
    var out = vec3(ZERO, ZERO, ZERO)
    for i in range(3):
        var value = ZERO
        for k in range(3):
            value += a[i * 3 + k] * x[k]
        out[i] = value
    return out^

def vec3_add(a: List[Float32], b: List[Float32]) -> List[Float32]:
    return vec3(a[0] + b[0], a[1] + b[1], a[2] + b[2])

def vec3_scale(a: List[Float32], scale: Float32) -> List[Float32]:
    return vec3(a[0] * scale, a[1] * scale, a[2] * scale)

def vec3_dot(a: List[Float32], b: List[Float32]) -> Float32:
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]

def vec3_outer(a: List[Float32], b: List[Float32]) -> List[Float32]:
    var out = mat3_zero()
    for i in range(3):
        for j in range(3):
            out[i * 3 + j] = a[i] * b[j]
    return out^

def mat32_mul22(a: List[Float32], b: List[Float32]) -> List[Float32]:
    var out = List[Float32](length=6, fill=ZERO)
    for i in range(3):
        for j in range(2):
            out[i * 2 + j] = (
                a[i * 2] * b[j]
                + a[i * 2 + 1] * b[2 + j]
            )
    return out^

def mat32_mul_transpose(a: List[Float32], b: List[Float32]) -> List[Float32]:
    var out = mat3_zero()
    for i in range(3):
        for j in range(3):
            out[i * 3 + j] = (
                a[i * 2] * b[j * 2]
                + a[i * 2 + 1] * b[j * 2 + 1]
            )
    return out^

def valid_candidate(x: List[Float32], p: List[Float32]) -> Bool:
    for value in x:
        if not isfinite(value):
            return False
    for value in p:
        if not isfinite(value):
            return False
    if p[0] <= ZERO or p[4] <= ZERO or p[8] <= ZERO:
        return False
    if abs(p[1] - p[3]) > SYMMETRY_TOLERANCE:
        return False
    if abs(p[2] - p[6]) > SYMMETRY_TOLERANCE:
        return False
    if abs(p[5] - p[7]) > SYMMETRY_TOLERANCE:
        return False
    return True

def load_vec3(state: List[Float32], base: Int) -> List[Float32]:
    return vec3(state[base], state[base + 1], state[base + 2])

def load_mat3(covariance: List[Float32], base: Int) -> List[Float32]:
    var out = mat3_zero()
    for i in range(9):
        out[i] = covariance[base + i]
    return out^

def store_vec3(mut state: List[Float32], base: Int, x: List[Float32]):
    state[base] = x[0]
    state[base + 1] = x[1]
    state[base + 2] = x[2]

def store_mat3(mut covariance: List[Float32], base: Int, p: List[Float32]):
    for i in range(9):
        covariance[base + i] = p[i]

def step(
    mut state: List[Float32], mut covariance: List[Float32],
    lane: Int, velocity: Float32, angular_velocity: Float32,
    bearing: Float32, checked: Bool,
) -> Bool:
    var x = load_vec3(state, lane * 3)
    var p = load_mat3(covariance, lane * 9)
    var theta = x[2]
    var s = sin(theta)
    var c = cos(theta)
    var d = velocity * DT

    var x_pred = vec3_add(x, vec3(d * c, d * s, angular_velocity * DT))

    var f = mat3_identity()
    f[2] = -d * s
    f[5] = d * c

    var g = List[Float32](length=6, fill=ZERO)
    g[0] = c * DT
    g[2] = s * DT
    g[5] = DT

    var q = mat2_process()
    var fp = mat3_mul(f, p)
    var predicted_p = mat3_mul(fp, mat3_transpose(f))
    var gq = mat32_mul22(g, q)
    predicted_p = mat3_add(predicted_p, mat32_mul_transpose(gq, g))

    var dx = LANDMARK_X - x_pred[0]
    var dy = LANDMARK_Y - x_pred[1]
    var delta = vec3(dx, dy, ZERO)
    var squared_range = vec3_dot(delta, delta)
    var predicted_bearing = atan2(dy, dx) - x_pred[2]
    var innovation = bearing - predicted_bearing
    innovation = atan2(sin(innovation), cos(innovation))

    var h = vec3(dy / squared_range, -dx / squared_range, -ONE)
    var pht = mat3_vec(predicted_p, h)
    var variance = vec3_dot(h, pht) + R
    var k = vec3_scale(pht, ONE / variance)
    var x_next = vec3_add(x_pred, vec3_scale(k, innovation))

    var identity = mat3_identity()
    var a = mat3_sub(identity, vec3_outer(k, h))
    var ap = mat3_mul(a, predicted_p)
    var p_next = mat3_mul(ap, mat3_transpose(a))
    p_next = mat3_add(p_next, mat3_scale(vec3_outer(k, k), R))

    if checked and not valid_candidate(x_next, p_next):
        return False
    store_vec3(state, lane * 3, x_next)
    store_mat3(covariance, lane * 9, p_next)
    return True

def reset_state(mut state: List[Float32]):
    var lane = 0
    while lane < len(state) // 3:
        state[lane * 3] = Float32(55.0)
        state[lane * 3 + 1] = Float32(25.0)
        state[lane * 3 + 2] = Float32(0.4)
        lane += 1

def reset_covariance(mut covariance: List[Float32]):
    var lane = 0
    while lane < len(covariance) // 9:
        var base = lane * 9
        for i in range(9):
            covariance[base + i] = ZERO
        covariance[base] = Float32(100.0)
        covariance[base + 4] = Float32(100.0)
        covariance[base + 8] = Float32(0.15)
        lane += 1

def dispatch(
    mut state: List[Float32], mut covariance: List[Float32],
    velocity: List[Float32], angular_velocity: List[Float32],
    bearing: List[Float32], turns: Int, checked: Bool,
) -> Int:
    var faults = 0
    var turn = 0
    while turn < turns:
        var lane = 0
        while lane < len(velocity):
            if not step(
                state,
                covariance,
                lane,
                velocity[lane],
                angular_velocity[lane],
                bearing[lane],
                checked,
            ):
                faults += 1
            lane += 1
        turn += 1
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
        angular_velocity[index] = Float32(0.015) * (
            ONE + Float32(0.1) * sin(phase * Float32(2.0))
        )
        bearing[index] = (
            Float32(-0.55)
            + Float32(0.01) * sin(phase * Float32(7.0))
            + Float32(0.005) * sin(phase * Float32(11.0))
        )
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
    for value in state:
        checksum += Float64(value)
    for value in covariance:
        checksum += Float64(value)

    print("lane: Mojo textbook matrix baseline")
    print("workload: resident EKF; ", instances, " filters x ", turns, " turns; matrix helpers; one CPU worker")
    print("instances:", instances)
    print("turns:", turns)
    print("timing: resident turn loop only; setup, warmup, and checksum excluded")
    print("synchronization: per-turn CPU publication")
    print("validation:", "checked" if checked else "unchecked")
    print("faults:", faults)
    print("elapsed_s:", elapsed_s)
    print("throughput:", throughput)
    print("checksum:", checksum)
