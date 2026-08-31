import argparse
import os
import time
import numpy as np
import taichi as ti
DT = np.float32(0.1)
R = np.float32(0.25)
FINITE_LIMIT = np.float32(3.402823466e38)
SYMMETRY_TOLERANCE = np.float32(0.0001)
def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("instances", nargs="?", type=int, default=100_000)
    parser.add_argument("turns", nargs="?", type=int, default=5)
    parser.add_argument(
        "mode",
        nargs="?",
        choices=("checked", "unchecked", "unchecked-batched"),
        default="unchecked",
    )
    parser.add_argument(
        "--cpu-threads",
        type=int,
        help="limit Taichi's LLVM CPU worker pool (only used with TAICHI_ARCH=cpu)",
    )
    return parser.parse_args()
def arch_from_environment():
    name = os.environ.get("TAICHI_ARCH", "gpu").lower()
    architectures = {
        "cpu": ti.cpu,
        "gpu": ti.gpu,
        "cuda": ti.cuda,
        "metal": ti.metal,
        "vulkan": ti.vulkan,
        "opengl": ti.opengl,
    }
    try:
        return architectures[name]
    except KeyError as error:
        raise SystemExit(f"unsupported TAICHI_ARCH={name!r}; choose {', '.join(architectures)}") from error
def make_inputs(instances: int) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    index = np.arange(instances, dtype=np.float32)
    phase = np.float32(2.0 * np.pi) * index / np.float32(instances)
    velocity = np.float32(1.0) + np.float32(0.05) * np.sin(phase * np.float32(3.0)).astype(np.float32)
    angular_velocity = np.float32(0.015) * (
        np.float32(1.0) + np.float32(0.1) * np.sin(phase * np.float32(2.0)).astype(np.float32)
    )
    bearing = (
        np.float32(-0.55)
        + np.float32(0.01) * np.sin(phase * np.float32(7.0)).astype(np.float32)
        + np.float32(0.005) * np.sin(phase * np.float32(11.0)).astype(np.float32)
    )
    return velocity, angular_velocity, bearing
def initial_state(instances: int) -> tuple[np.ndarray, np.ndarray]:
    state = np.empty((instances, 3), dtype=np.float32)
    state[:] = np.array([55.0, 25.0, 0.4], dtype=np.float32)
    covariance = np.zeros((instances, 3, 3), dtype=np.float32)
    covariance[:, 0, 0] = np.float32(100.0)
    covariance[:, 1, 1] = np.float32(100.0)
    covariance[:, 2, 2] = np.float32(0.15)
    return state, covariance
