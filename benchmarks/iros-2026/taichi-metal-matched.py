#!/usr/bin/env python3
"""Matched Taichi 1.7.4 Metal control for the parallel EKF benchmark."""

import argparse
import time

import numpy as np
import taichi as ti


DT = np.float32(0.1)
DT2 = np.float32(0.01)
R = np.float32(0.25)
FINITE_LIMIT = np.float32(3.402823466e38)
SYMMETRY_TOLERANCE = np.float32(0.0001)
COMPONENTS = 12
NO_FAULT = 2**31 - 1


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("instances", nargs="?", type=int, default=500_000)
    parser.add_argument("turns", nargs="?", type=int, default=40)
    parser.add_argument("mode", nargs="?", choices=("checked", "unchecked"), default="checked")
    parser.add_argument("--arch", choices=("metal", "cpu"), default="metal")
    parser.add_argument("--block-dim", type=int, default=32)
    parser.add_argument("--cpu-threads", type=int, default=8)
    parser.add_argument("--inject-fault", action="store_true")
    return parser.parse_args()


def make_inputs(instances: int, inject_fault: bool) -> np.ndarray:
    index = np.arange(instances, dtype=np.float32)
    phase = np.float32(2.0 * np.pi) * index / np.float32(instances)
    velocity = np.float32(1.0) + np.float32(0.05) * np.sin(
        phase * np.float32(3.0)
    ).astype(np.float32)
    angular_velocity = np.float32(0.015) * (
        np.float32(1.0)
        + np.float32(0.1)
        * np.sin(phase * np.float32(2.0)).astype(np.float32)
    )
    bearing = (
        np.float32(-0.55)
        + np.float32(0.01)
        * np.sin(phase * np.float32(7.0)).astype(np.float32)
        + np.float32(0.005)
        * np.sin(phase * np.float32(11.0)).astype(np.float32)
    )
    if inject_fault:
        velocity[0] = np.finfo(np.float32).max
    return np.stack((velocity, angular_velocity, bearing))


def initial_state(instances: int) -> np.ndarray:
    state = np.zeros((COMPONENTS, instances), dtype=np.float32)
    state[0].fill(np.float32(55.0))
    state[1].fill(np.float32(25.0))
    state[2].fill(np.float32(0.4))
    state[3].fill(np.float32(100.0))
    state[7].fill(np.float32(100.0))
    state[11].fill(np.float32(0.15))
    return state


