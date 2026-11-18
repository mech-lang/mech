"""Build the original verifier with the current release dependencies."""

from hashlib import file_digest
import json
import os
from pathlib import Path
import shutil
import subprocess

study = Path(__file__).resolve().parent
q = study.parent
checkout = q / "native-review-pr/checkout"
target = q / "host-opt-levels/opt3-cargo/release"
original = study / "before/vendor/cranelift-codegen"
out = study / "verifier-control"
out.mkdir()
rows = [json.loads(line) for line in (study / "cargo.jsonl").read_text().splitlines() if line.startswith("{")]
artifact, = [row for row in rows if row.get("target",{}).get("name") == "cranelift_codegen"]
generated, = [row for row in rows if row["reason"] == "build-script-executed"
              and row["package_id"] == artifact["package_id"]]
current, = [Path(path) for path in artifact["filenames"] if path.endswith(".rlib")]
compiler_id = current.stem.rsplit("-", 1)[1]
fingerprints = target / ".fingerprint"
fingerprint = json.loads((fingerprints / f"cranelift-codegen-{compiler_id}/lib-cranelift_codegen.json").read_text())
syntax = out / "source"
shutil.copytree(checkout / "vendor/cranelift-codegen/src", syntax)
module = Path("src/verifier/mod.rs")
shutil.copyfile(original / module, syntax / "verifier/mod.rs")
manifest = json.loads((study / "baseline.json").read_text())
repo_path = Path("vendor/cranelift-codegen") / module
old = subprocess.check_output(["git","show",f"{manifest['source']}:{repo_path}"], cwd=q.parent)
assert (syntax / "verifier/mod.rs").read_bytes() == old
library = out / "libcranelift_codegen.rlib"
command = ["rustc","+nightly-2026-03-03","--edition=2024","--crate-type","rlib",
           "--crate-name","cranelift_codegen","-C","opt-level=3","-C","codegen-units=16",
           "-C","embed-bitcode=no","-C","metadata=native-work-original-verifier",
           "-L",f"dependency={target/'deps'}",*fingerprint["rustflags"]]
for name in artifact["features"]:
    command.extend(("--cfg",f'feature="{name}"'))
for config in generated["cfgs"]:
    command.extend(("--cfg", config))
libraries = {entry.read_text().strip():entry for entry in fingerprints.glob("*/lib-*") if entry.suffix != ".json"}
for _, name, _, code in fingerprint["deps"]:
    if name == "build_script_build":
        continue
    pin = libraries[code.to_bytes(8,"little").hex()]
    dep_key = pin.parent.name.rsplit("-",1)[1]
    crate = pin.name.removeprefix("lib-")
    filename, = (f for ext in ("rlib","dylib") if (f:=target/"deps"/f"lib{crate}-{dep_key}.{ext}").is_file())
    command.extend(("--extern",f"{name}={filename}"))
command.extend((str(syntax / "lib.rs"),"-o",str(library)))
overrides = dict(generated["env"]) | {"OUT_DIR":generated["out_dir"],"CARGO_PKG_VERSION":"0.131.3"}
with (out / "build.log").open("wb") as log:
    subprocess.run(command, env=os.environ | overrides, stdout=log, stderr=subprocess.STDOUT, check=True)
with library.open("rb") as f:
    digest=file_digest(f,"sha256").hexdigest()
(out / "build.json").write_text(json.dumps({
    "source_commit":manifest["source"],"command":command,"generated_environment":overrides,
    "library_sha256":digest,
}, indent=2)+"\n")
