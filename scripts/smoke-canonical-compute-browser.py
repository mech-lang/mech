#!/usr/bin/env python3
"""Check canonical fixed-shape publication through WASM CPU and real WebGPU."""

from __future__ import annotations

import argparse
import functools
import hashlib
import http.server
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import threading
import time
from urllib.parse import urljoin, urlsplit

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from tests.browser.harness import ChromeSession, NavigationContextPending
from browser_webgpu_flags import chrome_webgpu_test_flags


STATIC_SOURCE = '''@mouse := pointer://mouse/frame{:read(pulse)}
pulse := @mouse/pulse
@compute := compute://filters/kernel{:write(turn), :read(sample/result)}
@compute/turn <- pulse
answer := @compute/sample/result
answer

calculation @compute
-------------------
~counter := 2f32
counter = counter * 3f32 + 1f32
counter
'''


def file_digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def wait_for_probe(session, timeout, predicate):
    deadline = time.monotonic() + timeout
    state = None
    while time.monotonic() < deadline:
        try:
            state = session.evaluate("window.staticComputeProbe?.snapshot() || null")
        except NavigationContextPending:
            state = None
        if state and state["errors"]:
            raise RuntimeError(json.dumps(state))
        if state and predicate(state):
            return state
        time.sleep(0.05)
    raise RuntimeError(f"static compute timeout: {json.dumps(state)}")


