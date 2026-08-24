#!/usr/bin/env python3
"""Cheap structural guards for the interactive/browser architecture."""

from __future__ import annotations

from pathlib import Path
import re
import shlex
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
legacy_contracts = (
    "data-tab", "data-panel", "#console-panel", "#output-panel", "#errors-panel",
    "#resizer", "#edgeHandle", "body.console-fullscreen", "is-collapsed",
)
for legacy in legacy_contracts:
    if legacy in browser_sources:
        fail(f"legacy browser component contract `{legacy}` reappeared")
obsolete_completion = ("acknowledgeComputeCommand", "rejectComputeCommand",
                       "rejectIntegrityComputeCommand")
for operation in obsolete_completion:
    if operation in browser_sources or any(
        operation in path.read_text(encoding="utf-8")
        for path in (ROOT / "src/wasm/src").rglob("*.rs")
    ):
        fail(f"obsolete compute completion operation `{operation}` reappeared")
wasm_sources = "\n".join(path.read_text(encoding="utf-8")
                         for path in (ROOT / "src/wasm/src").rglob("*.rs"))
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
test_only_globals = (
    "__MECH_RENDERED_DOCUMENT_VALUE__", "__MECH_ACCEPTED_REPL_SOURCE__",
    "__MECH_REPLACE_ACCEPTED_REPL_SOURCE__", "__MECH_GPU_RUNTIME__",
    "__MECH_RUNTIME_INFO__", "__MECH_LAST_FRAME__", "__MECH_STOP__",
    "__MECH_COMPUTE_RESOURCE_SEQUENCE__", "__MECH_COMPUTE_PIPELINE_BUILD_COUNT__",
)
for name in test_only_globals:
    if name in browser_sources:
        fail(f"test-only production global `{name}` reappeared")
canonical_cdp_harness = ROOT / "tests/browser/harness/chrome.py"
BROWSER_EXECUTABLE = (r"(?<![\w-])(?:google-chrome(?:-stable)?|chromium(?:-browser)?|microsoft-edge(?:-stable)?|msedge(?:\.exe)?|chrome(?:\.exe)?|firefox(?:-esr)?|geckodriver|safaridriver|webkit2gtk-driver)(?![\w-])")
PY_LAUNCHERS = ("popen", "run", "call", "check_call", "check_output", "create_subprocess_exec", "system")
JS_LAUNCHERS = ("spawn", "exec", "execfile", "fork")
WRAPPER_OPTIONS = {
    "env": {"-u", "--unset", "-c", "--chdir", "-s", "--split-string"},
    "exec": {"-a"},
    "xvfb-run": {"-e", "--error-file", "-f", "--auth-file", "-n", "--server-num", "-p", "--xauth-protocol", "-s", "--server-args", "-w", "--wait"},
    "timeout": {"-k", "--kill-after", "-s", "--signal"},
    "command": set(), "nohup": set(),
}


def unwrapped_executable(tokens: list[str]) -> str:
    tokens = list(tokens)
    while tokens and tokens[0] in {"str", "path", "os.fspath"}:
        tokens.pop(0)
    while tokens and tokens[0] in WRAPPER_OPTIONS:
        wrapper = tokens.pop(0)
        positional = 1 if wrapper == "timeout" else 0
        while tokens:
            token = tokens[0]
            option = token.split("=", 1)[0]
            if option in WRAPPER_OPTIONS[wrapper]:
                tokens.pop(0)
                if "=" not in token and tokens:
                    tokens.pop(0)
            elif token.startswith("-") or (wrapper == "env" and "=" in token):
                tokens.pop(0)
            elif positional:
                tokens.pop(0)
                positional -= 1
            else:
                break
    return tokens[0] if tokens else ""

def launched_executable(argv: str) -> str:
    tokens = []
    for _, literal, identifier, number in re.findall(
            r"(['\"])(.*?)\1|([a-z_$][\w.$-]*)|(\d+(?:\.\d+)?)", argv):
        path = literal and re.search(BROWSER_EXECUTABLE, literal) and re.search(r"[/\\]", literal)
        tokens.extend([literal] if path else literal.split() if literal else [identifier or number])
    return unwrapped_executable(tokens)

def discovery_argument(arguments: str) -> str:
    keyword = re.search(r"(?:^|,)\s*cmd\s*=\s*(?P<value>[^,]+)", arguments)
    return keyword.group("value") if keyword else arguments.split(",", 1)[0]

