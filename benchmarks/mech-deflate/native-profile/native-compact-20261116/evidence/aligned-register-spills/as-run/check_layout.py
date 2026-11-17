"""Exercise the actual layout code and a stale profile through native emission."""

import json
import os
from pathlib import Path
import subprocess

study = Path(__file__).resolve().parent
q = study.parent
checkout = q / "native-review-pr/checkout"
candidate = q / "aligned-register-spills-native/final-candidate"
out = study / "layout-check"
out.mkdir()
crate = checkout / "hosts/gpu/src/native_artifact/program/compact/phase_layout.rs"
packet = checkout / "benchmarks/mech-deflate/native-profile/native-compact-20261116"
profile_path = packet / "profiles/mech_compact_numeric_475_vector-paired-spills.json"
libraries = {}
for line in (study / "cargo.jsonl").read_text().splitlines():
    if line.startswith("{"):
        record = json.loads(line)
        if record["reason"] == "compiler-artifact":
            for file in record["filenames"]:
                if file.endswith(".rlib"):
                    libraries[record["target"]["name"]] = Path(file)
deps = libraries["serde_json"].parent
source = out / "module.rs"
source.write_text(f"#[path = {json.dumps(str(crate))}]\nmod phase_layout;\n" + '''
#[test]
fn appends_whole_nops_at_the_object_boundary() {
    let directory = std::env::var_os("MECH_SPILL_LAYOUT_TEST_DIR").unwrap();
    let path = std::path::PathBuf::from(directory).join("test-profile.json");
    std::fs::write(&path, r#"{"order":[0],"object_tail":{"code_bytes":16,"reference_code_bytes":112,"alignment":64}}"#)
        .unwrap();
    let profiles = format!("local_leaf@{}", path.display());
    let mut code = vec![1; 16].into_boxed_slice();
    phase_layout::pad_object_tail(&profiles, "local_leaf", &mut code).unwrap();
    assert_eq!(&code[..16], &[1; 16]);
    assert_eq!(&code[16..], &0xd503_201f_u32.to_le_bytes().repeat(8));
    assert!(phase_layout::pad_object_tail(&profiles, "local_leaf", &mut code).is_err());

    let mut context = cranelift_codegen::Context::new();
    phase_layout::apply(&mut context, "local_leaf").unwrap();
    assert_eq!(context.func.aarch64_block_layout, [0]);
}
''')
test = out / "layout-tests"
command = ["rustc", "+nightly-2026-03-03", "--edition=2024", "--test", "-D", "warnings",
           "-L", f"dependency={deps}", str(source), "-o", str(test)]
for dependency in ("cranelift_codegen", "serde", "serde_json"):
    command.extend(("--extern", f"{dependency}={libraries[dependency]}"))
with (out / "unit-tests.log").open("w") as log:
    subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, check=True)
    subprocess.run([test, "--test-threads=1"], stdout=log, stderr=subprocess.STDOUT,
                   env=os.environ | {"MECH_SPILL_LAYOUT_TEST_DIR": str(out),
                                     "MECH_COMPACT_PHASE_BLOCK_LAYOUT": f"local_leaf@{out / 'test-profile.json'}"},
                   check=True)
profile = json.loads(profile_path.read_text())
stale = out / "stale-profile.json"
profile["object_tail"]["code_bytes"] += 4
stale.write_text(json.dumps(profile, indent=2) + "\n")
runtime = json.loads((candidate / "runtime.json").read_text())
env = {name: value for name, value in os.environ.items() if not name.startswith("MECH_")}
flags = dict(runtime["flags"])
var = "MECH_COMPACT_PHASE_BLOCK_LAYOUT"
assert str(profile_path) in flags[var]
flags[var] = flags[var].replace(str(profile_path), str(stale))
argv = [*runtime["emit"][:2], out / "rejected"]
(out / "rejected").mkdir()
with (out / "stale-emission.log").open("w") as log:
    result = subprocess.run(argv, stdout=log, stderr=subprocess.STDOUT,
                            env=env | flags | {"PATH": "/usr/bin:/bin"})
assert result.returncode != 0
assert "object tail padding profile does not match mech_compact_numeric_475_vector" in (out / "stale-emission.log").read_text()
assert not (out / "rejected/program.o").exists()
(out / "receipt.json").write_text(json.dumps({
    "unit_checks": 2,
    "stale_profile_returncode": result.returncode,
    "object_written": False,
    "commands": {"rustc": command, "direct_emission": list(map(str, argv))},
    "stale_flags": flags,
}, indent=2) + "\n")
