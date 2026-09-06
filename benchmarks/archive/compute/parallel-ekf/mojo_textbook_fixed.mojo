from std.collections import List
from std.math import abs, atan2, cos, sin
from std.sys import argv
from std.time import monotonic
from std.utils.numerics import isfinite

comptime DT: Float32 = 0.1
comptime Q0: Float32 = 0.01
comptime Q1: Float32 = 0.0025
comptime R: Float32 = 0.25
comptime TOL: Float32 = 0.0001
comptime ZERO: Float32 = 0.0
comptime ONE: Float32 = 1.0
comptime LANDMARK_X: Float32 = 140.0
comptime LANDMARK_Y: Float32 = 12.0

struct Vec3:
    var x: Float32
    var y: Float32
    var z: Float32

    def __init__(out self, x: Float32, y: Float32, z: Float32):
        self.x = x
        self.y = y
        self.z = z

    @always_inline
    def add(self, other: Vec3) -> Vec3:
        return Vec3(self.x + other.x, self.y + other.y, self.z + other.z)

    @always_inline
    def scale(self, value: Float32) -> Vec3:
        return Vec3(self.x * value, self.y * value, self.z * value)

    @always_inline
    def dot(self, other: Vec3) -> Float32:
        return self.x * other.x + self.y * other.y + self.z * other.z

struct Mat2:
    var a00: Float32
    var a01: Float32
    var a10: Float32
    var a11: Float32

    def __init__(
        out self,
        a00: Float32,
        a01: Float32,
        a10: Float32,
        a11: Float32,
    ):
        self.a00 = a00
        self.a01 = a01
        self.a10 = a10
        self.a11 = a11

struct Mat32:
    var a00: Float32
    var a01: Float32
    var a10: Float32
    var a11: Float32
    var a20: Float32
    var a21: Float32

    def __init__(
        out self,
        a00: Float32,
        a01: Float32,
        a10: Float32,
        a11: Float32,
        a20: Float32,
        a21: Float32,
    ):
        self.a00 = a00
        self.a01 = a01
        self.a10 = a10
        self.a11 = a11
        self.a20 = a20
        self.a21 = a21

    @always_inline
    def mul(self, other: Mat2) -> Mat32:
        return Mat32(
            self.a00 * other.a00 + self.a01 * other.a10,
            self.a00 * other.a01 + self.a01 * other.a11,
            self.a10 * other.a00 + self.a11 * other.a10,
            self.a10 * other.a01 + self.a11 * other.a11,
            self.a20 * other.a00 + self.a21 * other.a10,
            self.a20 * other.a01 + self.a21 * other.a11,
        )

    @always_inline
    def mul_transpose(self, other: Mat32) -> Mat3:
        return Mat3(
            self.a00 * other.a00 + self.a01 * other.a01,
            self.a00 * other.a10 + self.a01 * other.a11,
            self.a00 * other.a20 + self.a01 * other.a21,
            self.a10 * other.a00 + self.a11 * other.a01,
            self.a10 * other.a10 + self.a11 * other.a11,
            self.a10 * other.a20 + self.a11 * other.a21,
            self.a20 * other.a00 + self.a21 * other.a01,
            self.a20 * other.a10 + self.a21 * other.a11,
            self.a20 * other.a20 + self.a21 * other.a21,
        )

struct Mat3:
    var a00: Float32
    var a01: Float32
    var a02: Float32
    var a10: Float32
    var a11: Float32
    var a12: Float32
    var a20: Float32
    var a21: Float32
    var a22: Float32

    def __init__(
        out self,
        a00: Float32,
        a01: Float32,
        a02: Float32,
        a10: Float32,
        a11: Float32,
        a12: Float32,
        a20: Float32,
        a21: Float32,
        a22: Float32,
    ):
        self.a00 = a00
        self.a01 = a01
        self.a02 = a02
        self.a10 = a10
        self.a11 = a11
        self.a12 = a12
        self.a20 = a20
        self.a21 = a21
        self.a22 = a22

    @always_inline
    def transpose(self) -> Mat3:
        return Mat3(
            self.a00, self.a10, self.a20,
            self.a01, self.a11, self.a21,
            self.a02, self.a12, self.a22,
        )

    @always_inline
    def mul(self, other: Mat3) -> Mat3:
        return Mat3(
            self.a00 * other.a00 + self.a01 * other.a10 + self.a02 * other.a20,
            self.a00 * other.a01 + self.a01 * other.a11 + self.a02 * other.a21,
            self.a00 * other.a02 + self.a01 * other.a12 + self.a02 * other.a22,
            self.a10 * other.a00 + self.a11 * other.a10 + self.a12 * other.a20,
            self.a10 * other.a01 + self.a11 * other.a11 + self.a12 * other.a21,
            self.a10 * other.a02 + self.a11 * other.a12 + self.a12 * other.a22,
            self.a20 * other.a00 + self.a21 * other.a10 + self.a22 * other.a20,
            self.a20 * other.a01 + self.a21 * other.a11 + self.a22 * other.a21,
            self.a20 * other.a02 + self.a21 * other.a12 + self.a22 * other.a22,
        )

    @always_inline
    def add(self, other: Mat3) -> Mat3:
        return Mat3(
            self.a00 + other.a00, self.a01 + other.a01, self.a02 + other.a02,
            self.a10 + other.a10, self.a11 + other.a11, self.a12 + other.a12,
            self.a20 + other.a20, self.a21 + other.a21, self.a22 + other.a22,
        )

    @always_inline
    def sub(self, other: Mat3) -> Mat3:
        return Mat3(
            self.a00 - other.a00, self.a01 - other.a01, self.a02 - other.a02,
            self.a10 - other.a10, self.a11 - other.a11, self.a12 - other.a12,
            self.a20 - other.a20, self.a21 - other.a21, self.a22 - other.a22,
        )

    @always_inline
    def scale(self, value: Float32) -> Mat3:
        return Mat3(
            self.a00 * value, self.a01 * value, self.a02 * value,
            self.a10 * value, self.a11 * value, self.a12 * value,
            self.a20 * value, self.a21 * value, self.a22 * value,
        )

    @always_inline
    def mul_vec(self, x: Vec3) -> Vec3:
        return Vec3(
            self.a00 * x.x + self.a01 * x.y + self.a02 * x.z,
            self.a10 * x.x + self.a11 * x.y + self.a12 * x.z,
            self.a20 * x.x + self.a21 * x.y + self.a22 * x.z,
        )