def shell_executable(line: str) -> str:
    try:
        tokens = shlex.split(line, comments=True, posix=True)
    except ValueError:
        return ""
    while tokens and re.fullmatch(r"[a-z_]\w*=.*", tokens[0]):
        tokens.pop(0)
    return unwrapped_executable(tokens)

def discovery_aliases(source: str) -> tuple[set[str], set[str]]:
    finders, whiches = {"find_browser"}, {"which", "shutil.which"}
    for module, imports in re.findall(r"from\s+([\w.]+)\s+import\s+([^\n]+)", source):
        for original, alias in re.findall(r"\b(find_browser|which)\b(?:\s+as\s+(\w+))?", imports):
            if original == "find_browser" or module == "shutil":
                (finders if original == "find_browser" else whiches).add(alias or original)
    for module, alias in re.findall(r"import\s+([\w.]+)\s+as\s+(\w+)", source):
        if module == "shutil":
            whiches.add(f"{alias}.which")
        if module.endswith("browser.harness"):
            finders.add(f"{alias}.find_browser")
    return finders, whiches

def browser_value_names(source: str) -> set[str]:
    source = source.lower()
    assignments = re.findall(
        r"(?m)(?:^|;)\s*(?:const\s+|let\s+|var\s+)?([a-z_$]\w*)\s*=\s*([^\n;]+)",
        source,
    )
    browser_value = rf"{BROWSER_EXECUTABLE}|find_browser|args\.browser|(?:chrome|edge|firefox)_bin"
    finders, whiches = discovery_aliases(source)
    finder = "|".join(sorted(map(re.escape, finders), key=len, reverse=True))
    which = "|".join(sorted(map(re.escape, whiches), key=len, reverse=True))
    browser_names: set[str] = set()
    for _ in assignments:
        for name, value in assignments:
            executable = launched_executable(value)
            discovery = re.search(rf"\b(?P<finder>{finder})\s*\(|"
                                  rf"\b(?:{which})\s*\((?P<args>[^)]*)\)", value)
            candidate = launched_executable(discovery_argument(discovery.group("args"))) \
                if discovery and discovery.group("args") is not None else ""
            if (re.search(browser_value, executable) or executable in browser_names or
                discovery and (discovery.group("finder") or
                               re.search(BROWSER_EXECUTABLE, candidate) or
                               candidate in browser_names)):
                browser_names.add(name)
    return browser_names

def owns_browser_process(source: str, language: str = "python",
                         known_browser_names: set[str] | None = None) -> bool:
    browser_names = set(browser_value_names(source) if known_browser_names is None
                        else known_browser_names)
    source = source.lower()
    browser_value = rf"{BROWSER_EXECUTABLE}|find_browser|args\.browser|(?:chrome|edge|firefox)_bin"
    launchers = {f"subprocess.{name}" for name in PY_LAUNCHERS[:5]}
    launchers |= {"asyncio.create_subprocess_exec", "os.system", "deno.command", "bun.spawn"}
    for imports in re.findall(r"from\s+(?:subprocess|asyncio|os)\s+import\s+([^\n]+)", source):
        for original, alias in re.findall(rf"\b({'|'.join(PY_LAUNCHERS)})\b(?:\s+as\s+(\w+))?", imports):
            launchers.add(alias or original)
    for module, alias in re.findall(r"import\s+(subprocess|asyncio|os)\s+as\s+(\w+)", source):
        methods = PY_LAUNCHERS[:5] if module == "subprocess" else PY_LAUNCHERS[5:6] if module == "asyncio" else PY_LAUNCHERS[6:]
        launchers.update(f"{alias}.{method}" for method in methods)
    child_imports = re.findall(
        r"(?:import|const|let|var)[^\n]*?(?:node:)?child_process[^\n]*", source)
    for imported in child_imports:
        for original, alias in re.findall(rf"\b({'|'.join(JS_LAUNCHERS)})\b(?:\s*(?:as|:)\s*(\w+))?", imported):
            launchers.add(alias or original)
        for alias in re.findall(r"(?:\*\s+as\s+|(?:const|let|var)\s+)(\w+)", imported):
            launchers.update(f"{alias}.{method}" for method in JS_LAUNCHERS)
    launchers.update(f"child_process.{name}" for name in JS_LAUNCHERS)
    launcher = "|".join(sorted(map(re.escape, launchers), key=len, reverse=True))
    for match in re.finditer(rf"(?:{launcher})\s*\((?P<argv>.{{0,1000}}?)\)", source, re.DOTALL):
        executable = launched_executable(match.group("argv"))
        if re.search(browser_value, executable) or executable in browser_names:
            return True
    if re.search(r"\b(?:chromium|firefox|webkit)\.launch\s*\(|"
                 r"\bwebdriver\.(?:chrome|firefox|safari|edge)\s*\(", source):
        return True
    if language != "shell":
        return False
    browser_names |= {"chrome_bin", "edge_bin", "firefox_bin"}
    for line in source.splitlines():
        executable = shell_executable(line)
        variable = re.fullmatch(r"\$\{?([a-z_]\w*)\}?", executable)
        if re.search(BROWSER_EXECUTABLE, executable) or (
            variable and variable.group(1) in browser_names
        ):
            return True
    return False

