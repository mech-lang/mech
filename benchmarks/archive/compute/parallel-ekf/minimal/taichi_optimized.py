import argparse
import os
import time
import numpy as np
import taichi as ti
DT = np.float32(0.1)
DT2 = np.float32(0.01)
R = np.float32(0.25)
FINITE_LIMIT = np.float32(3.402823466e38)
SYMMETRY_TOLERANCE = np.float32(0.0001)
def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("instances", nargs="?", type=int, default=100_000)
    parser.add_argument("turns", nargs="?", type=int, default=5)
    parser.add_argument("mode", nargs="?", choices=("checked", "unchecked", "unchecked-batched"), default="unchecked")
    parser.add_argument("--cpu-threads", type=int)
    parser.add_argument("--block-dim", type=int, default=int(os.environ.get("TAICHI_BLOCK_DIM", "32")))
    return parser.parse_args()
def arch_from_environment():
    name = os.environ.get("TAICHI_ARCH", "gpu").lower()
    architectures = {"cpu": ti.cpu, "gpu": ti.gpu, "cuda": ti.cuda, "metal": ti.metal, "vulkan": ti.vulkan, "opengl": ti.opengl}
    try:
        return architectures[name]
    except KeyError as error:
        raise SystemExit(f"unsupported TAICHI_ARCH={name!r}") from error
def make_inputs(instances: int) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    index = np.arange(instances, dtype=np.float32)
    phase = np.float32(2.0 * np.pi) * index / np.float32(instances)
    velocity = np.float32(1.0) + np.float32(0.05) * np.sin(phase * np.float32(3.0)).astype(np.float32)
    angular_velocity = np.float32(0.015) * (np.float32(1.0) + np.float32(0.1) * np.sin(phase * np.float32(2.0)).astype(np.float32))
    bearing = np.float32(-0.55) + np.float32(0.01) * np.sin(phase * np.float32(7.0)).astype(np.float32) + np.float32(0.005) * np.sin(phase * np.float32(11.0)).astype(np.float32)
    return velocity, angular_velocity, bearing
def initial_state(instances: int) -> tuple[np.ndarray, ...]:
    values = [np.full(instances, 0.0, dtype=np.float32) for _ in range(12)]
    values[0].fill(55.0)
    values[1].fill(25.0)
    values[2].fill(0.4)
    values[3].fill(100.0)
    values[7].fill(100.0)
    values[11].fill(0.15)
    return tuple(values)