def main() -> None:
    args = parse_args()
    instances = max(1, args.instances)
    turns = max(1, args.turns)
    checked = args.mode == "checked"
    batched = args.mode == "unchecked-batched"
    architecture = arch_from_environment()
    init_kwargs = {"arch": architecture, "default_fp": ti.f32, "kernel_profiler": False}
    if architecture == ti.cpu and args.cpu_threads is not None:
        if args.cpu_threads < 1:
            raise SystemExit("--cpu-threads must be at least 1")
        init_kwargs["cpu_max_num_threads"] = args.cpu_threads
    ti.init(**init_kwargs)
    state = ti.Vector.field(3, dtype=ti.f32, shape=instances)
    covariance = ti.Matrix.field(3, 3, dtype=ti.f32, shape=instances)
    checked_state_a = ti.Vector.field(3, dtype=ti.f32, shape=instances)
    checked_state_b = ti.Vector.field(3, dtype=ti.f32, shape=instances)
    checked_covariance_a = ti.Matrix.field(3, 3, dtype=ti.f32, shape=instances)
    checked_covariance_b = ti.Matrix.field(3, 3, dtype=ti.f32, shape=instances)
    velocity = ti.field(dtype=ti.f32, shape=instances)
    angular_velocity = ti.field(dtype=ti.f32, shape=instances)
    bearing = ti.field(dtype=ti.f32, shape=instances)
    fault_count = ti.field(dtype=ti.i32, shape=())
    fault_instance = ti.field(dtype=ti.i32, shape=())
    process_noise = ti.Matrix([[0.01, 0.0], [0.0, 0.0025]])
    identity = ti.Matrix([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]])
    landmark = ti.Vector([140.0, 12.0])
    @ti.func
    def compute_update(i, xi, pi):
        theta = xi[2]
        sin_theta = ti.sin(theta)
        cos_theta = ti.cos(theta)
        distance = velocity[i] * DT
        predicted_state = xi + ti.Vector([
            distance * cos_theta,
            distance * sin_theta,
            angular_velocity[i] * DT,
        ])
        motion = ti.Matrix([
            [1.0, 0.0, -distance * sin_theta],
            [0.0, 1.0, distance * cos_theta],
            [0.0, 0.0, 1.0],
        ])
        control = ti.Matrix([
            [cos_theta * DT, 0.0],
            [sin_theta * DT, 0.0],
            [0.0, DT],
        ])
        predicted_covariance = (
            motion @ pi @ motion.transpose()
            + control @ process_noise @ control.transpose()
        )
        delta = landmark - ti.Vector([
            predicted_state[0],
            predicted_state[1],
        ])
        delta_x = delta[0]
        delta_y = delta[1]
        squared_range = delta.dot(delta)
        predicted_bearing = ti.atan2(delta_y, delta_x) - predicted_state[2]
        raw_innovation = bearing[i] - predicted_bearing
        innovation = ti.atan2(ti.sin(raw_innovation), ti.cos(raw_innovation))
        observation = ti.Vector([
            delta_y / squared_range,
            -delta_x / squared_range,
            -1.0,
        ])
        ph_t = predicted_covariance @ observation
        innovation_variance = observation.dot(ph_t) + R
        gain = ph_t / innovation_variance
        corrected_state = predicted_state + gain * innovation
        correction = identity - gain.outer_product(observation)
        corrected_covariance = (
            correction @ predicted_covariance @ correction.transpose()
            + gain.outer_product(gain) * R
        )
        return corrected_state, corrected_covariance
    @ti.func
    def valid_candidate(corrected_state, corrected_covariance):
        return (
            abs(corrected_state[0]) <= FINITE_LIMIT
            and abs(corrected_state[1]) <= FINITE_LIMIT
            and abs(corrected_state[2]) <= FINITE_LIMIT
            and abs(corrected_covariance[0, 0]) <= FINITE_LIMIT
            and abs(corrected_covariance[0, 1]) <= FINITE_LIMIT
            and abs(corrected_covariance[0, 2]) <= FINITE_LIMIT
            and abs(corrected_covariance[1, 0]) <= FINITE_LIMIT
            and abs(corrected_covariance[1, 1]) <= FINITE_LIMIT
            and abs(corrected_covariance[1, 2]) <= FINITE_LIMIT
            and abs(corrected_covariance[2, 0]) <= FINITE_LIMIT
            and abs(corrected_covariance[2, 1]) <= FINITE_LIMIT
            and abs(corrected_covariance[2, 2]) <= FINITE_LIMIT
            and corrected_covariance[0, 0] > 0.0
            and corrected_covariance[1, 1] > 0.0
            and corrected_covariance[2, 2] > 0.0
            and abs(corrected_covariance[0, 1] - corrected_covariance[1, 0]) <= SYMMETRY_TOLERANCE
            and abs(corrected_covariance[0, 2] - corrected_covariance[2, 0]) <= SYMMETRY_TOLERANCE
            and abs(corrected_covariance[1, 2] - corrected_covariance[2, 1]) <= SYMMETRY_TOLERANCE
        )
    @ti.kernel
    def ekf_step_unchecked():
        for i in range(instances):
            corrected_state, corrected_covariance = compute_update(i, state[i], covariance[i])
            state[i] = corrected_state
            covariance[i] = corrected_covariance
    @ti.kernel
    def ekf_step_unchecked_batched(turn_count: ti.i32):
        for i in range(instances):
            lane_state = state[i]
            lane_covariance = covariance[i]
            for _ in range(turn_count):
                lane_state, lane_covariance = compute_update(i, lane_state, lane_covariance)
            state[i] = lane_state
            covariance[i] = lane_covariance
    @ti.kernel
    def ekf_step_checked(group: ti.i32):
        for i in range(instances):
            if group == 0:
                corrected_state, corrected_covariance = compute_update(
                    i, checked_state_a[i], checked_covariance_a[i]
                )
                if valid_candidate(corrected_state, corrected_covariance):
                    checked_state_b[i] = corrected_state
                    checked_covariance_b[i] = corrected_covariance
                else:
                    ti.atomic_add(fault_count[None], 1)
                    ti.atomic_min(fault_instance[None], i)
            else:
                corrected_state, corrected_covariance = compute_update(
                    i, checked_state_b[i], checked_covariance_b[i]
                )
                if valid_candidate(corrected_state, corrected_covariance):
                    checked_state_a[i] = corrected_state
                    checked_covariance_a[i] = corrected_covariance
                else:
                    ti.atomic_add(fault_count[None], 1)
                    ti.atomic_min(fault_instance[None], i)
    velocity_values, angular_values, bearing_values = make_inputs(instances)
    velocity.from_numpy(velocity_values)
    angular_velocity.from_numpy(angular_values)
    bearing.from_numpy(bearing_values)
    reset_state, reset_covariance = initial_state(instances)
    def reset() -> None:
        state.from_numpy(reset_state)
        covariance.from_numpy(reset_covariance)
        checked_state_a.from_numpy(reset_state)
        checked_state_b.from_numpy(reset_state)
        checked_covariance_a.from_numpy(reset_covariance)
        checked_covariance_b.from_numpy(reset_covariance)
        fault_count[None] = 0
        fault_instance[None] = instances
        ti.sync()
    def dispatch(count: int) -> tuple[int, int]:
        total_faults = 0
        group = 0
        if batched:
            ekf_step_unchecked_batched(count)
            ti.sync()
            return total_faults, group
        for _ in range(count):
            if checked:
                fault_count[None] = 0
                fault_instance[None] = instances
                ekf_step_checked(group)
            else:
                ekf_step_unchecked()
            ti.sync()
            if checked:
                turn_faults = int(fault_count[None])
                total_faults += turn_faults
                if turn_faults:
                    break
                group = 1 - group
        return total_faults, group
    reset()
    dispatch(5)
    reset()
    started = time.perf_counter()
    fault_count, group = dispatch(turns)
    elapsed = time.perf_counter() - started
    if checked:
        state_values = (checked_state_a if group == 0 else checked_state_b).to_numpy()
        covariance_values = (checked_covariance_a if group == 0 else checked_covariance_b).to_numpy()
    else:
        state_values = state.to_numpy()
        covariance_values = covariance.to_numpy()
    checksum = float(state_values.astype(np.float64).sum() + covariance_values.astype(np.float64).sum())
    print("lane: Taichi Vector/Matrix resident")
    print(f"instances: {instances}")
    print(f"turns: {turns}")
    print(f"elapsed_s: {elapsed:.9f}")
    print(f"throughput: {instances * turns / elapsed:.3f}")
    print(f"checksum: {checksum:.9f}")
    print(f"validation: {'checked' if checked else 'unchecked'}")
    print(f"faults: {fault_count}")
    print(f"synchronization: {'once after batched kernel' if batched else 'per-turn'}")
    if architecture == ti.cpu:
        print(f"cpu_threads: {args.cpu_threads or 'taichi-default'}")
if __name__ == "__main__":
    main()
