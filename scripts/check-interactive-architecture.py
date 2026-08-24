#!/usr/bin/env python3
"""Cheap structural guards for the interactive/browser architecture."""

from __future__ import annotations

from pathlib import Path
import re
import sys


ROOT = Path(__file__).resolve().parents[1]


def fail(message: str) -> None:
    print(f"interactive architecture contract failed: {message}", file=sys.stderr)
    raise SystemExit(1)


def text(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


rust_sources = [
    path for path in ROOT.rglob("*.rs")
    if "target" not in path.parts and ".git" not in path.parts
]
for path in rust_sources:
    if "#![allow(warnings)]" in path.read_text(encoding="utf-8"):
        fail(f"crate-wide warning suppression remains in {path.relative_to(ROOT)}")

browser_sources = "\n".join(
    text(path)
    for path in (
        "include/document.js",
        "include/browser-compute.js",
        "include/project.js",
        "include/mech-repl.css",
        "include/style.css",
    )
)
legacy_contracts = (
    "data-tab",
    "data-panel",
    "#console-panel",
    "#output-panel",
    "#errors-panel",
    "#resizer",
    "#edgeHandle",
    "body.console-fullscreen",
    "is-collapsed",
)
for legacy in legacy_contracts:
    if legacy in browser_sources:
        fail(f"legacy browser component contract `{legacy}` reappeared")

obsolete_completion = (
    "acknowledgeComputeCommand",
    "rejectComputeCommand",
    "rejectIntegrityComputeCommand",
)
for operation in obsolete_completion:
    if operation in browser_sources or any(
        operation in path.read_text(encoding="utf-8")
        for path in (ROOT / "src/wasm/src").rglob("*.rs")
    ):
        fail(f"obsolete compute completion operation `{operation}` reappeared")

wasm_sources = "\n".join(
    path.read_text(encoding="utf-8")
    for path in (ROOT / "src/wasm/src").rglob("*.rs")
)
completion_names = {
    name for name in re.findall(r"js_name\s*=\s*(\w+)", wasm_sources)
    if name.endswith("ComputeCommand")
}
if completion_names != {"completeComputeCommand"}:
    fail(f"expected one completion ABI name, found {sorted(completion_names)}")

registry = text("src/compute/src/registry.rs")
trait_start = registry.index("pub trait ComputeSession")
trait_end = registry.index("\n}\n", trait_start)
compute_session = registry[trait_start:trait_end]
if "dispatch_requested" in registry or compute_session.count("fn dispatch(") != 1:
    fail("ComputeSession must expose exactly one request-based dispatch method")

selection = text("src/compute/src/registry.rs")
if not re.search(r"Samples\s*\{[^}]*instance:\s*u32", selection, re.DOTALL):
    fail("sample selection must name its instance explicitly")

for path in rust_sources:
    relative = path.relative_to(ROOT).as_posix()
    if relative == "hosts/gpu/src/execution_plan.rs":
        continue
    if re.search(r"GpuExecutionPlan\s*\{", path.read_text(encoding="utf-8")):
        fail(f"physical GPU plans may only be constructed in execution_plan.rs ({relative})")

gpu_and_wasm_sources = "\n".join(
    path.read_text(encoding="utf-8")
    for root in (ROOT / "hosts/gpu/src", ROOT / "src/wasm/src")
    for path in root.rglob("*.rs")
)
for mirror in ("GpuPlanBindingAccess", "GpuPlanBindingRole", "BrowserGpuProgram"):
    if mirror in gpu_and_wasm_sources:
        fail(f"transitional GPU plan mirror `{mirror}` reappeared")

project_source = text("src/wasm/src/project.rs")
for bootstrap_variant in (
    "DetachedDocumentBootstrap",
    "SourceBackedDocumentBootstrap",
    "WasmDocumentBootstrap::Detached",
    "WasmDocumentBootstrap::SourceBacked",
    "WasmDocumentBootstrap::Served",
):
    if bootstrap_variant in project_source:
        fail(f"parallel document bootstrap path `{bootstrap_variant}` reappeared")

if "prepare_browser_compute_host" in wasm_sources:
    fail("the transitional browser compute constructor reappeared")
if wasm_sources.count("fn prepare_browser_compute_runtime(") != 1:
    fail("browser compute must have exactly one prepared runtime constructor")

test_only_globals = (
    "__MECH_RENDERED_DOCUMENT_VALUE__",
    "__MECH_ACCEPTED_REPL_SOURCE__",
    "__MECH_REPLACE_ACCEPTED_REPL_SOURCE__",
    "__MECH_GPU_RUNTIME__",
    "__MECH_RUNTIME_INFO__",
    "__MECH_LAST_FRAME__",
    "__MECH_STOP__",
    "__MECH_COMPUTE_RESOURCE_SEQUENCE__",
    "__MECH_COMPUTE_PIPELINE_BUILD_COUNT__",
)
for name in test_only_globals:
    if name in browser_sources:
        fail(f"test-only production global `{name}` reappeared")

canonical_cdp_harness = ROOT / "tests/browser/harness/chrome.py"
for browser_test_root in (ROOT / "scripts", ROOT / "tests/browser"):
    for path in browser_test_root.rglob("*"):
        if (
            not path.is_file()
            or path.suffix not in {".js", ".mjs", ".py", ".sh"}
            or path in {Path(__file__).resolve(), canonical_cdp_harness}
        ):
            continue
        source = path.read_text(encoding="utf-8")
        if "websocket" in source.lower() or "sec-websocket" in source.lower():
            fail(f"scenario-specific CDP transport found in {path.relative_to(ROOT)}")

architecture = text("docs/architecture/interactive-runtime.md")
if len(architecture.splitlines()) > 250:
    fail("interactive runtime architecture record must remain concise")

obsolete_builders = (
    "scripts/build-mech-browser.sh",
    "scripts/build-mech-gpu-browser.sh",
    "scripts/build-mech-gpu-browser.ps1",
)
for builder in obsolete_builders:
    if (ROOT / builder).exists():
        fail(f"obsolete WASM build wrapper `{builder}` reappeared")
build_wasm = text("scripts/build-wasm.py")
for profile in ("browser", "browser-compute"):
    if f'"{profile}"' not in build_wasm:
        fail(f"unified WASM builder is missing the `{profile}` profile")

print("interactive architecture contract passed")
