#include <Halide.h>
#include <algorithm>
#include <array>
#include <chrono>
#include <cmath>
#include <cstdint>
#include <cstdlib>
#include <iomanip>
#include <iostream>
#include <string>
#include <vector>

using namespace Halide;
using A = std::array<Expr, 9>;

static A mm(const A &a, const A &b) {
    return {
        a[0] * b[0] + a[1] * b[3] + a[2] * b[6],
        a[0] * b[1] + a[1] * b[4] + a[2] * b[7],
        a[0] * b[2] + a[1] * b[5] + a[2] * b[8],
        a[3] * b[0] + a[4] * b[3] + a[5] * b[6],
        a[3] * b[1] + a[4] * b[4] + a[5] * b[7],
        a[3] * b[2] + a[4] * b[5] + a[5] * b[8],
        a[6] * b[0] + a[7] * b[3] + a[8] * b[6],
        a[6] * b[1] + a[7] * b[4] + a[8] * b[7],
        a[6] * b[2] + a[7] * b[5] + a[8] * b[8]
    };
}

static A tr(const A &a) {
    return {a[0], a[3], a[6], a[1], a[4], a[7], a[2], a[5], a[8]};
}

int main(int ac, char **av) try {
    int N = ac > 1 ? std::max(1, std::atoi(av[1])) : 10000;
    int K = ac > 2 ? std::max(1, std::atoi(av[2])) : 20;
    bool ck = ac > 3 && std::string(av[3]) == "checked";
    bool gpu = ac > 4 && std::string(av[4]) == "gpu";
    bool inject_fault = ac > 5 && std::string(av[5]) == "fault";
    Var i("i");

    ImageParam x0(Float(32), 1, "x0");
    ImageParam x1(Float(32), 1, "x1");
    ImageParam x2(Float(32), 1, "x2");
    ImageParam v(Float(32), 1, "v");
    ImageParam w(Float(32), 1, "w");
    ImageParam z(Float(32), 1, "z");
    std::array<ImageParam, 9> p = {
        ImageParam(Float(32), 1, "p0"),
        ImageParam(Float(32), 1, "p1"),
        ImageParam(Float(32), 1, "p2"),
        ImageParam(Float(32), 1, "p3"),
        ImageParam(Float(32), 1, "p4"),
        ImageParam(Float(32), 1, "p5"),
        ImageParam(Float(32), 1, "p6"),
        ImageParam(Float(32), 1, "p7"),
        ImageParam(Float(32), 1, "p8")
    };

    A P;
    std::array<Func, 9> pf;
    for (int j = 0; j < 9; j++) {
        pf[j] = Func("pf" + std::to_string(j));
        pf[j](i) = p[j](i);
        P[j] = pf[j](i);
    }

    Func thf("theta");
    Func snf("sin_theta");
    Func csf("cos_theta");
    Func x0f("x0_value");
    Func x1f("x1_value");
    Func wf("angular_velocity");
    thf(i) = x2(i);
    snf(i) = sin(thf(i));
    csf(i) = cos(thf(i));
    x0f(i) = x0(i);
    x1f(i) = x1(i);
    wf(i) = w(i);

    Expr th = thf(i);
    Expr sn = snf(i);
    Expr cs = csf(i);
    Expr d = v(i) * 0.1f;
    Expr X = x0f(i) + d * cs;
    Expr Y = x1f(i) + d * sn;
    Expr Z = th + wf(i) * 0.1f;
    A F = {1.f, 0.f, -d * sn, 0.f, 1.f, d * cs, 0.f, 0.f, 1.f};
    A U = mm(mm(F, P), tr(F));
    Expr j = cs * cs * 0.01f * 0.01f;
    Expr k = cs * sn * 0.01f * 0.01f;
    Expr l = sn * sn * 0.01f * 0.01f;
    Expr h = 0.01f * 0.0025f;
    A Q = {U[0] + j, U[1] + k, U[2], U[3] + k, U[4] + l, U[5], U[6], U[7], U[8] + h};
    Expr dx = 140.f - X;
    Expr dy = 12.f - Y;
    Expr rr = dx * dx + dy * dy;
    Expr raw = z(i) - (atan2(dy, dx) - Z);
    Expr nn = atan2(sin(raw), cos(raw));
    Expr h0 = dy / rr;
    Expr h1 = -dx / rr;
    Expr h2 = -1.f;
    Expr q0 = Q[0] * h0 + Q[1] * h1 + Q[2] * h2;
    Expr q1 = Q[3] * h0 + Q[4] * h1 + Q[5] * h2;
    Expr q2 = Q[6] * h0 + Q[7] * h1 + Q[8] * h2;
    Expr iv = h0 * q0 + h1 * q1 + h2 * q2 + 0.25f;
    Expr k0 = q0 / iv;
    Expr k1 = q1 / iv;
    Expr k2 = q2 / iv;
    A M = {
        1.f - k0 * h0, -k0 * h1, -k0 * h2,
        -k1 * h0, 1.f - k1 * h1, -k1 * h2,
        -k2 * h0, -k2 * h1, 1.f - k2 * h2
    };
    A C = mm(mm(M, Q), tr(M));
    A V = {
        C[0] + k0 * k0 * 0.25f,
        C[1] + k0 * k1 * 0.25f,
        C[2] + k0 * k2 * 0.25f,
        C[3] + k1 * k0 * 0.25f,
        C[4] + k1 * k1 * 0.25f,
        C[5] + k1 * k2 * 0.25f,
        C[6] + k2 * k0 * 0.25f,
        C[7] + k2 * k1 * 0.25f,
        C[8] + k2 * k2 * 0.25f
    };
    Expr n0 = X + k0 * nn;
    Expr n1 = Y + k1 * nn;
    Expr n2 = Z + k2 * nn;
    Expr finite_state = (abs(n0) <= 3.402823466e38f) &&
                        (abs(n1) <= 3.402823466e38f) &&
                        (abs(n2) <= 3.402823466e38f);
    Expr finite_cov = abs(V[0]) <= 3.402823466e38f;
    for (int j2 = 1; j2 < 9; j2++) {
        finite_cov = finite_cov && (abs(V[j2]) <= 3.402823466e38f);
    }
    Expr positive = (V[0] > 0.f) && (V[4] > 0.f) && (V[8] > 0.f);
    Expr symmetric = (abs(V[1] - V[3]) <= 0.0001f) &&
                     (abs(V[2] - V[6]) <= 0.0001f) &&
                     (abs(V[5] - V[7]) <= 0.0001f);
    Expr ok = finite_state && finite_cov && positive && symmetric;
    Expr fault_code = select(!finite_state, 1.f,
                             select(!finite_cov, 2.f,
                                    select(!positive, 3.f,
                                           select(!symmetric, 4.f, 0.f))));

    std::array<Expr, 12> candidate = {
        n0, n1, n2, V[0], V[1], V[2], V[3], V[4], V[5], V[6], V[7], V[8]
    };
    std::array<Expr, 12> previous = {
        x0f(i), x1f(i), thf(i), pf[0](i), pf[1](i), pf[2](i),
        pf[3](i), pf[4](i), pf[5](i), pf[6](i), pf[7](i), pf[8](i)
    };
    std::array<Func, 12> cf;
    for (int j2 = 0; j2 < 12; j2++) {
        cf[j2] = Func("candidate" + std::to_string(j2));
        cf[j2](i) = candidate[j2];
    }
    Func valid("valid");
    valid(i) = ok;
    std::vector<Expr> outputs;
    for (int j2 = 0; j2 < 12; j2++) {
        outputs.push_back(ck ? select(valid(i), cf[j2](i), previous[j2]) : cf[j2](i));
    }
    if (ck) {
        outputs.push_back(fault_code);
    }
    Func fused("ekf");
    fused(i) = Tuple(outputs);
    if (gpu) {
        Var block("block");
        Var thread("thread");
        for (auto &f : pf) {
            f.compute_at(fused, thread);
        }
        thf.compute_at(fused, thread);
        snf.compute_at(fused, thread);
        csf.compute_at(fused, thread);
        x0f.compute_at(fused, thread);
        x1f.compute_at(fused, thread);
        wf.compute_at(fused, thread);
        for (auto &f : cf) {
            f.compute_at(fused, thread);
        }
        valid.compute_at(fused, thread);
        fused.gpu_tile(i, block, thread, 256, TailStrategy::GuardWithIf);
    } else {
        fused.parallel(i).vectorize(i, 8, TailStrategy::GuardWithIf);
    }
    Pipeline pipe(fused);
    Target target = get_host_target();
    if (gpu) {
        target = target.with_feature(Target::Metal);
    }
    Callable callable;
    try {
        callable = pipe.compile_to_callable(pipe.infer_arguments(), target);
    } catch (const Halide::Error &e) {
        std::cerr << "compile_error: " << e.what() << "\n";
        return 2;
    }

    int output_count = 12 + (ck ? 1 : 0);
    std::vector<Buffer<float>> a;
    std::vector<Buffer<float>> b;
    for (int j2 = 0; j2 < output_count; j2++) {
        a.emplace_back(N);
        b.emplace_back(N);
    }
    Buffer<float> vb(N);
    Buffer<float> wb(N);
    Buffer<float> zb(N);
    for (int q3 = 0; q3 < N; q3++) {
        float ph = 2.f * float(M_PI) * q3 / N;
        vb(q3) = inject_fault && q3 == 0 ? INFINITY : 1.f + 0.05f * std::sin(3.f * ph);
        wb(q3) = 0.015f * (1.f + 0.1f * std::sin(2.f * ph));
        zb(q3) = -0.55f + 0.01f * std::sin(7.f * ph) + 0.005f * std::sin(11.f * ph);
        a[0](q3) = 55;
        a[1](q3) = 25;
        a[2](q3) = 0.4f;
        for (int j2 = 0; j2 < 9; j2++) {
            a[j2 + 3](q3) = j2 == 0 || j2 == 4 ? 100.f : j2 == 8 ? 0.15f : 0.f;
        }
    }
    if (ck) {
        a[12].fill(0.f);
    }

    uint64_t fault_lanes = 0;
    int first_fault_instance = -1;
    int first_fault_code = 0;
    auto turn = [&]() {
        try {
            if (ck) {
                callable(
                    a[3].raw_buffer(), a[4].raw_buffer(), a[5].raw_buffer(),
                    a[6].raw_buffer(), a[7].raw_buffer(), a[8].raw_buffer(),
                    a[9].raw_buffer(), a[10].raw_buffer(), a[11].raw_buffer(),
                    vb.raw_buffer(), wb.raw_buffer(), a[0].raw_buffer(),
                    a[1].raw_buffer(), a[2].raw_buffer(), zb.raw_buffer(),
                    b[0].raw_buffer(), b[1].raw_buffer(), b[2].raw_buffer(),
                    b[3].raw_buffer(), b[4].raw_buffer(), b[5].raw_buffer(),
                    b[6].raw_buffer(), b[7].raw_buffer(), b[8].raw_buffer(),
                    b[9].raw_buffer(), b[10].raw_buffer(), b[11].raw_buffer(),
                    b[12].raw_buffer());
            } else {
                callable(
                    a[3].raw_buffer(), a[4].raw_buffer(), a[5].raw_buffer(),
                    a[6].raw_buffer(), a[7].raw_buffer(), a[8].raw_buffer(),
                    a[9].raw_buffer(), a[10].raw_buffer(), a[11].raw_buffer(),
                    vb.raw_buffer(), wb.raw_buffer(), a[0].raw_buffer(),
                    a[1].raw_buffer(), a[2].raw_buffer(), zb.raw_buffer(),
                    b[0].raw_buffer(), b[1].raw_buffer(), b[2].raw_buffer(),
                    b[3].raw_buffer(), b[4].raw_buffer(), b[5].raw_buffer(),
                    b[6].raw_buffer(), b[7].raw_buffer(), b[8].raw_buffer(),
                    b[9].raw_buffer(), b[10].raw_buffer(), b[11].raw_buffer());
            }
            if (gpu && b[0].device_sync() != 0) {
                throw Halide::Error("device synchronization failed");
            }
            if (ck && gpu) {
                b[12].copy_to_host();
            }
            if (ck) {
                for (int q3 = 0; q3 < N; q3++) {
                    int code = int(b[12](q3));
                    if (code) {
                        fault_lanes++;
                        if (first_fault_instance < 0) {
                            first_fault_instance = q3;
                            first_fault_code = code;
                        }
                    }
                }
            }
        } catch (const Halide::Error &e) {
            std::cerr << "runtime_error: " << e.what() << "\n";
            std::exit(2);
        }
        std::swap(a, b);
    };
    for (int q3 = 0; q3 < 5; q3++) {
        for (int q4 = 0; q4 < K; q4++) {
            turn();
        }
    }
    if (gpu) {
        for (auto &q : a) {
            q.copy_to_host();
        }
        for (auto &q : b) {
            q.copy_to_host();
        }
    }
    a[0].fill(55);
    a[1].fill(25);
    a[2].fill(0.4f);
    for (int q3 = 0; q3 < N; q3++) {
        for (int j2 = 0; j2 < 9; j2++) {
            a[j2 + 3](q3) = j2 == 0 || j2 == 4 ? 100.f : j2 == 8 ? 0.15f : 0.f;
        }
    }
    if (ck) {
        a[12].fill(0.f);
    }
    fault_lanes = 0;
    first_fault_instance = -1;
    first_fault_code = 0;
    auto st = std::chrono::steady_clock::now();
    for (int q3 = 0; q3 < K; q3++) {
        turn();
    }
    double e = std::chrono::duration<double>(std::chrono::steady_clock::now() - st).count();
    if (gpu) {
        for (auto &q : a) {
            q.copy_to_host();
        }
    }
    double sum = 0;
    for (int j2 = 0; j2 < 12; j2++) {
        for (int q3 = 0; q3 < N; q3++) {
            sum += a[j2](q3);
        }
    }
    std::cout << std::fixed << std::setprecision(9);
    std::cout << "lane: Halide " << (gpu ? "GPU Metal " : "") << (ck ? "checked" : "unchecked") << "\n";
    std::cout << "instances: " << N << "\n";
    std::cout << "turns: " << K << "\n";
    std::cout << "elapsed_s: " << e << "\n";
    std::cout << "throughput: " << N * K / e << "\n";
    std::cout << "checksum: " << sum << "\n";
    std::cout << "validation: " << (ck ? "checked" : "unchecked") << "\n";
    std::cout << "backend: " << (gpu ? "metal" : "cpu") << "\n";
    if (ck) {
        std::cout << "fault_lanes: " << fault_lanes << "\n";
        std::cout << "first_fault_instance: " << first_fault_instance << "\n";
        std::cout << "first_fault_code: " << first_fault_code << "\n";
    }
    return 0;
} catch (const Halide::Error &e) {
    std::cerr << "halide_error: " << e.what() << "\n";
    return 2;
} catch (const std::exception &e) {
    std::cerr << "std_error: " << e.what() << "\n";
    return 2;
}
