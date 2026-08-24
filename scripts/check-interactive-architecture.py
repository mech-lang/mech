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


def owns_browser_process(source: str) -> bool:
    """Track browser-valued commands into Python, JS, and shell launchers."""
    source = source.lower()
    executable = (r"(?<![\w-])(?:google-chrome|chromium(?:-browser)?|microsoft-edge(?:-stable)?|"
                  r"msedge(?:\.exe)?|chrome(?:\.exe)?|firefox(?:-esr)?|geckodriver|"
                  r"safaridriver|webkit2gtk-driver)(?![\w-])")
    assignments = re.findall(
        r"(?m)^\s*(?:const\s+|let\s+|var\s+)?([a-z_$]\w*)\s*=\s*([^\n;]+)",
        source,
    )
    browser_value = rf"{executable}|find_browser|args\.browser|(?:chrome|edge|firefox)_bin"
    browser_names: set[str] = set()
    for _ in assignments:
        for name, value in assignments:
            if re.search(browser_value, value) or any(
                re.search(rf"\b{known}\b", value) for known in browser_names
            ):
                browser_names.add(name)
    launchers = {
        "subprocess.popen", "subprocess.run", "subprocess.call",
        "subprocess.check_call", "subprocess.check_output",
        "asyncio.create_subprocess_exec", "os.system", "child_process.spawn",
        "child_process.exec", "child_process.execfile", "deno.command", "bun.spawn",
    }
    for imports in re.findall(r"from\s+(?:subprocess|asyncio|os)\s+import\s+([^\n]+)", source):
        for item in imports.split(","):
            original, _, alias = item.strip().partition(" as ")
            if original in {"popen", "run", "call", "check_call", "check_output",
                            "create_subprocess_exec", "system"}:
                launchers.add(alias or original)
    for module, alias in re.findall(r"import\s+(subprocess|asyncio|os)\s+as\s+(\w+)", source):
        methods = {"subprocess": ("popen", "run", "call", "check_call", "check_output"),
                   "asyncio": ("create_subprocess_exec",), "os": ("system",)}[module]
        launchers.update(f"{alias}.{method}" for method in methods)
    for imports in re.findall(
        r"(?:import\s*|(?:const|let|var)\s*)\{([^}]+)\}\s*(?:from|=\s*require\()\s*"
        r"['\"](?:node:)?child_process['\"]",
        source,
    ):
        for item in imports.split(","):
            original, *aliases = re.split(r"\s+as\s+|\s*:\s*", item.strip(), maxsplit=1)
            if original in {"spawn", "exec", "execfile", "fork"}:
                launchers.add(aliases[-1] if aliases else original)
    module_aliases = re.findall(
        r"(?:import\s+(?:\*\s+as\s+)?|(?:const|let|var)\s+)([a-z_$]\w*)\s*"
        r"(?:from\s*|=\s*require\()['\"](?:node:)?child_process['\"]",
        source,
    )
    launchers.update(f"{alias}.{method}" for alias in module_aliases
                     for method in ("spawn", "exec", "execfile", "fork"))
    for alias, method in re.findall(
        r"(?:const|let|var)\s+([a-z_$]\w*)\s*=\s*require\(['\"]"
        r"(?:node:)?child_process['\"]\)\.(spawn|exec|execfile|fork)", source,
    ):
        launchers.add(alias)
    launcher = "|".join(sorted(map(re.escape, launchers), key=len, reverse=True))
    for match in re.finditer(rf"(?:{launcher})\s*\((?P<argv>.{{0,1000}}?)\)", source, re.DOTALL):
        argv = match.group("argv")
        if re.search(browser_value, argv) or any(
            re.search(rf"\b{name}\b", argv) for name in browser_names
        ):
            return True
    if re.search(r"\b(?:chromium|firefox|webkit)\.launch\s*\(|"
                 r"\bwebdriver\.(?:chrome|firefox|safari|edge)\s*\(", source):
        return True
    names = "|".join(map(re.escape, browser_names | {"chrome_bin", "edge_bin", "firefox_bin"}))
    wrappers = r"(?:(?:exec|command|nohup|xvfb-run|timeout)(?:\s+[-\w.]+)*\s+)*"
    shell = rf"(?m)(?<!\\\n)^\s*(?:env\s+[^\n]*\s+)?{wrappers}(?:\"?\$\{{?(?:{names})\}}?\"?|{executable})(?=\s)"
    return re.search(shell, source) is not None


for browser_test_root in (ROOT / "scripts", ROOT / "tests/browser"):
    for path in browser_test_root.rglob("*"):
        if (
            not path.is_file()
            or path.suffix not in {".js", ".mjs", ".cjs", ".jsx",
                                   ".ts", ".mts", ".cts", ".tsx",
                                   ".py", ".sh", ".bash"}
            or path in {Path(__file__).resolve(), canonical_cdp_harness}
        ):
            continue
        source = path.read_text(encoding="utf-8")
        lower_source = source.lower()
        relative = path.relative_to(ROOT)
        if ("websocket" in lower_source or "sec-websocket" in lower_source) and any(
            marker in lower_source for marker in (
            "websocketdebuggerurl", "/devtools/", "/json/version",
            "runtime.evaluate", "target.attach", "target.createtarget",
        )):
            fail(f"scenario-specific CDP transport found in {path.relative_to(ROOT)}")
        if "--remote-debugging-port" in lower_source or "--dump-dom" in lower_source:
            fail(f"scenario-specific browser process ownership found in {relative}")
        if owns_browser_process(source):
            fail(f"browser scenario directly owns a process outside ChromeSession: {relative}")

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
