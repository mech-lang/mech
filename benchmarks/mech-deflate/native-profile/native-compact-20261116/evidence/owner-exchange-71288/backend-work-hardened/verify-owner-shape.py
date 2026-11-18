"""Run the exact owner retirement and exchange units with locked Cranelift libraries."""

import hashlib
import json
from pathlib import Path
import shutil
import subprocess

study = Path(__file__).resolve().parent
q = study.parent
compact = q / "native-review-pr/checkout/hosts/gpu/src/native_artifact/program/compact"
out = study / "owner-unit"
target = out / "native_artifact/program/compact"
target.mkdir(parents=True)
files = ("inlining.rs", "owner_retirement.rs", "owner_retirement/exchange.rs",
         "owner_retirement/exchange/exchange_tests.rs")
for name in files:
    file = target / name
    file.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(compact / name, file)
unit = out / "owner.rs"
unit.write_text("""#![allow(dead_code)]
mod native_artifact {
    pub type NativeCompileError = String;
    pub mod compiler {
        pub fn backend(error: impl std::fmt::Display) -> super::NativeCompileError {
            error.to_string()
        }
    }
    pub mod program {
        pub mod compact {
            mod inlining;
            mod owner_retirement;
        }
    }
}
""")
deps = q / "host-opt-levels/opt3-cargo/release/deps"
names = ("cranelift_codegen", "cranelift_frontend", "cranelift_module", "cranelift_jit")
crates = {}
for line in (study / "cargo.jsonl").read_text().splitlines():
    row = json.loads(line)
    if row["reason"] != "compiler-artifact":
        continue
    name = row["target"]["name"]
    if name in names:
        library, = [filename for filename in row["filenames"] if filename.endswith(".rlib")]
        crates[name] = library
binary = out / "tests"
command = ["rustc", "+nightly-2026-03-03", "--edition=2024", "--test", "-L", f"dependency={deps}",
           "-o", str(binary), str(unit)]
for name in names:
    command += ["--extern", f"{name}={crates[name]}"]
with (out / "build.log").open("wb") as log:
    subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, check=True)
test = [str(binary), "--test-threads=1"]
with (out / "test.log").open("wb") as log:
    subprocess.run(test, stdout=log, stderr=subprocess.STDOUT, check=True)
text = (out / "test.log").read_text()
assert "exchange_tests::preserves_nullable_exchange_order_overflow_and_checked_continuations ... ok" in text
record = {"build": command, "run": test, "sources": {name: hashlib.sha256((target / name).read_bytes()).hexdigest()
          for name in files}, "result": text}
(out / "result.json").write_text(json.dumps(record, indent=2) + "\n")
print(text, end="")
