#!/usr/bin/env python3
"""Isolate floating CDF costs without replacing admitted benchmark sources.

All generated Rust variants are local diagnostic experiments, not upstream
submissions. Compare the exact current Mech binary in the same shuffled rounds.
"""
from datetime import datetime, timezone
import hashlib
import importlib.util
import json
from pathlib import Path
import random
import shutil
import statistics
import subprocess

HERE = Path(__file__).resolve().parent
FASTA = HERE.parents[1]
CLBG = FASTA.parent
POWER = {"source": "AC Power", "low_power_mode": 0}
SAMPLES = 5


def sha(path):
    with Path(path).open("rb") as handle:
        return hashlib.file_digest(handle, "sha256").hexdigest()


def replace_once(source, old, new):
    assert source.count(old) == 1, old
    return source.replace(old, new, 1)


def main():
    destination = HERE / "results.json"
    assert not destination.exists(), "Keep the existing experiment; use a new directory for another run."
    rust6 = (FASTA / "upstream/fasta-rust-6.rs").read_text()
    rust7 = (FASTA / "upstream/fasta-rust-7.rs").read_text()
    rust6_count = replace_once(rust6, """        for &(c, p) in tab.iter() {
            if p > rand {
                res.push(c);
                break;
            }
        }""", """        // Diagnostic: same floating CDF, count crossed thresholds.
        let index = tab[..tab.len() - 1].iter()
            .filter(|&&(_, p)| rand >= p).count();
        res.push(tab[index].0);""")
    float7 = replace_once(rust7, "cumprob: Aligned<[u32; 16]>", "cumprob: Aligned<[f64; 16]>")
    float7 = replace_once(float7, "mapping: &[(T, f32)]", "mapping: &[(T, f64)]")
    float7 = replace_once(float7, "Aligned([i32::MAX as u32; 16])", "Aligned([f64::INFINITY; 16])")
    float7 = replace_once(float7, "(acc * IM as f32).floor() as u32", "acc")
    first = float7.index('    #[cfg(target_feature = "sse2")]')
    last = float7.index("\n}\n\nstruct MyRandom", first)
    float7 = float7[:first] + """    // Diagnostic: preserve the existing generator/thread/buffer pipeline.
    // Every nucleotide still performs floating division and CDF comparisons.
    fn gen_from_u32(&self, prob: u32) -> T {
        let random = prob as f64 / IM as f64;
        let mut count = 0usize;
        for p in self.cumprob.0.iter() {
            count += (random >= *p) as usize;
        }
        self.elements[count]
    }
""" + float7[last:]
    active7 = replace_once(float7, "elements: [T; 16],", "elements: [T; 16],\n    thresholds: usize,")
    active7 = replace_once(active7, "Self { elements, cumprob }", "Self { elements, cumprob, thresholds: mapping.len() - 1 }")
    active7 = replace_once(active7, "for p in self.cumprob.0.iter()", "for p in self.cumprob.0[..self.thresholds].iter()")
    sources = {
        "rust6-original": rust6,
        "rust6-floating-count": rust6_count,
        "rust7-original-integer": rust7,
        "rust7-floating-16": float7,
        "rust7-floating-active": active7,
    }
    spec = importlib.util.spec_from_file_location("fasta_guard", CLBG / "compact/run.py")
    guard = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(guard)
    rustc = shutil.which("rustc")
    report = {
        "status": "building", "created_at": datetime.now(timezone.utc).isoformat(),
        "scope": "Diagnostic only; neither the explorer nor admitted upstream sources are modified.",
        "workload": 25000000, "sample_count": SAMPLES, "required_power": POWER,
        "rustc": {"path": rustc, "sha256": sha(rustc),
                  "version": subprocess.check_output([rustc, "-vV"], text=True)},
        "protocol": "Fresh sequential processes, shuffled rounds, complete stdout to regular file; full output hashes checked after timing; AC normal power and compiler-load guards.",
        "programs": {}, "orders": [],
    }
    original = json.loads((CLBG / "retime/results/apple-m3-ac-normal-20261008.json").read_text())
    command = next(p["compile_command"] for p in original["programs"] if p["id"] == "fasta-rust-7")
    for name, source in sources.items():
        src = HERE / (name + ".rs")
        binary = HERE / name
        src.write_text(source)
        if name.startswith("rust7"):
            build = [rustc, *command[1:]]
            build[build.index("-o") - 1] = str(src)
            build[build.index("-o") + 1] = str(binary)
        else:
            build = [rustc, "--edition=2021", "-O", "-Ctarget-cpu=native", "-Ccodegen-units=1", str(src), "-o", str(binary)]
        subprocess.run(build, check=True, capture_output=True, text=True)
        report["programs"][name] = {"id": name, "benchmark": "fasta", "source": str(src),
            "source_sha256": sha(src), "binary_sha256": sha(binary), "build_command": build,
            "command": [str(binary), "{n}"], "samples_seconds": [], "sample_validation": []}
        print("BUILT", name, flush=True)
    mech = CLBG / "work-audit/20261009/builds/fasta-mech-fasta-final-expanded-v2-floating"
    assert sha(mech) == "37c24e617689fbbec6221ff4c107d9ea35f18eb62ed7387286bfa7d6224a92e4"
    report["programs"]["mech-current-floating"] = {
        "id": "mech-current-floating", "benchmark": "fasta", "binary_sha256": sha(mech),
        "command": [str(mech), "{n}"], "samples_seconds": [], "sample_validation": []}
    for row in report["programs"].values():
        check = guard.execute(row, False, 60)
        row["correctness_1000"] = check["validation"]
    report["status"] = "measuring"
    def save():
        destination.write_text(json.dumps(report, indent=2) + "\n")
    save()
    assert guard.power_profile() == POWER, "Need AC normal power for this comparison."
    rng = random.Random(20261009)
    for index in range(SAMPLES):
        order = list(report["programs"])
        rng.shuffle(order)
        report["orders"].append(order)
        for name in order:
            assert guard.power_profile() == POWER
            assert not guard.compiler_load(), "Competing compiler is active; do not measure."
            row = report["programs"][name]
            result = guard.execute(row, True, 60, watch_compilers=True, expected_power=POWER)
            assert not result["power_changes"] and not result["competing_compilers"], result
            row["samples_seconds"].append(result["seconds"])
            row["sample_validation"].append(result["validation"])
            print(index + 1, name, f'{result["seconds"]:.6f}', flush=True)
            save()
    report["status"] = "passed"
    report["finished_at"] = datetime.now(timezone.utc).isoformat()
    for name, row in report["programs"].items():
        row["median_seconds"] = statistics.median(row["samples_seconds"])
        row["sample_sd_seconds"] = statistics.stdev(row["samples_seconds"])
        print("RESULT", name, f'{row["median_seconds"]:.6f}', flush=True)
    save()


if __name__ == "__main__":
    main()
