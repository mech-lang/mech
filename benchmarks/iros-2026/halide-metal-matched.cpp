#include <Halide.h>

#include <algorithm>
#include <chrono>
#include <cmath>
#include <cstdlib>
#include <iomanip>
#include <iostream>
#include <string>
#include <vector>

using namespace Halide;

// Matched resident Metal EKF control for the IROS comparison. Both modes use
// the same packed component-major state, publication buffers, launch geometry,
// and per-turn synchronization. Halide's Metal backend cannot lower atomic
// reductions, so checked mode emits a resident per-lane fault plane, reads that
// plane after synchronization, and swaps publication buffers only when clear.

namespace {

constexpr int kComponents = 12;
constexpr int kStorageComponents = 13;
constexpr int kThreads = 256;

int argument(int argc, char **argv, int index, int fallback) {
    return argc > index ? std::max(1, std::atoi(argv[index])) : fallback;
}

void initialize(Buffer<float> &state, int instances) {
    const float values[kComponents] = {
        55.0f, 25.0f, 0.4f, 100.0f, 0.0f, 0.0f,
        0.0f, 100.0f, 0.0f, 0.0f, 0.0f, 0.15f,
    };
    for (int component = 0; component < kComponents; ++component) {
        for (int i = 0; i < instances; ++i) {
            state(i, component) = values[component];
        }
    }
    for (int i = 0; i < instances; ++i) {
        state(i, kComponents) = 0.0f;
    }
}

}  // namespace