@always_inline
def identity() -> Mat3:
    return Mat3(
        ONE, ZERO, ZERO,
        ZERO, ONE, ZERO,
        ZERO, ZERO, ONE,
    )

@always_inline
def outer(a: Vec3, b: Vec3) -> Mat3:
    return Mat3(
        a.x * b.x, a.x * b.y, a.x * b.z,
        a.y * b.x, a.y * b.y, a.y * b.z,
        a.z * b.x, a.z * b.y, a.z * b.z,
    )

@always_inline
def valid_candidate(x: Vec3, p: Mat3) -> Bool:
    return (
        isfinite(x.x) and isfinite(x.y) and isfinite(x.z)
        and isfinite(p.a00) and isfinite(p.a01) and isfinite(p.a02)
        and isfinite(p.a10) and isfinite(p.a11) and isfinite(p.a12)
        and isfinite(p.a20) and isfinite(p.a21) and isfinite(p.a22)
        and p.a00 > ZERO and p.a11 > ZERO and p.a22 > ZERO
        and abs(p.a01 - p.a10) <= TOL
        and abs(p.a02 - p.a20) <= TOL
        and abs(p.a12 - p.a21) <= TOL
    )

@always_inline
def step(
    mut state: List[Float32], mut covariance: List[Float32],
    lane: Int, velocity: Float32, angular_velocity: Float32,
    bearing: Float32, checked: Bool,
) -> Bool:
    var sb = lane * 3
    var pb = lane * 9
    var x = Vec3(state[sb], state[sb + 1], state[sb + 2])
    var p = Mat3(
        covariance[pb], covariance[pb + 1], covariance[pb + 2],
        covariance[pb + 3], covariance[pb + 4], covariance[pb + 5],
        covariance[pb + 6], covariance[pb + 7], covariance[pb + 8],
    )
    var theta = x.z
    var s = sin(theta)
    var c = cos(theta)
    var d = velocity * DT
    var x_pred = x.add(Vec3(d * c, d * s, angular_velocity * DT))
    var f = Mat3(
        ONE, ZERO, -d * s,
        ZERO, ONE, d * c,
        ZERO, ZERO, ONE,
    )
    var g = Mat32(c * DT, ZERO, s * DT, ZERO, ZERO, DT)
    var q = Mat2(Q0, ZERO, ZERO, Q1)
    var predicted_p = f.mul(p).mul(f.transpose()).add(g.mul(q).mul_transpose(g))

    var dx = LANDMARK_X - x_pred.x
    var dy = LANDMARK_Y - x_pred.y
    var delta = Vec3(dx, dy, ZERO)
    var squared_range = delta.dot(delta)
    var predicted_bearing = atan2(dy, dx) - x_pred.z
    var innovation = bearing - predicted_bearing
    innovation = atan2(sin(innovation), cos(innovation))
    var h = Vec3(dy / squared_range, -dx / squared_range, -ONE)
    var pht = predicted_p.mul_vec(h)
    var variance = h.dot(pht) + R
    var k = pht.scale(ONE / variance)
    var x_next = x_pred.add(k.scale(innovation))
    var a = identity().sub(outer(k, h))
    var p_next = a.mul(predicted_p).mul(a.transpose()).add(outer(k, k).scale(R))

    if checked and not valid_candidate(x_next, p_next):
        return False
    state[sb] = x_next.x
    state[sb + 1] = x_next.y
    state[sb + 2] = x_next.z
    covariance[pb] = p_next.a00
    covariance[pb + 1] = p_next.a01
    covariance[pb + 2] = p_next.a02
    covariance[pb + 3] = p_next.a10
    covariance[pb + 4] = p_next.a11
    covariance[pb + 5] = p_next.a12
    covariance[pb + 6] = p_next.a20
    covariance[pb + 7] = p_next.a21
    covariance[pb + 8] = p_next.a22
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
                state, covariance, lane, velocity[lane],
                angular_velocity[lane], bearing[lane], checked,
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

    print("lane: Mojo textbook fixed matrix")
    print("workload: resident EKF; ", instances, " filters x ", turns, " turns; fixed Mat3 values; one CPU worker")
    print("instances:", instances)
    print("turns:", turns)
    print("timing: resident turn loop only; setup, warmup, and checksum excluded")
    print("synchronization: per-turn CPU publication")
    print("validation:", "checked" if checked else "unchecked")
    print("faults:", faults)
    print("elapsed_s:", elapsed_s)
    print("throughput:", throughput)
    print("checksum:", checksum)
