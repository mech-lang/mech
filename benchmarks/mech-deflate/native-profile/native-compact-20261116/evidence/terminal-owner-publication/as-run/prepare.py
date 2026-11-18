"""Relink the same encoder with a single reference-count publication."""

from hashlib import file_digest
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

study = Path(__file__).resolve().parent
q = study.parent
reference = q / "native-size-20261117/source-aligned"
old_runtime = q / "scratch-runtime-backends/word-zero-pages"
source = study / "runtime"
shutil.copytree(old_runtime / "compact_runtime", source / "compact_runtime")
wrapper = (old_runtime / "runtime-lib.rs").read_text().replace(str(old_runtime), str(source))
(source / "runtime-lib.rs").write_text(wrapper)
leaf = source / "compact_runtime/terminal_appends.rs"
before = leaf.read_text()
canonical = q / "native-review-pr/checkout/hosts/gpu/src/native_artifact/compact_runtime/terminal_appends.rs"
assert before == canonical.read_text()
old = "            Ok(view) => (view[0] as *mut u8, view[5] as *mut super::Owner),"
new = """            Ok(view) => {
                let owner = view[5] as *mut super::Owner;
                // SAFETY: the proved nonempty allocation has a fresh owner.
                // The heap and backing stay private until the output write.
                unsafe { (*owner).references = 1 };
                (view[0] as *mut u8, owner)
            }"""
assert before.count(old) == 1
text = before.replace(old, new)
old = """    unsafe {
        (*owner).references = 1;
        output.write(result);
    }"""
assert text.count(old) == 1
leaf.write_text(text.replace(old, "    unsafe { output.write(result) };"))
(study / "original-terminal-appends.rs").write_text(before)


def sha(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


env = {key: value for key, value in os.environ.items() if not key.startswith("MECH_")}
runtime = source / "libmech_gpu_runtime.rlib"
compile_runtime = ["rustc", "+nightly-2026-03-03", "--edition=2024", "-D", "warnings",
                   "--crate-name", "mech_gpu", "--crate-type", "rlib", "-C", "opt-level=3",
                   str(source / "runtime-lib.rs"), "--remap-path-prefix", f"{source}={old_runtime}",
                   "-o", str(runtime)]
subprocess.run(compile_runtime, check=True, env=env)
pinned = json.loads((reference / "runtime.json").read_text())
baseline_lib = old_runtime / "libmech_gpu_runtime.rlib"
assert pinned["link"].count(f"mech_gpu={baseline_lib}") == 1
report = {"compile_runtime": compile_runtime, "pinned_runtime_sha256": sha(baseline_lib),
          "candidate_runtime_sha256": sha(runtime), "workers": {}}
subjects = {"terminal-owner-only": reference,
            "terminal-owner-cold-origins": q / "cold-origin-packed-native/final-candidate"}
for name, inputs in subjects.items():
    output = q / f"{name}-native/final-candidate"
    output.mkdir(parents=True)
    for file in ("program.o", "metadata.json"):
        shutil.copyfile(inputs / file, output / file)
    library = ["--extern", f"mech_gpu={runtime}"]
    link = [arg.replace(str(reference / "program.o"), str(output / "program.o"))
            .replace(str(reference / "mech-worker"), str(output / "mech-worker"))
            if arg != f"mech_gpu={baseline_lib}" else library[1]
            for arg in pinned["link"]]
    effective = json.loads((inputs / "runtime.json").read_text())
    (output / "runtime.json").write_text(json.dumps(effective | {
        "link": link, "candidate_runtime": {"sha256": sha(runtime), "compile": compile_runtime},
    }, indent=2) + "\n")
    subprocess.run(link, check=True, env=env)
    subprocess.run([sys.executable, "-B", q / "verify-worker.py", output / "mech-worker",
                    output / "metadata.json", name], check=True, env=env)
    report["workers"][name] = {file: sha(output / file) for file in ("program.o", "metadata.json", "mech-worker")}
(study / "worker-builds.json").write_text(json.dumps(report, indent=2) + "\n")