def main() -> None:
    args = parse_args()
    if args.block_dim < 1 or args.block_dim & (args.block_dim - 1):
        raise SystemExit("--block-dim must be a positive power of two")
    instances = max(1, args.instances)
    turns = max(1, args.turns)
    checked = args.mode == "checked"
    batched = args.mode == "unchecked-batched"
    architecture = arch_from_environment()
    init_kwargs = {
        "arch": architecture,
        "default_fp": ti.f32,
        "fast_math": False,
        "kernel_profiler": False,
    }
    if architecture == ti.cpu and args.cpu_threads is not None:
        if args.cpu_threads < 1:
            raise SystemExit("--cpu-threads must be at least 1")
        init_kwargs["cpu_max_num_threads"] = args.cpu_threads
    ti.init(**init_kwargs)
    def field() -> ti.Field:
        return ti.field(dtype=ti.f32, shape=instances)
    velocity, angular_velocity, bearing = field(), field(), field()
    x0, x1, x2, p00, p01, p02, p10, p11, p12, p20, p21, p22 = (field() for _ in range(12))
    a0, a1, a2, ap00, ap01, ap02, ap10, ap11, ap12, ap20, ap21, ap22 = (field() for _ in range(12))
    b0, b1, b2, bp00, bp01, bp02, bp10, bp11, bp12, bp20, bp21, bp22 = (field() for _ in range(12))
    fault_count = ti.field(dtype=ti.i32, shape=())
    fault_instance = ti.field(dtype=ti.i32, shape=())
    Update = ti.types.struct(
        x0=ti.f32, x1=ti.f32, x2=ti.f32,
        p00=ti.f32, p01=ti.f32, p02=ti.f32,
        p10=ti.f32, p11=ti.f32, p12=ti.f32,
        p20=ti.f32, p21=ti.f32, p22=ti.f32,
    )
    @ti.func
    def compute_update(i, sx0, sx1, sx2, sp00, sp01, sp02, sp10, sp11, sp12, sp20, sp21, sp22):
        st = ti.sin(sx2)
        ct = ti.cos(sx2)
        distance = velocity[i] * DT
        predicted_x0 = sx0 + distance * ct
        predicted_x1 = sx1 + distance * st
        predicted_x2 = sx2 + angular_velocity[i] * DT
        f02 = -distance * st
        f12 = distance * ct
        ap0 = sp00 + f02 * sp20
        ap1 = sp01 + f02 * sp21
        ap2 = sp02 + f02 * sp22
        aq0 = sp10 + f12 * sp20
        aq1 = sp11 + f12 * sp21
        aq2 = sp12 + f12 * sp22
        process00 = ct * ct * DT2 * ti.f32(0.01)
        process01 = ct * st * DT2 * ti.f32(0.01)
        process11 = st * st * DT2 * ti.f32(0.01)
        process22 = DT2 * ti.f32(0.0025)
        predicted_p00 = ap0 + ap2 * f02 + process00
        predicted_p01 = ap1 + ap2 * f12 + process01
        predicted_p02 = ap2
        predicted_p10 = aq0 + aq2 * f02 + process01
        predicted_p11 = aq1 + aq2 * f12 + process11
        predicted_p12 = aq2
        predicted_p20 = sp20 + sp22 * f02
        predicted_p21 = sp21 + sp22 * f12
        predicted_p22 = sp22 + process22
        dx = ti.f32(140.0) - predicted_x0
        dy = ti.f32(12.0) - predicted_x1
        squared_range = dx * dx + dy * dy
        predicted_bearing = ti.atan2(dy, dx) - predicted_x2
        raw_innovation = bearing[i] - predicted_bearing
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
            x0=candidate_x0, x1=candidate_x1, x2=candidate_x2,
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
            abs(update.x0) <= FINITE_LIMIT and abs(update.x1) <= FINITE_LIMIT and abs(update.x2) <= FINITE_LIMIT
            and abs(update.p00) <= FINITE_LIMIT and abs(update.p01) <= FINITE_LIMIT and abs(update.p02) <= FINITE_LIMIT
            and abs(update.p10) <= FINITE_LIMIT and abs(update.p11) <= FINITE_LIMIT and abs(update.p12) <= FINITE_LIMIT
            and abs(update.p20) <= FINITE_LIMIT and abs(update.p21) <= FINITE_LIMIT and abs(update.p22) <= FINITE_LIMIT
            and update.p00 > 0.0 and update.p11 > 0.0 and update.p22 > 0.0
            and abs(update.p01 - update.p10) <= SYMMETRY_TOLERANCE
            and abs(update.p02 - update.p20) <= SYMMETRY_TOLERANCE
            and abs(update.p12 - update.p21) <= SYMMETRY_TOLERANCE
        )
    def store(update, out):
        out[0] = update.x0
        out[1] = update.x1
        out[2] = update.x2
        out[3] = update.p00
        out[4] = update.p01
        out[5] = update.p02
        out[6] = update.p10
        out[7] = update.p11
        out[8] = update.p12
        out[9] = update.p20
        out[10] = update.p21
        out[11] = update.p22
    @ti.kernel
    def step_unchecked():
        ti.loop_config(block_dim=args.block_dim)
        for i in range(instances):
            update = compute_update(i, x0[i], x1[i], x2[i], p00[i], p01[i], p02[i], p10[i], p11[i], p12[i], p20[i], p21[i], p22[i])
            x0[i], x1[i], x2[i] = update.x0, update.x1, update.x2
            p00[i], p01[i], p02[i] = update.p00, update.p01, update.p02
            p10[i], p11[i], p12[i] = update.p10, update.p11, update.p12
            p20[i], p21[i], p22[i] = update.p20, update.p21, update.p22
    @ti.kernel
    def step_unchecked_batched(count: ti.i32):
        ti.loop_config(block_dim=args.block_dim)
        for i in range(instances):
            sx0, sx1, sx2 = x0[i], x1[i], x2[i]
            sp00, sp01, sp02 = p00[i], p01[i], p02[i]
            sp10, sp11, sp12 = p10[i], p11[i], p12[i]
            sp20, sp21, sp22 = p20[i], p21[i], p22[i]
            for _ in range(count):
                update = compute_update(i, sx0, sx1, sx2, sp00, sp01, sp02, sp10, sp11, sp12, sp20, sp21, sp22)
                sx0, sx1, sx2 = update.x0, update.x1, update.x2
                sp00, sp01, sp02 = update.p00, update.p01, update.p02
                sp10, sp11, sp12 = update.p10, update.p11, update.p12
                sp20, sp21, sp22 = update.p20, update.p21, update.p22
            x0[i], x1[i], x2[i] = sx0, sx1, sx2
            p00[i], p01[i], p02[i] = sp00, sp01, sp02
            p10[i], p11[i], p12[i] = sp10, sp11, sp12
            p20[i], p21[i], p22[i] = sp20, sp21, sp22
    @ti.kernel
    def step_checked(group: ti.i32):
        ti.loop_config(block_dim=args.block_dim)
        for i in range(instances):
            if group == 0:
                update = compute_update(i, a0[i], a1[i], a2[i], ap00[i], ap01[i], ap02[i], ap10[i], ap11[i], ap12[i], ap20[i], ap21[i], ap22[i])
                if valid(update):
                    b0[i], b1[i], b2[i] = update.x0, update.x1, update.x2
                    bp00[i], bp01[i], bp02[i] = update.p00, update.p01, update.p02
                    bp10[i], bp11[i], bp12[i] = update.p10, update.p11, update.p12
                    bp20[i], bp21[i], bp22[i] = update.p20, update.p21, update.p22
                else:
                    ti.atomic_add(fault_count[None], 1)
                    ti.atomic_min(fault_instance[None], i)
            else:
                update = compute_update(i, b0[i], b1[i], b2[i], bp00[i], bp01[i], bp02[i], bp10[i], bp11[i], bp12[i], bp20[i], bp21[i], bp22[i])
                if valid(update):
                    a0[i], a1[i], a2[i] = update.x0, update.x1, update.x2
                    ap00[i], ap01[i], ap02[i] = update.p00, update.p01, update.p02
                    ap10[i], ap11[i], ap12[i] = update.p10, update.p11, update.p12
                    ap20[i], ap21[i], ap22[i] = update.p20, update.p21, update.p22
                else:
                    ti.atomic_add(fault_count[None], 1)
                    ti.atomic_min(fault_instance[None], i)
    arrays = (x0, x1, x2, p00, p01, p02, p10, p11, p12, p20, p21, p22)
    checked_arrays = (a0, a1, a2, ap00, ap01, ap02, ap10, ap11, ap12, ap20, ap21, ap22, b0, b1, b2, bp00, bp01, bp02, bp10, bp11, bp12, bp20, bp21, bp22)
    velocity_values, angular_values, bearing_values = make_inputs(instances)
    velocity.from_numpy(velocity_values)
    angular_velocity.from_numpy(angular_values)
    bearing.from_numpy(bearing_values)
    initial = initial_state(instances)
    def reset() -> None:
        for target in arrays:
            target.from_numpy(initial[arrays.index(target)])
        for offset, target in enumerate(checked_arrays):
            target.from_numpy(initial[offset % 12])
        fault_count[None] = 0
        fault_instance[None] = instances
        ti.sync()
    def dispatch(count: int) -> tuple[int, int]:
        if batched:
            step_unchecked_batched(count)
            ti.sync()
            return 0, 0
        faults = 0
        group = 0
        for _ in range(count):
            if checked:
                fault_count[None] = 0
                fault_instance[None] = instances
                step_checked(group)
            else:
                step_unchecked()
            ti.sync()
            if checked:
                faults += int(fault_count[None])
                if fault_count[None]:
                    break
                group = 1 - group
        return faults, group
    reset()
    dispatch(5)
    reset()
    started = time.perf_counter()
    faults, group = dispatch(turns)
    elapsed = time.perf_counter() - started
    if checked:
        source = (a0, a1, a2, ap00, ap01, ap02, ap10, ap11, ap12, ap20, ap21, ap22) if group == 0 else (b0, b1, b2, bp00, bp01, bp02, bp10, bp11, bp12, bp20, bp21, bp22)
    else:
        source = arrays
    checksum = sum(float(target.to_numpy().astype(np.float64).sum()) for target in source)
    print("lane: Taichi optimized scalar-SoA resident")
    print(f"instances: {instances}")
    print(f"turns: {turns}")
    print(f"elapsed_s: {elapsed:.9f}")
    print(f"throughput: {instances * turns / elapsed:.3f}")
    print(f"checksum: {checksum:.9f}")
    print(f"validation: {'checked' if checked else 'unchecked'}")
    print(f"faults: {faults}")
    print(f"synchronization: {'once after batched kernel' if batched else 'per-turn'}")
    print(f"block_dim: {args.block_dim}")
    if architecture == ti.cpu:
        print(f"cpu_threads: {args.cpu_threads or 'taichi-default'}")
if __name__ == "__main__":
    main()
