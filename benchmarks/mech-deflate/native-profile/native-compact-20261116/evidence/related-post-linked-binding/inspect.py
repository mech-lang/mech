"""Collect same-object host audit evidence from the published Git tree."""

from collections import Counter
from hashlib import sha256
import json
from pathlib import Path
import re
import subprocess
import sys

repo = Path(sys.argv[1]).resolve()
head = "b72b7419b39b22765fe1eed6b5c636e63964b905"
pkg = "benchmarks/related-post-gen/"
gpu = "hosts/gpu/src/native_artifact/"


def git(*args):
    return subprocess.check_output(["git", *args], cwd=repo)


def read(name):
    return git("show", f"{head}:{name}")


def load(name):
    return json.loads(read(name))


def digest(data):
    return sha256(data).hexdigest()


manifest = load(pkg + "review/manifest.json")
inventory = load(pkg + "integration/source-inventory.json")
allowlist = {
    line.split("|", 1)[0]
    for line in read("scripts/unsafe-boundaries.allowlist").decode().splitlines()
    if line and not line.startswith("#")
}
unsafe_code = re.compile(
    r"unsafe\s*(?:\{|fn|impl|trait|extern)|#\[unsafe\s*\("
    r"|(?:^|[^\w])(?:transmute(?:_copy)?|from_raw|from_raw_parts(?:_mut)?"
    r"|ptr::(?:read|write|copy|copy_nonoverlapping)|NonNull::new_unchecked"
    r"|get_unchecked(?:_mut)?|unwrap_unchecked|unreachable_unchecked"
    r"|assume_init)\s*[:<(]|Value::as_unchecked|\.as_unchecked\s*[(<]"
    r"|(?:\*|as\s+\*)\s*(?:const|mut)\s+[\w:(<\[]"
)
roots = [pkg + "hosts", pkg + "measurement.rs", pkg + "review/qualification"]
paths = git("ls-tree", "-r", "--name-only", head, "--", *roots).decode().splitlines()

findings = {}
groups = Counter()
for name in paths:
    if not name.endswith(".rs") or name in allowlist:
        continue
    data = read(name)
    lines = [
        number
        for number, line in enumerate(data.decode().splitlines(), 1)
        if not line.lstrip().startswith(("//", "/*", "*")) and unsafe_code.search(line)
    ]
    if not lines:
        continue
    local = name.removeprefix(pkg)
    group = "/".join(local.split("/")[:3]) if local.startswith("review/") else local.split("/")[0]
    groups[group] += 1
    hashed = digest(data)
    findings[local] = {
        "lines": lines,
        "sha256": hashed,
        "review_manifest_pin": manifest["files"].get(local, {}).get("sha256") == hashed,
        "integration_inventory_pin": inventory["files"].get(name) == hashed,
    }

reviewed_host = "hosts/after-4.rs"
assert findings[reviewed_host]["lines"] == [104]
assert findings[reviewed_host]["review_manifest_pin"]
assert findings[reviewed_host]["integration_inventory_pin"]
host_text = read(pkg + reviewed_host).decode()
for attachment in ('include!("build/linked.rs")', 'include!("build/catalog.rs")', 'include_str!("build/source.mec")'):
    assert attachment in host_text

receipt_path = "review/qualification/after-4/runner-aot.build.json"
receipt_bytes = read(pkg + receipt_path)
assert digest(receipt_bytes) == manifest["files"][receipt_path]["sha256"]
receipt = json.loads(receipt_bytes)
assert receipt["source_sha256"] == findings[reviewed_host]["sha256"]
bundle = Path(receipt["bundle"]["object.o"]["path"]).parent
assert bundle == Path(receipt["source_path"]).parent / "build"
assert f"link-arg={bundle / 'object.o'}" in receipt["command"]

builder = read(pkg + "build-host.py").decode()
assert digest(builder.encode()) == manifest["files"]["build-host.py"]["sha256"]
assert 'args.bundle.resolve() / name' in builder
assert "args.bundle.resolve() / 'object.o'" in builder
assert "args.source.resolve()" in builder

report = {
    "review_head": head,
    "base_head": "9087b1f07727a9bfcb7c09059f48edaca6dc2301",
    "integration_head": "3a670730c8953f275fb59fcd7ac503d086995001",
    "hosted_architecture_job": {
        "run": 44080079325,
        "job": 134028828788,
        "conclusion": "failure",
        "path": pkg + reviewed_host,
        "line": 104,
    },
    "source_scan_roots": roots,
    "source_match_counts": dict(groups),
    "source_matches": findings,
    "historical_after_4": {
        "receipt": pkg + receipt_path,
        "receipt_sha256": digest(receipt_bytes),
        "source_path": receipt["source_path"],
        "host_sha256": receipt["source_sha256"],
        "object_path": receipt["bundle"]["object.o"]["path"],
        "object_sha256": receipt["bundle"]["object.o"]["sha256"],
        "linked_executable_sha256": receipt["executable_sha256"],
        "included_and_linked_roots_coincide_in_receipt": True,
    },
    "producer_proof_paths": {
        "current_host_builder": pkg + "build-host.py",
        "historical_producer": pkg + "review/recorded-recipes/compiler/compiler-facts.rs",
        "restore_contract": gpu + "program/linked.rs",
        "checked_generator_binding": gpu + "program/application.rs",
        "checked_generator_pins": gpu + "templates/linked_build.rs.in",
        "generated_launcher": gpu + "launch/managed.rs",
    },
}
print(json.dumps(report, indent=2))