def qualify_static_bundle(args):
    """Observe the shipping bootstrap; do not construct a substitute project."""
    work = Path(tempfile.mkdtemp(prefix="static-canonical-compute-"))
    print(f"Static compute product artifacts: {work}", file=sys.stderr)
    wasm_pkg = Path(args.wasm_pkg).resolve(strict=True)
    mech_bin = Path(args.mech_bin).resolve(strict=True)
    package_hashes = {
        name: file_digest(wasm_pkg / name)
        for name in ("mech_wasm.js", "mech_wasm_bg.wasm")
    }
    site = work / "site"
    site.mkdir()
    prefixes = {}
    product_fetches = {}
    for backend in ("cpu", "wgpu"):
        project = work / f"project-{backend}"
        project.mkdir()
        (project / "main.mec").write_text(STATIC_SOURCE)
        (project / "index.html").write_text(
            '<!doctype html><html><head><meta charset="utf-8"></head>'
            '<body style="min-height:90vh">Static compute qualification'
            '<script type="module" src="./_mech/project.js" '
            'data-mech-project="." data-mech-max-inputs="8"></script></body></html>'
        )
        (project / "mech.mcfg").write_text('''config := {
 hosts: [
  {name: "filters" provider: "compute" settings: {region: "calculation" backend: "BACKEND"}}
  {name: "mouse" provider: "pointer" settings: {}}
 ]
 run: {paths: ["main.mec"] grants: [
  {target: "filters/kernel" operations: ["read", "write"] paths: ["sample/result", "turn"]}
  {target: "mouse/frame" operations: ["read"] paths: ["pulse"]}
 ]}
 serve: {paths: ["main.mec", "index.html"] shim: "index.html"}
}
'''.replace("BACKEND", backend))
        prefix = f"/acceptance/compute/{backend}/"
        output = site / prefix.strip("/")
        # This command runs the actual selected package's Node admission before
        # any destination is published. Both the compiler and package are built
        # from the candidate by the caller, not supplied by a mocked factory.
        subprocess.run(
            [str(mech_bin), "bundle-web", str(project), "--out", str(output),
             "--wasm", str(wasm_pkg)],
            cwd=ROOT, check=True, timeout=args.timeout,
        )
        manifest = json.loads((output / "_mech/project-sources.json").read_text())
        if manifest["version"] != 4 or manifest["roots"] != ["main.mec"]:
            raise RuntimeError(f"unexpected retained-source manifest: {manifest}")
        root_source = next(source for source in manifest["sources"]
                           if source["specifier"] == "main.mec")
        if not root_source.get("documentUrl"):
            raise RuntimeError("bundle did not emit its actual root document transport")
        product_fetches[backend] = [
            prefix + name for name in (
                "pkg/mech_wasm.js", "pkg/mech_wasm_bg.wasm", "mech.mcfg",
                "_mech/project.js", "_mech/project-sources.json",
            )
        ] + [urlsplit(urljoin(prefix, source[key])).path
             for source in manifest["sources"] for key in ("url", "documentUrl")
             if key in source]
        if (output / root_source["url"]).read_text() != STATIC_SOURCE:
            raise RuntimeError("bundle changed retained source")
        for name, digest in package_hashes.items():
            if file_digest(output / "pkg" / name) != digest:
                raise RuntimeError(f"published package differs from admitted package: {name}")
        # Replace only the test page's bootstrap reference. The observer loads
        # the original emitted bootstrap and leaves all shipping JS/WASM bytes
        # intact; it neither drives frames nor alters command outputs.
        page = (output / "index.html").read_text()
        old = 'src="./_mech/project.js"'
        if page.count(old) != 1:
            raise RuntimeError("expected one emitted static bootstrap")
        page = page.replace(
            old, 'src="./_mech/qualification-probe.js" '
            f'data-mech-original="./_mech/project.js" data-mech-backend="{backend}"',
        )
        (output / "index.html").write_text(page)
        shutil.copyfile(
            ROOT / "scripts/tests/static-compute-product-probe.mjs",
            output / "_mech/qualification-probe.js",
        )
        prefixes[backend] = prefix

    requests = []

    class Handler(http.server.SimpleHTTPRequestHandler):
        def do_GET(self):
            requests.append(self.path.split("?", 1)[0])
            super().do_GET()

        def log_message(self, *args):
            pass

    server = http.server.ThreadingHTTPServer(
        ("127.0.0.1", 0), functools.partial(Handler, directory=str(site)),
    )
    threading.Thread(target=server.serve_forever, daemon=True).start()
    session = None
    passed = False
    try:
        flags = chrome_webgpu_test_flags(
            software_adapter=args.software_adapter or sys.platform.startswith("linux"),
            linux=sys.platform.startswith("linux"),
        )
        session = ChromeSession(
            args.browser, work / "profile", work / "chrome.log", flags=flags,
        ).start()
        results = []
        for backend, prefix in prefixes.items():
            page_url = f"http://127.0.0.1:{server.server_port}{prefix}"
            expected_backend = "wgpu" if backend == "wgpu" else "cpu-scalar"
            session.navigate(page_url)

            def ready(state):
                # Page.navigate is asynchronous: a ready probe in the previous
                # document must not satisfy the next product's readiness.
                return (state["ready"] and state["url"] == page_url
                        and state["backend"] == expected_backend)

            initial = wait_for_probe(session, args.timeout, ready)
            # Idle animation frames must not manufacture an initialization turn.
            time.sleep(0.2)
            idle = wait_for_probe(session, args.timeout, ready)
            if (idle["answer"] != initial["answer"] or idle["pointerInputs"] != 0
                    or idle["submitted"] or idle["accepted"]):
                raise RuntimeError(f"compute advanced without admitted input: {idle}")
            observed = []
            for turn, expected in enumerate((7, 22, 67), 1):
                # Chrome sends genuine pointer ingress to the shipping client.
                # No test calls project.pointerInput/frame or submits a command.
                session.call("Input.dispatchMouseEvent", {
                    "type": "mouseMoved", "x": 80 + turn * 25, "y": 90 + turn * 20,
                })

                def completed(state):
                    return (ready(state) and state["pointerInputs"] == turn
                            and state["answer"] == str(expected)
                            and (backend == "cpu" or
                                 (len(state["accepted"]) == turn and not state["pending"])))

                state = wait_for_probe(session, args.timeout, completed)
                if backend == "wgpu":
                    accepted = state["accepted"][-1]
                    if accepted["values"] != [expected]:
                        raise RuntimeError(f"wrong completed GPU output: {state}")
                    if len(state["submitted"]) != turn:
                        raise RuntimeError(f"extra or missing GPU submission: {state}")
                elif state["submitted"] or state["accepted"]:
                    raise RuntimeError(f"CPU unexpectedly submitted GPU work: {state}")
                observed.append(state["answer"])
            actual_requests = [path for path in requests if path.startswith(prefix)]
            if any(path not in actual_requests for path in product_fetches[backend]):
                raise RuntimeError(f"missing prefix-relative product fetches: {actual_requests}")
            results.append({"backend": state["backend"], "prefix": prefix,
                            "values": observed, "submitted": state["submitted"],
                            "accepted": state["accepted"], "adapter": state["adapter"],
                            "packageSha256": package_hashes, "requests": actual_requests})
        if any(path.startswith(("/pkg/", "/_mech/", "/main.mec")) for path in requests):
            raise RuntimeError(f"bundle relied on server-root product routes: {requests}")
        print("STATIC_CANONICAL_COMPUTE_PRODUCT", json.dumps(results))
        passed = True
    finally:
        if session:
            session.close()
        server.shutdown()
        server.server_close()
        if passed:
            shutil.rmtree(work)
        else:
            print(f"Static compute product artifacts: {work}", file=sys.stderr)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--browser", help="path to Chrome or Edge")
    parser.add_argument("--software-adapter", action="store_true")
    parser.add_argument("--timeout", type=int, default=120)
    parser.add_argument("--static-bundle", action="store_true",
                        help="qualify real emitted CPU/WGPU bundles under a non-root prefix")
    parser.add_argument("--mech-bin", default=str(ROOT / "target/debug/mech"))
    parser.add_argument("--wasm-pkg", default=str(ROOT / "src/wasm/pkg"))
    args = parser.parse_args()
    if args.static_bundle:
        qualify_static_bundle(args)
        return
    work = Path(tempfile.mkdtemp(prefix="canonical-compute-browser-"))
    shutil.copytree(ROOT / "src/wasm/pkg", work / "pkg")
    shutil.copyfile(ROOT / "include/browser-compute.js", work / "browser-compute.js")
    source = '''+> math
@pointer := pointer://pointer/frame{:read(pulse), :read(position)}
@particles := compute://particles/kernel{:write(input/force-point), :write(turn)}
@particles/input/force-point <- @pointer/position
@particles/turn <- @pointer/pulse

particle-field @compute
-------------------------------------------------------------------------------
force-point := [0f32; 0f32]
row := math/mod(1f32..=3f32, 4f32)
~matrix := [row; row + 3f32]
matrix = matrix + force-point
matrix'
'''
    config = (ROOT / "examples/gpu-particles/mech.mcfg").read_text()
    script = '''import init, {WasmMixedComputeProject} from './pkg/mech_wasm.js';
try {
  await init();
  const config = CONFIG;
  const source = SOURCE;
  const results = [];
  const adapter = await navigator.gpu.requestAdapter();
  if (!adapter) throw new Error('no WebGPU adapter');
  for (const backend of ['cpu', 'gpu']) {
    const project = WasmMixedComputeProject.fromSource(config, source, backend, true);
    const expectedBackend = backend === 'gpu' ? 'wgpu' : 'cpu-scalar';
    if (project.backend() !== expectedBackend) throw new Error('unexpected compute backend');
    const manifest = project.computeManifest();
    const resource = backend === 'gpu'
      ? await MechBrowserCompute.Device.create(manifest, adapter, ['result'])
      : null;
    project.start();
    let active = 0;
    const frames = [];
    for (let turn = 1; turn <= 2; turn++) {
      const command = project.frame(10, 20, false, 1 / 60, 8);
      let actual;
      if (resource) {
        // Probe the publication buffer through the real bridge's planned
        // readback. Runtime completion receives only its requested outputs.
        resource.setRequestedOutputs(['result']);
        const submission = resource.submit(command, active);
        const result = await resource.finish(submission);
        if (result.integrity) throw new Error(JSON.stringify(result.integrity));
        actual = Array.from(result.outputs.find(output => output.name === 'result').values);
        project.completeComputeCommand({
          version: 1,
          token: command.dispatchToken,
          status: 'completed',
          outputs: result.outputs.filter(output =>
            (command.requestedOutputs || []).includes(output.name)),
        });
        active = submission.outputIndex;
      } else {
        actual = Array.from(project.cpuOutput('result'));
      }
      const expected = [
        1 + 10 * turn, 4 + 20 * turn,
        2 + 10 * turn, 5 + 20 * turn,
        3 + 10 * turn, 6 + 20 * turn,
      ];
      if (JSON.stringify(actual) !== JSON.stringify(expected)) {
        throw new Error(JSON.stringify({backend, turn, actual, expected}));
      }
      frames.push(actual);
    }
    project.stop();
    resource?.dispose();
    if (resource?.disposeCompletion) await resource.disposeCompletion;
    results.push({backend: project.backend(), frames});
    project.free();
  }
  window.outcome = {ok: true, results};
} catch (error) {
  window.outcome = {ok: false, error: String(error), stack: error?.stack};
}
'''.replace('CONFIG', json.dumps(config)).replace('SOURCE', json.dumps(source))
    (work / "index.html").write_text(
        '<script src="browser-compute.js"></script>'
        '<script type="module">' + script + '</script>'
    )

    class Handler(http.server.SimpleHTTPRequestHandler):
        def log_message(self, *args):
            pass

    server = http.server.ThreadingHTTPServer(
        ("127.0.0.1", 0), functools.partial(Handler, directory=str(work)),
    )
    threading.Thread(target=server.serve_forever, daemon=True).start()
    session = None
    passed = False
    try:
        flags = chrome_webgpu_test_flags(
            software_adapter=args.software_adapter or sys.platform.startswith("linux"),
            linux=sys.platform.startswith("linux"),
        )
        session = ChromeSession(
            args.browser, work / "profile", work / "chrome.log", flags=flags,
        ).start()
        session.navigate(f"http://127.0.0.1:{server.server_port}/")
        deadline = time.monotonic() + args.timeout
        while time.monotonic() < deadline:
            try:
                outcome = session.evaluate("window.outcome || null")
            except NavigationContextPending:
                outcome = None
            if outcome:
                if not outcome["ok"]:
                    raise RuntimeError(json.dumps(outcome))
                print("CANONICAL_COMPUTE_PUBLICATION", json.dumps(outcome["results"]))
                passed = True
                break
            time.sleep(0.1)
        else:
            raise RuntimeError("canonical compute browser timeout")
    finally:
        if session:
            session.close()
        server.shutdown()
        server.server_close()
        if passed:
            shutil.rmtree(work, ignore_errors=True)
        else:
            print(f"Canonical compute browser artifacts: {work}", file=sys.stderr)


if __name__ == "__main__":
    main()
