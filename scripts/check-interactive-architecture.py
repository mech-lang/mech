#!/usr/bin/env python3
"""Small structural guards for the interactive/browser architecture."""

from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[1]


def fail(message: str) -> None:
    print(f"interactive architecture contract failed: {message}", file=sys.stderr)
    raise SystemExit(1)


def text(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


rust_sources = [path for path in ROOT.rglob("*.rs")
                if "target" not in path.parts and ".git" not in path.parts]
for path in rust_sources:
    if "#![allow(warnings)]" in path.read_text(encoding="utf-8"):
        fail(f"crate-wide warning suppression remains in {path.relative_to(ROOT)}")

browser_sources = "\n".join(text(path) for path in (
    "include/document.js", "include/browser-compute.js", "include/project.js",
    "include/mech-repl.css", "include/style.css",
))
for legacy in (
    "data-tab", "data-panel", "#console-panel", "#output-panel", "#errors-panel",
    "#resizer", "#edgeHandle", "body.console-fullscreen", "is-collapsed",
):
    if legacy in browser_sources:
        fail(f"legacy browser component contract `{legacy}` reappeared")

wasm_sources = "\n".join(path.read_text(encoding="utf-8")
                         for path in (ROOT / "src/wasm/src").rglob("*.rs"))
for operation in (
    "acknowledgeComputeCommand", "rejectComputeCommand",
    "rejectIntegrityComputeCommand",
):
    if operation in browser_sources or operation in wasm_sources:
        fail(f"obsolete compute completion operation `{operation}` reappeared")
completion_names = {
    name for name in re.findall(r"js_name\s*=\s*(\w+)", wasm_sources)
    if name.endswith("ComputeCommand")
}
if completion_names != {"completeComputeCommand"}:
    fail(f"expected one completion ABI name, found {sorted(completion_names)}")

registry = text("src/compute/src/registry.rs")
trait_start = registry.index("pub trait ComputeSession")
trait_end = registry.index("\n}\n", trait_start)
if "dispatch_requested" in registry or registry[trait_start:trait_end].count("fn dispatch(") != 1:
    fail("ComputeSession must expose exactly one request-based dispatch method")
if not re.search(r"Samples\s*\{[^}]*instance:\s*u32", registry, re.DOTALL):
    fail("sample selection must name its instance explicitly")

for path in rust_sources:
    relative = path.relative_to(ROOT).as_posix()
    if relative != "hosts/gpu/src/execution_plan.rs" and re.search(
        r"GpuExecutionPlan\s*\{", path.read_text(encoding="utf-8")
    ):
        fail(f"physical GPU plans may only be constructed in execution_plan.rs ({relative})")
gpu_and_wasm_sources = "\n".join(path.read_text(encoding="utf-8")
    for root in (ROOT / "hosts/gpu/src", ROOT / "src/wasm/src")
    for path in root.rglob("*.rs"))
for mirror in ("GpuPlanBindingAccess", "GpuPlanBindingRole", "BrowserGpuProgram"):
    if mirror in gpu_and_wasm_sources:
        fail(f"transitional GPU plan mirror `{mirror}` reappeared")

project_source = text("src/wasm/src/project.rs")
for bootstrap_variant in (
    "DetachedDocumentBootstrap", "SourceBackedDocumentBootstrap",
    "WasmDocumentBootstrap::Detached", "WasmDocumentBootstrap::SourceBacked",
    "WasmDocumentBootstrap::Served",
):
    if bootstrap_variant in project_source:
        fail(f"parallel document bootstrap path `{bootstrap_variant}` reappeared")
if "prepare_browser_compute_host" in wasm_sources:
    fail("the transitional browser compute constructor reappeared")
if wasm_sources.count("fn prepare_browser_compute_runtime(") != 1:
    fail("browser compute must have exactly one prepared runtime constructor")

for name in (
    "__MECH_RENDERED_DOCUMENT_VALUE__", "__MECH_ACCEPTED_REPL_SOURCE__",
    "__MECH_REPLACE_ACCEPTED_REPL_SOURCE__", "__MECH_GPU_RUNTIME__",
    "__MECH_RUNTIME_INFO__", "__MECH_LAST_FRAME__", "__MECH_STOP__",
    "__MECH_COMPUTE_RESOURCE_SEQUENCE__", "__MECH_COMPUTE_PIPELINE_BUILD_COUNT__",
):
    if name in browser_sources:
        fail(f"test-only production global `{name}` reappeared")

# Scenarios may use the shared ChromeSession, but the unmistakable process and
# CDP ownership primitives stay in one file. This deliberately enforces an
# ownership boundary instead of attempting to interpret every host language.
canonical_harness = ROOT / "tests/browser/harness/chrome.py"
for browser_test_root in (ROOT / "scripts", ROOT / "tests/browser"):
    for path in browser_test_root.rglob("*"):
        if not path.is_file() or path in {Path(__file__).resolve(), canonical_harness}:
            continue
        if path.suffix not in {".js", ".mjs", ".py", ".sh"}:
            continue
        source = path.read_text(encoding="utf-8").lower()
        relative = path.relative_to(ROOT)
        for marker, boundary in {
            "--remote-debugging-port": "browser process ownership",
            "websocketdebuggerurl": "CDP endpoint discovery",
            "/json/version": "CDP endpoint discovery",
            "sec-websocket-key": "CDP transport",
        }.items():
            if marker in source:
                fail(f"scenario-specific {boundary} found in {relative}")
        if re.search(r"\b(?:chromium|firefox|webkit)\.launch\s*\(|"
                     r"\bwebdriver\.(?:chrome|firefox|safari|edge)\s*\(", source):
            fail(f"scenario-specific browser launcher found in {relative}")

architecture = text("docs/architecture/interactive-runtime.md")
if len(architecture.splitlines()) > 250:
    fail("interactive runtime architecture record must remain concise")
for builder in (
    "scripts/build-mech-browser.sh",
    "scripts/build-mech-gpu-browser.sh",
    "scripts/build-mech-gpu-browser.ps1",
):
    if (ROOT / builder).exists():
        fail(f"obsolete WASM build wrapper `{builder}` reappeared")
build_wasm = text("scripts/build-wasm.py")
for profile in ("browser", "browser-compute"):
    if f'"{profile}"' not in build_wasm:
        fail(f"unified WASM builder is missing the `{profile}` profile")
print("interactive architecture contract passed")