int main(int argc, char **argv) try {
    const int instances = argument(argc, argv, 1, 500000);
    const int turns = argument(argc, argv, 2, 40);
    const std::string mode = argc > 3 ? argv[3] : "checked";
    if (mode != "checked" && mode != "unchecked") {
        std::cerr << "mode must be checked or unchecked\n";
        return 2;
    }
    const bool checked = mode == "checked";
    const bool inject_fault = argc > 4 && std::string(argv[4]) == "fault";
    const char *backend_environment = std::getenv("HALIDE_BACKEND");
    const std::string backend = backend_environment ? backend_environment : "metal";
    if (backend != "metal" && backend != "cpu") {
        std::cerr << "HALIDE_BACKEND must be metal or cpu\n";
        return 2;
    }
    const bool metal = backend == "metal";

    ImageParam state(Float(32), 2, "state");
    ImageParam velocity(Float(32), 1, "velocity");
    ImageParam angular_velocity(Float(32), 1, "angular_velocity");
    ImageParam bearing(Float(32), 1, "bearing");
    Var i("i"), component("component");

    const Expr sx0 = state(i, 0);
    const Expr sx1 = state(i, 1);
    const Expr sx2 = state(i, 2);
    const Expr sp00 = state(i, 3);
    const Expr sp01 = state(i, 4);
    const Expr sp02 = state(i, 5);
    const Expr sp10 = state(i, 6);
    const Expr sp11 = state(i, 7);
    const Expr sp12 = state(i, 8);
    const Expr sp20 = state(i, 9);
    const Expr sp21 = state(i, 10);
    const Expr sp22 = state(i, 11);

    const Expr st = sin(sx2);
    const Expr ct = cos(sx2);
    const Expr distance = velocity(i) * 0.1f;
    const Expr nx0 = sx0 + distance * ct;
    const Expr nx1 = sx1 + distance * st;
    const Expr nx2 = sx2 + angular_velocity(i) * 0.1f;
    const Expr f02 = -distance * st;
    const Expr f12 = distance * ct;

    const Expr ap0 = sp00 + f02 * sp20;
    const Expr ap1 = sp01 + f02 * sp21;
    const Expr ap2 = sp02 + f02 * sp22;
    const Expr aq0 = sp10 + f12 * sp20;
    const Expr aq1 = sp11 + f12 * sp21;
    const Expr aq2 = sp12 + f12 * sp22;
    const Expr a00 = ap0 + ap2 * f02 + ct * ct * 0.0001f;
    const Expr a01 = ap1 + ap2 * f12 + ct * st * 0.0001f;
    const Expr a02 = ap2;
    const Expr a10 = aq0 + aq2 * f02 + ct * st * 0.0001f;
    const Expr a11 = aq1 + aq2 * f12 + st * st * 0.0001f;
    const Expr a12 = aq2;
    const Expr a20 = sp20 + sp22 * f02;
    const Expr a21 = sp21 + sp22 * f12;
    const Expr a22 = sp22 + 0.000025f;

    const Expr dx = 140.0f - nx0;
    const Expr dy = 12.0f - nx1;
    const Expr rr = dx * dx + dy * dy;
    const Expr raw = bearing(i) - (atan2(dy, dx) - nx2);
    const Expr innovation = atan2(sin(raw), cos(raw));
    const Expr h0 = dy / rr;
    const Expr h1 = -dx / rr;
    const Expr h2 = -1.0f;
    const Expr ph0 = a00 * h0 + a01 * h1 + a02 * h2;
    const Expr ph1 = a10 * h0 + a11 * h1 + a12 * h2;
    const Expr ph2 = a20 * h0 + a21 * h1 + a22 * h2;
    const Expr variance = h0 * ph0 + h1 * ph1 + h2 * ph2 + 0.25f;
    const Expr k0 = ph0 / variance;
    const Expr k1 = ph1 / variance;
    const Expr k2 = ph2 / variance;
    const Expr b00 = 1.0f - k0 * h0;
    const Expr b01 = -k0 * h1;
    const Expr b02 = -k0 * h2;
    const Expr b10 = -k1 * h0;
    const Expr b11 = 1.0f - k1 * h1;
    const Expr b12 = -k1 * h2;
    const Expr b20 = -k2 * h0;
    const Expr b21 = -k2 * h1;
    const Expr b22 = 1.0f - k2 * h2;
    const Expr c00 = b00 * a00 + b01 * a10 + b02 * a20;
    const Expr c01 = b00 * a01 + b01 * a11 + b02 * a21;
    const Expr c02 = b00 * a02 + b01 * a12 + b02 * a22;
    const Expr c10 = b10 * a00 + b11 * a10 + b12 * a20;
    const Expr c11 = b10 * a01 + b11 * a11 + b12 * a21;
    const Expr c12 = b10 * a02 + b11 * a12 + b12 * a22;
    const Expr c20 = b20 * a00 + b21 * a10 + b22 * a20;
    const Expr c21 = b20 * a01 + b21 * a11 + b22 * a21;
    const Expr c22 = b20 * a02 + b21 * a12 + b22 * a22;

    std::vector<Expr> values = {
        nx0 + k0 * innovation,
        nx1 + k1 * innovation,
        nx2 + k2 * innovation,
        c00 * b00 + c01 * b01 + c02 * b02 + k0 * k0 * 0.25f,
        c00 * b10 + c01 * b11 + c02 * b12 + k0 * k1 * 0.25f,
        c00 * b20 + c01 * b21 + c02 * b22 + k0 * k2 * 0.25f,
        c10 * b00 + c11 * b01 + c12 * b02 + k1 * k0 * 0.25f,
        c10 * b10 + c11 * b11 + c12 * b12 + k1 * k1 * 0.25f,
        c10 * b20 + c11 * b21 + c12 * b22 + k1 * k2 * 0.25f,
        c20 * b00 + c21 * b01 + c22 * b02 + k2 * k0 * 0.25f,
        c20 * b10 + c21 * b11 + c22 * b12 + k2 * k1 * 0.25f,
        c20 * b20 + c21 * b21 + c22 * b22 + k2 * k2 * 0.25f,
    };

    Func candidate("candidate");
    candidate(i) = Tuple(values);
    std::vector<Expr> candidate_values;
    for (int index = 0; index < kComponents; ++index) {
        candidate_values.push_back(candidate(i)[index]);
    }
    const Expr finite =
        is_finite(candidate_values[0]) &&
        is_finite(candidate_values[1]) &&
        is_finite(candidate_values[2]) &&
        is_finite(candidate_values[3]) &&
        is_finite(candidate_values[4]) &&
        is_finite(candidate_values[5]) &&
        is_finite(candidate_values[6]) &&
        is_finite(candidate_values[7]) &&
        is_finite(candidate_values[8]) &&
        is_finite(candidate_values[9]) &&
        is_finite(candidate_values[10]) &&
        is_finite(candidate_values[11]);
    const Expr positive = candidate_values[3] > 0.0f &&
                          candidate_values[7] > 0.0f &&
                          candidate_values[11] > 0.0f;
    const Expr symmetric = abs(candidate_values[4] - candidate_values[6]) <= 0.0001f &&
                           abs(candidate_values[5] - candidate_values[9]) <= 0.0001f &&
                           abs(candidate_values[8] - candidate_values[10]) <= 0.0001f;
    const Expr zero_u32 = cast<uint32_t>(0);
    const Expr fault_code = select(!finite, cast<uint32_t>(1),
                                   select(!positive, cast<uint32_t>(2),
                                          select(!symmetric, cast<uint32_t>(3), zero_u32)));
    candidate_values.push_back(checked ? cast<float>(fault_code) : 0.0f);
    Func output("ekf");
    output(i, component) = mux(component, candidate_values);

    Var block("block"), thread("thread");
    output.bound(component, 0, kStorageComponents)
        .reorder(component, i)
        .unroll(component);
    if (metal) {
        output.gpu_tile(i, block, thread, kThreads, TailStrategy::GuardWithIf);
        candidate.compute_at(output, thread);
    } else {
        output.parallel(i).vectorize(i, 8, TailStrategy::GuardWithIf);
        candidate.compute_at(output, i);
    }

    Pipeline pipeline(output);
    Target target = get_host_target();
    if (metal) {
        target = target.with_feature(Target::Metal);
    }
    const std::vector<Argument> arguments = pipeline.infer_arguments();
    Callable callable = pipeline.compile_to_callable(arguments, target);

    Buffer<float> states[2] = {
        Buffer<float>(instances, kStorageComponents),
        Buffer<float>(instances, kStorageComponents),
    };
    Buffer<float> velocity_data(instances);
    Buffer<float> angular_velocity_data(instances);
    Buffer<float> bearing_data(instances);
    uint32_t fault_count = 0;
    uint32_t first_fault = UINT32_MAX;
    for (int lane = 0; lane < instances; ++lane) {
        const float phase = 2.0f * float(M_PI) * lane / instances;
        velocity_data(lane) = inject_fault && lane == 0
                                  ? INFINITY
                                  : 1.0f + 0.05f * std::sin(phase * 3.0f);
        angular_velocity_data(lane) = 0.015f * (1.0f + 0.1f * std::sin(phase * 2.0f));
        bearing_data(lane) = -0.55f + 0.01f * std::sin(phase * 7.0f) +
                             0.005f * std::sin(phase * 11.0f);
    }
    initialize(states[0], instances);
    initialize(states[1], instances);

    auto dispatch = [&](int source) {
        const int result = callable(
            angular_velocity_data.raw_buffer(), bearing_data.raw_buffer(),
            states[source].raw_buffer(), velocity_data.raw_buffer(),
            states[1 - source].raw_buffer());
        if (result != 0 || (metal && states[1 - source].device_sync() != 0)) {
            throw Error("Halide Metal dispatch failed");
        }
        if (checked) {
            Buffer<float> fault_plane = states[1 - source].sliced(1, kComponents);
            fault_plane.copy_to_host();
            fault_count = 0;
            first_fault = UINT32_MAX;
            for (int lane = 0; lane < instances; ++lane) {
                const uint32_t code = static_cast<uint32_t>(fault_plane(lane));
                if (code != 0) {
                    ++fault_count;
                    first_fault = std::min(first_fault,
                                           (static_cast<uint32_t>(lane) << 8) | code);
                }
            }
        }
        return !checked || fault_count == 0;
    };

    int source = 0;
    for (int turn = 0; turn < 5; ++turn) {
        if (dispatch(source)) {
            source = 1 - source;
        }
    }
    states[0].copy_to_host();
    states[1].copy_to_host();
    initialize(states[0], instances);
    initialize(states[1], instances);
    if (metal &&
        (states[0].copy_to_device(DeviceAPI::Metal, target) != 0 ||
         states[1].copy_to_device(DeviceAPI::Metal, target) != 0 ||
         states[0].device_sync() != 0 || states[1].device_sync() != 0)) {
        throw Error("Halide Metal reset upload failed");
    }
    source = 0;

    const auto started = std::chrono::steady_clock::now();
    for (int turn = 0; turn < turns; ++turn) {
        if (dispatch(source)) {
            source = 1 - source;
        }
    }
    const double elapsed = std::chrono::duration<double>(
                               std::chrono::steady_clock::now() - started)
                               .count();
    states[source].copy_to_host();
    double checksum = 0.0;
    for (int component_index = 0; component_index < kComponents; ++component_index) {
        for (int lane = 0; lane < instances; ++lane) {
            checksum += states[source](lane, component_index);
        }
    }

    std::cout << std::fixed << std::setprecision(9);
    std::cout << "lane: Halide " << backend << " matched packed SoA\n";
    std::cout << "instances: " << instances << "\n";
    std::cout << "turns: " << turns << "\n";
    std::cout << "elapsed_s: " << elapsed << "\n";
    std::cout << "throughput: " << instances * turns / elapsed << "\n";
    std::cout << "checksum: " << checksum << "\n";
    std::cout << "validation: " << mode << "\n";
    std::cout << "faults: " << (checked ? fault_count : 0) << "\n";
    std::cout << "fault_word: " << (checked ? first_fault : UINT32_MAX) << "\n";
    std::cout << "threadgroup_size: " << (metal ? kThreads : 0) << "\n";
    return 0;
} catch (const Halide::Error &error) {
    std::cerr << "halide_error: " << error.what() << "\n";
    return 2;
} catch (const std::exception &error) {
    std::cerr << "std_error: " << error.what() << "\n";
    return 2;
}
