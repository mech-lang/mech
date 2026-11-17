from pathlib import Path
import json
import re
import sys
import time

from tests.browser.harness import BrowserFailure, ChromeSession

root = Path(__file__).resolve().parent
capture = next(root.glob('ci-artifacts/*/formatted-docs/chrome.dom')).read_text()
styles = {name: css for name, css in re.findall(r'<style data-mech-style-layer="([^"]+)">(.*?)</style>', capture, re.S)}
controller = (root / 'include/document.js').read_text()
stop = controller.index('main().catch(error =>')
controller = controller[:stop] + """
state.root = documentRoot();
initializeReplComponentContract();
attachConsole();
initializeLayout();
globalThis.__layoutReady = true;
"""
page = (root / 'include/docs.html').read_text()
page = re.sub(r'<link [^>]+https?://[^>]+>\n?|<script defer src="https?://[^>]+></script>\n?', '', page)
for token, value in {
    'PALETTE_STYLESHEET': styles['palette'],
    'MECH_SOURCE_STYLESHEET': styles['source'],
    'MECHDOWN_STYLESHEET': styles['mechdown'],
    'PAGE_STYLESHEET': styles['page'],
    'MECH_REPL_STYLESHEET': styles['repl'],
    'PRESENTATION': 'document',
    'TITLE': 'Formatted documentation resize probe',
    'REPL': '<div class="console-scroll mech-repl" id="mech-output"></div>',
    'DOCUMENT_SCRIPT': controller,
}.items():
    page = page.replace('{{' + token + '}}', value)
page = re.sub(r'\{\{[A-Z_]+\}\}', '', page)
(root / 'resize-fixture.html').write_text(page)

variant = sys.argv[1]
exit_type = sys.argv[2]
path = root / ('before.sh' if variant == 'before' else 'scripts/smoke-served-rich-document-browser.sh')
source = path.read_text()
profile = root / f'{variant}-{exit_type}-profile'
with ChromeSession(None, profile, root / f'{variant}-{exit_type}-chrome.log', window_size=(1680,900), startup_timeout=15,
                   flags=['--disable-gpu', '--run-all-compositor-stages-before-draw', '--hide-scrollbars', '--disable-background-networking']) as browser_session:
    devtools = browser_session.devtools
    session_id = browser_session.session_id
    evaluate = browser_session.evaluate
    evaluate_json = browser_session.evaluate_json
    wait_for = browser_session.wait_for
    def fail(message):
        raise BrowserFailure(message)

    browser_session.call("Emulation.setDeviceMetricsOverride", {
        "width": 1680, "height": 900, "deviceScaleFactor": 1, "mobile": False,
    })
    browser_session.navigate((root / 'resize-fixture.html').as_uri())
    wait_for("globalThis.__layoutReady === true", "the shipped docs controller's layout initialization", timeout=10)

    if exit_type == 'fallback':
        evaluate("Object.defineProperty(document.querySelector('[data-mech-console-pane]'), 'requestFullscreen', {value: undefined})")
    else:
        evaluate("""
(() => {
  const nativeExit = document.exitFullscreen.bind(document);
  window.__resizeProbeNativeExits = 0;
  document.exitFullscreen = async function () {
    window.__resizeProbeNativeExits += 1;
    if (window.__resizeProbeNativeExits === 2) {
      await new Promise(resolve => setTimeout(resolve, 2000));
    }
    await nativeExit();
  };
})();
""")

    for name in ('assert_fullscreen_accessibility', 'assert_right_console_resize_direction'):
        start = source.index(f'def {name}():')
        end = source.index('\ndef ', start + 1)
        exec(compile(source[start:end], str(path), 'exec'), globals())

    start = time.monotonic()
    assert_fullscreen_accessibility()
    geometry = evaluate_json("""
(() => ({
    viewport: innerWidth,
    mode: document.querySelector('[data-mech-repl-host]').dataset.mechConsoleMode,
    nativeFullscreen: document.fullscreenElement !== null,
    timedExitCalls: window.__resizeProbeNativeExits || 0,
    dockedResizeStartWidth: document.querySelector('[data-mech-console-pane]').getBoundingClientRect().width,
}))()
""")
    print(json.dumps(dict(variant=variant, exit_type=exit_type, seconds=time.monotonic()-start, **geometry)), flush=True)
    assert_right_console_resize_direction()
    print("All original drag assertions passed", flush=True)
