"""Benchmark runner for Mojo — n-body and spectral-norm.

Mojo is a compiled language with Python-like syntax. These implementations
are direct translations of the official Benchmarks Game algorithms.

Build: uv run --prerelease=allow --extra-index-url https://whl.modular.com/nightly/simple/ --with "mojo>=0.26.3.0.dev0" -- mojo build -O3 -o /tmp/bench_mojo bench.mojo
Run:   /tmp/bench_mojo
"""

from std.time import perf_counter_ns

from nbody import run_nbody, DEFAULT_N as NBODY_N
from spectral_norm import run_spectral, DEFAULT_N as SPECTRAL_N


# ---------------------------------------------------------------------------
# Utilities
# ---------------------------------------------------------------------------

comptime RUNS = 5


def median(mut values: List[Float64]) -> Float64:
    """Simple selection sort to find median (only 5 elements)."""
    var n = len(values)
    for i in range(n):
        var min_idx = i
        for j in range(i + 1, n):
            if values[j] < values[min_idx]:
                min_idx = j
        if min_idx != i:
            var tmp = values[i]
            values[i] = values[min_idx]
            values[min_idx] = tmp
    return values[n // 2]


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


def main() raises:
    print("Mojo nightly (CPU, compiled with -O3)")
    print("Runs:", RUNS)
    print()

    # Warmup
    _ = run_nbody()
    _ = run_spectral()

    # N-body
    times_nb = List[Float64]()
    for _ in range(RUNS):
        t0 = perf_counter_ns()
        _ = run_nbody()
        t1 = perf_counter_ns()
        times_nb.append(Float64(t1 - t0) / 1_000_000.0)
    med_nb = median(times_nb)
    print(t"  n-body (500000 iterations)... {med_nb} ms")

    # Spectral-norm
    times_sn = List[Float64]()
    for _ in range(RUNS):
        t0 = perf_counter_ns()
        _ = run_spectral()
        t1 = perf_counter_ns()
        times_sn.append(Float64(t1 - t0) / 1_000_000.0)
    med_sn = median(times_sn)
    print(t"  spectral-norm (N=2000)...  {med_sn} ms")

    # Correctness
    result = run_nbody()
    sn = run_spectral()
    print()
    print("Correctness:")
    print(t"  nbody energy_before: {result["energy_before"]}")
    print(t"  nbody energy_after:  {result["energy_after"]}")
    print(t"  spectral_norm:       {sn}")