def main() -> None:
    args = parse_args()
    if args.block_dim < 1 or args.block_dim & (args.block_dim - 1):
        raise SystemExit("--block-dim must be a positive power of two")
    instances = max(1, args.instances)
    turns = max(1, args.turns)
    checked = args.mode == "checked"

    init_options = {
        "arch": ti.metal if args.arch == "metal" else ti.cpu,
        "default_fp": ti.f32,
        "fast_math": False,
        "kernel_profiler": False,
    }
    if args.arch == "cpu":
        init_options["cpu_max_num_threads"] = max(1, args.cpu_threads)
    ti.init(**init_options)
    if args.arch == "metal":
        inputs = ti.field(dtype=ti.f32, shape=(3, instances))
        state = ti.field(dtype=ti.f32, shape=(2, COMPONENTS, instances))
    else:
        inputs = ti.field(dtype=ti.f32, shape=(instances, 3))
        state = ti.field(dtype=ti.f32, shape=(instances, COMPONENTS, 2))
    fault_count = ti.field(dtype=ti.i32, shape=())
    fault_word = ti.field(dtype=ti.i32, shape=())

    Update = ti.types.struct(
        x0=ti.f32,
        x1=ti.f32,
        x2=ti.f32,
        p00=ti.f32,
        p01=ti.f32,
        p02=ti.f32,
        p10=ti.f32,
        p11=ti.f32,
        p12=ti.f32,
        p20=ti.f32,
        p21=ti.f32,
        p22=ti.f32,
    )

    @ti.func
    def compute_update(i, group):
        sx0, sx1, sx2 = ti.f32(0.0), ti.f32(0.0), ti.f32(0.0)
        sp00, sp01, sp02 = ti.f32(0.0), ti.f32(0.0), ti.f32(0.0)
        sp10, sp11, sp12 = ti.f32(0.0), ti.f32(0.0), ti.f32(0.0)
        sp20, sp21, sp22 = ti.f32(0.0), ti.f32(0.0), ti.f32(0.0)
        velocity, angular_velocity, bearing = ti.f32(0.0), ti.f32(0.0), ti.f32(0.0)
        if ti.static(args.arch == "metal"):
            sx0, sx1, sx2 = state[group, 0, i], state[group, 1, i], state[group, 2, i]
            sp00, sp01, sp02 = state[group, 3, i], state[group, 4, i], state[group, 5, i]
            sp10, sp11, sp12 = state[group, 6, i], state[group, 7, i], state[group, 8, i]
            sp20, sp21, sp22 = state[group, 9, i], state[group, 10, i], state[group, 11, i]
            velocity = inputs[0, i]
            angular_velocity = inputs[1, i]
            bearing = inputs[2, i]
        else:
            sx0, sx1, sx2 = state[i, 0, group], state[i, 1, group], state[i, 2, group]
            sp00, sp01, sp02 = state[i, 3, group], state[i, 4, group], state[i, 5, group]
            sp10, sp11, sp12 = state[i, 6, group], state[i, 7, group], state[i, 8, group]
            sp20, sp21, sp22 = state[i, 9, group], state[i, 10, group], state[i, 11, group]
            velocity = inputs[i, 0]
            angular_velocity = inputs[i, 1]
            bearing = inputs[i, 2]
        st = ti.sin(sx2)
        ct = ti.cos(sx2)
        distance = velocity * DT
        predicted_x0 = sx0 + distance * ct
        predicted_x1 = sx1 + distance * st
        predicted_x2 = sx2 + angular_velocity * DT
        f02 = -distance * st
        f12 = distance * ct
        ap00 = sp00 + f02 * sp20
        ap01 = sp01 + f02 * sp21
        ap02 = sp02 + f02 * sp22
        ap10 = sp10 + f12 * sp20
        ap11 = sp11 + f12 * sp21
        ap12 = sp12 + f12 * sp22
        process00 = ct * ct * ti.f32(0.0001)
        process01 = ct * st * ti.f32(0.0001)
        process11 = st * st * ti.f32(0.0001)
        predicted_p00 = ap00 + ap02 * f02 + process00
        predicted_p01 = ap01 + ap02 * f12 + process01
        predicted_p02 = ap02
        predicted_p10 = ap10 + ap12 * f02 + process01
        predicted_p11 = ap11 + ap12 * f12 + process11
        predicted_p12 = ap12
        predicted_p20 = sp20 + sp22 * f02
        predicted_p21 = sp21 + sp22 * f12
        predicted_p22 = sp22 + DT2 * ti.f32(0.0025)
        dx = ti.f32(140.0) - predicted_x0
        dy = ti.f32(12.0) - predicted_x1
        squared_range = dx * dx + dy * dy
        predicted_bearing = ti.atan2(dy, dx) - predicted_x2
        raw_innovation = bearing - predicted_bearing
        innovation = ti.atan2(ti.sin(raw_innovation), ti.cos(raw_innovation))
        h0 = dy / squared_range
        h1 = -dx / squared_range
        h2 = ti.f32(-1.0)
        pht0 = predicted_p00 * h0 + predicted_p01 * h1 + predicted_p02 * h2
        pht1 = predicted_p10 * h0 + predicted_p11 * h1 + predicted_p12 * h2
        pht2 = predicted_p20 * h0 + predicted_p21 * h1 + predicted_p22 * h2
        variance = h0 * pht0 + h1 * pht1 + h2 * pht2 + R
        k0, k1, k2 = pht0 / variance, pht1 / variance, pht2 / variance
        candidate_x0 = predicted_x0 + k0 * innovation
        candidate_x1 = predicted_x1 + k1 * innovation
        candidate_x2 = predicted_x2 + k2 * innovation
        a00, a01, a02 = ti.f32(1.0) - k0 * h0, -k0 * h1, -k0 * h2
        a10, a11, a12 = -k1 * h0, ti.f32(1.0) - k1 * h1, -k1 * h2
        a20, a21, a22 = -k2 * h0, -k2 * h1, ti.f32(1.0) - k2 * h2
        b00 = a00 * predicted_p00 + a01 * predicted_p10 + a02 * predicted_p20
        b01 = a00 * predicted_p01 + a01 * predicted_p11 + a02 * predicted_p21
        b02 = a00 * predicted_p02 + a01 * predicted_p12 + a02 * predicted_p22
        b10 = a10 * predicted_p00 + a11 * predicted_p10 + a12 * predicted_p20
        b11 = a10 * predicted_p01 + a11 * predicted_p11 + a12 * predicted_p21
        b12 = a10 * predicted_p02 + a11 * predicted_p12 + a12 * predicted_p22
        b20 = a20 * predicted_p00 + a21 * predicted_p10 + a22 * predicted_p20
        b21 = a20 * predicted_p01 + a21 * predicted_p11 + a22 * predicted_p21
        b22 = a20 * predicted_p02 + a21 * predicted_p12 + a22 * predicted_p22
        return Update(
            x0=candidate_x0,
            x1=candidate_x1,
            x2=candidate_x2,
            p00=b00 * a00 + b01 * a01 + b02 * a02 + k0 * k0 * R,
            p01=b00 * a10 + b01 * a11 + b02 * a12 + k0 * k1 * R,
            p02=b00 * a20 + b01 * a21 + b02 * a22 + k0 * k2 * R,
            p10=b10 * a00 + b11 * a01 + b12 * a02 + k1 * k0 * R,
            p11=b10 * a10 + b11 * a11 + b12 * a12 + k1 * k1 * R,
            p12=b10 * a20 + b11 * a21 + b12 * a22 + k1 * k2 * R,
            p20=b20 * a00 + b21 * a01 + b22 * a02 + k2 * k0 * R,
            p21=b20 * a10 + b21 * a11 + b22 * a12 + k2 * k1 * R,
            p22=b20 * a20 + b21 * a21 + b22 * a22 + k2 * k2 * R,
        )

    @ti.func
    def valid(update):
        return (
            abs(update.x0) <= FINITE_LIMIT
            and abs(update.x1) <= FINITE_LIMIT
            and abs(update.x2) <= FINITE_LIMIT
            and abs(update.p00) <= FINITE_LIMIT
            and abs(update.p01) <= FINITE_LIMIT
            and abs(update.p02) <= FINITE_LIMIT
            and abs(update.p10) <= FINITE_LIMIT
            and abs(update.p11) <= FINITE_LIMIT
            and abs(update.p12) <= FINITE_LIMIT
            and abs(update.p20) <= FINITE_LIMIT
            and abs(update.p21) <= FINITE_LIMIT
            and abs(update.p22) <= FINITE_LIMIT
            and update.p00 > 0.0
            and update.p11 > 0.0
            and update.p22 > 0.0
            and abs(update.p01 - update.p10) <= SYMMETRY_TOLERANCE
            and abs(update.p02 - update.p20) <= SYMMETRY_TOLERANCE
            and abs(update.p12 - update.p21) <= SYMMETRY_TOLERANCE
        )

    @ti.func
    def store(update, group, i):
        if ti.static(args.arch == "metal"):
            state[group, 0, i] = update.x0
            state[group, 1, i] = update.x1
            state[group, 2, i] = update.x2
            state[group, 3, i] = update.p00
            state[group, 4, i] = update.p01
            state[group, 5, i] = update.p02
            state[group, 6, i] = update.p10
            state[group, 7, i] = update.p11
            state[group, 8, i] = update.p12
            state[group, 9, i] = update.p20
            state[group, 10, i] = update.p21
            state[group, 11, i] = update.p22
        else:
            state[i, 0, group] = update.x0
            state[i, 1, group] = update.x1
            state[i, 2, group] = update.x2
            state[i, 3, group] = update.p00
            state[i, 4, group] = update.p01
            state[i, 5, group] = update.p02
            state[i, 6, group] = update.p10
            state[i, 7, group] = update.p11
            state[i, 8, group] = update.p12
            state[i, 9, group] = update.p20
            state[i, 10, group] = update.p21
            state[i, 11, group] = update.p22

    @ti.kernel
    def step_checked(group: ti.i32):
        ti.loop_config(block_dim=args.block_dim)
        for i in range(instances):
            update = compute_update(i, group)
            if valid(update):
                store(update, 1 - group, i)
            else:
                ti.atomic_add(fault_count[None], 1)
                ti.atomic_min(fault_word[None], i * 256 + 1)

    @ti.kernel
    def step_unchecked(group: ti.i32):
        ti.loop_config(block_dim=args.block_dim)
        for i in range(instances):
            store(compute_update(i, group), 1 - group, i)

    input_values = make_inputs(instances, args.inject_fault)
    initial_values = initial_state(instances)

    def reset() -> None:
        if args.arch == "metal":
            inputs.from_numpy(input_values)
            state.from_numpy(np.stack((initial_values, initial_values)))
        else:
            inputs.from_numpy(input_values.T.copy())
            state.from_numpy(
                np.stack((initial_values, initial_values), axis=2).transpose(1, 0, 2)
            )
        fault_count[None] = 0
        fault_word[None] = NO_FAULT
        ti.sync()

    def dispatch(count: int) -> tuple[int, int, int]:
        group = 0
        faults = 0
        first_fault = NO_FAULT
        for _ in range(count):
            if checked:
                step_checked(group)
            else:
                step_unchecked(group)
            ti.sync()
            cumulative_faults = int(fault_count[None]) if checked else faults
            turn_faults = cumulative_faults - faults
            faults = cumulative_faults
            if checked and turn_faults:
                first_fault = min(first_fault, int(fault_word[None]))
            else:
                group = 1 - group
        return group, faults, first_fault

    reset()
    dispatch(5)
    reset()
    started = time.perf_counter()
    group, faults, first_fault = dispatch(turns)
    elapsed = time.perf_counter() - started
    state_values = state.to_numpy()
    if args.arch == "metal":
        checksum = float(state_values[group].astype(np.float64).sum())
    else:
        checksum = float(state_values[:, :, group].astype(np.float64).sum())

    print(f"implementation: Taichi {args.arch} matched")
    print(f"mode: {args.mode}")
    print(f"instances: {instances}")
    print(f"turns: {turns}")
    print(f"threadgroup_size: {args.block_dim}")
    print(f"cpu_threads: {max(1, args.cpu_threads) if args.arch == 'cpu' else 0}")
    print(f"elapsed_s: {elapsed:.9f}")
    print(f"throughput_million_ekf_turns_per_second: {instances * turns / elapsed / 1_000_000.0}")
    print(f"checksum: {checksum}")
    print(f"faults: {faults}")
    print(f"fault_word: {first_fault}")
    print(f"resident_bytes: {(2 * COMPONENTS + 3) * instances * 4 + 8}")


if __name__ == "__main__":
    main()
