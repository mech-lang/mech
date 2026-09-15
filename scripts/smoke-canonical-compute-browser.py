#!/usr/bin/env python3
"""Check canonical fixed-shape publication through WASM CPU and real WebGPU."""

from __future__ import annotations

import argparse
import functools
import http.server
import json
from pathlib import Path
import shutil
import sys
import tempfile
import threading
import time

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from tests.browser.harness import ChromeSession, NavigationContextPending
from browser_webgpu_flags import chrome_webgpu_test_flags


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--browser", help="path to Chrome or Edge")
    parser.add_argument("--software-adapter", action="store_true")
    parser.add_argument("--timeout", type=int, default=120)
    args = parser.parse_args()
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