def discovers_browser(source: str, language: str = "python",
                      known_browser_names: set[str] | None = None) -> bool:
    finders, whiches = discovery_aliases(source.lower())
    browser_names = set(browser_value_names(source) if known_browser_names is None
                        else known_browser_names)
    normalized = source.lower()
    finder = "|".join(sorted(map(re.escape, finders), key=len, reverse=True))
    which = "|".join(sorted(map(re.escape, whiches), key=len, reverse=True))
    if re.search(rf"\b(?:{finder})\s*\(", normalized):
        return True
    for match in re.finditer(rf"\b(?:{which})\s*\((?P<args>[^)]*)\)", normalized):
        candidate = launched_executable(discovery_argument(match.group("args")))
        if re.search(BROWSER_EXECUTABLE, candidate) or candidate in browser_names:
            return True
    return re.search(rf"\b(?:command\s+-v|which)\s+{BROWSER_EXECUTABLE}|"
                     r"\b(?:chrome|edge|firefox)_bin\b", normalized) is not None

ownership_probes = (
    ('subprocess.run(["python3", "verify.py", "--browser-family", "chromium"])', False, "python"),
    ('cmd = ["python3", "verify.py", "chromium"]\nsubprocess.run(cmd)', False, "python"),
    ('browser = "/usr/bin/chromium"\nsubprocess.run(["timeout", "2s", browser])', True, "python"),
    ('browser = args.browser\nsubprocess.run(["timeout", "-s", "KILL", "2", browser])', True, "python"),
    ('subprocess.run(["google-chrome-stable", "--headless"])', True, "python"),
    ('browser = args.browser\ncmd = [browser, "--headless"]\nsubprocess.run(cmd)', True, "python"),
    ('candidate = "chromium"\nbrowser = shutil.which(candidate)\nsubprocess.run([browser])', True, "python"),
    ('candidate = "chromium"; browser = shutil.which(cmd=candidate)\nsubprocess.run([browser])', True, "python"),
    ('import shutil as fs\ncandidate = "chromium"\nbrowser = fs.which(cmd=candidate)\nsubprocess.run([browser])', True, "python"),
    ('from shutil import which as locate\ncandidate = "chromium"\nbrowser = locate(cmd=candidate)\nsubprocess.run([browser])', True, "python"),
    ('chrome_bin="/usr/bin/chromium"\ntimeout --signal=KILL 2 "$chrome_bin"', True, "shell"),
    ('xvfb-run --server-num=2 google-chrome --headless', True, "shell"),
    ('from subprocess import run as launch\nb = args.browser\nlaunch([b])', True, "python"),
    ('import { spawn as launch } from "node:child_process"; launch("firefox", [])', True, "javascript"),
)
for probe, expected, language in ownership_probes:
    if owns_browser_process(probe, language) is not expected:
        fail(f"browser executable-position detector failed its {expected=} probe: {probe}")
for probe in ('browser = shutil.which("chromium")',
              'from shutil import which as locate\nbrowser = locate("chromium")',
              'candidate = "chromium"\nfrom shutil import which as locate\nbrowser = locate(candidate)',
              'candidate = "chromium"; browser = shutil.which(cmd=candidate)',
              'import shutil as fs\ncandidate = "chromium"\nbrowser = fs.which(cmd=candidate)',
              'from shutil import which as locate\ncandidate = "chromium"\nbrowser = locate(cmd=candidate)',
              'browser = shutil.which("google-chrome-stable")'):
    if not discovers_browser(probe):
        fail(f"browser discovery detector missed: {probe}")
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
        language = "python" if path.suffix == ".py" \
            else "shell" if path.suffix in {".sh", ".bash"} else "javascript"
        browser_names = browser_value_names(source)
        if discovers_browser(source, language, browser_names):
            fail(f"scenario-specific browser executable discovery found in {relative}")
        if owns_browser_process(source, language, browser_names):
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
