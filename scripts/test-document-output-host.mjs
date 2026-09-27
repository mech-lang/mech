import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";

// Execute the shared controller itself, without importing or emulating WASM.
// Presentation startup keeps this focused on the supported output-host API.
const dataKey = name => name.replace(/^data-/, "").replace(/-([a-z])/g, (_, letter) => letter.toUpperCase());
class Element extends EventTarget {
  constructor(className = "") {
    super();
    this.className = className;
    this.dataset = {};
    this.attributes = new Map();
    this.children = [];
    this.hidden = false;
    this.scrollHeight = this.clientHeight = 100;
    this.scrollTop = 0;
    this.scrollLeft = 0;
    this.style = { scrollBehavior: "auto", overflowY: "visible" };
    this.offsetLeft = this.offsetTop = 0;
    this.offsetParent = null;
    this.classList = {
      contains: name => this.className.split(/\s+/).includes(name),
      add: name => { if (!this.classList.contains(name)) this.className += ` ${name}`; },
      remove: name => { this.className = this.className.split(/\s+/).filter(value => value !== name).join(" "); },
      toggle: (name, force) => {
        const present = force ?? !this.classList.contains(name);
        this.classList[present ? "add" : "remove"](name);
        return present;
      },
    };
  }
  get childNodes() { return this.children; }
  get firstElementChild() { return this.children.find(child => child instanceof Element) || null; }
  setAttribute(name, value) {
    this.attributes.set(name, String(value));
    if (name.startsWith("data-")) this.dataset[dataKey(name)] = String(value);
  }
  getAttribute(name) { return this.attributes.get(name) ?? null; }
  removeAttribute(name) { this.attributes.delete(name); }
  append(...children) {
    for (const child of children) {
      if (child instanceof Element) { child.remove(); child.parentElement = this; }
      this.children.push(child);
    }
  }
  replaceChildren(...children) {
    for (const child of this.children) if (child instanceof Element) child.parentElement = null;
    this.children = [];
    this.append(...children);
  }
  remove() {
    if (this.parentElement) this.parentElement.children = this.parentElement.children.filter(child => child !== this);
    this.parentElement = null;
  }
  matches(selector) {
    const attribute = selector.match(/^\[data-([a-z-]+)(?:=["']([^"']*)["'])?\]$/);
    if (attribute) {
      const key = dataKey(`data-${attribute[1]}`);
      return key in this.dataset && (attribute[2] === undefined || this.dataset[key] === attribute[2]);
    }
    return selector.startsWith(".") && this.classList.contains(selector.slice(1));
  }
  querySelectorAll(selector) {
    if (selector.includes(",")) {
      return [...new Set(selector.split(",").flatMap(part => this.querySelectorAll(part.trim())))];
    }
    if (selector.startsWith(":scope > ")) {
      const [first, ...rest] = selector.slice(9).split(/\s+/);
      const direct = this.children.filter(child => child instanceof Element && child.matches(first));
      return rest.length ? direct.flatMap(child => child.querySelectorAll(rest.join(" "))) : direct;
    }
    return this.children.flatMap(child => child instanceof Element
      ? [...(child.matches(selector) ? [child] : []), ...child.querySelectorAll(selector)] : []);
  }
  querySelector(selector) { return this.querySelectorAll(selector)[0] || null; }
  closest(selector) { return this.matches(selector) ? this : this.parentElement?.closest(selector) || null; }
  scrollTo(x, y) { this.scrollLeft = x; this.scrollTop = y; }
}

const root = new Element();
const page = new Element();
const controller = { dataset: { mechDocumentMode: "presentation" } };
const window = new EventTarget();
window.scrollX = window.scrollY = 0;
window.scrollTo = (x, y) => { window.scrollX = x; window.scrollY = y; };
let contentShell = null;
const document = Object.assign(new EventTarget(), {
  documentElement: page,
  body: new Element(),
  querySelector: selector => selector === "script[data-mech-document-controller]" ? controller
    : selector === ".mech-root, .mech-document" ? root
    : selector === ".content-shell" ? contentShell : null,
  querySelectorAll: () => [],
  getElementById: () => null,
  createElement: () => new Element(),
});
let imports = 0;
const sandbox = vm.createContext({
  document, window, AbortController, CustomEvent, Node: { TEXT_NODE: 3 }, console,
  history: { scrollRestoration: "auto" },
  location: { origin: "https://example.test", pathname: "/article", search: "" },
  localStorage: { getItem: () => null },
  getComputedStyle: element => element.style,
});
new vm.Script(readFileSync(new URL("../include/document.js", import.meta.url), "utf8"), {
  importModuleDynamically() { imports++; throw new Error("output presentation must not import WASM"); },
}).runInContext(sandbox);
await new Promise(resolve => setImmediate(resolve));
const run = source => vm.runInContext(source, sandbox);
const api = sandbox.MechDocumentController;
assert.equal(api.showOutput(), false, "missing output component is a supported no-op");

const pane = new Element();
pane.dataset.mechConsolePane = "";
const toggle = new Element();
root.append(pane, toggle);
const panels = Object.fromEntries(["output", "console", "errors"].map(name => {
  const panel = new Element("console-scroll");
  panel.dataset.mechConsolePanel = name;
  if (name === "output") panel.dataset.mechOutputPanel = "";
  if (name === "errors") panel.dataset.mechErrorsPanel = "";
  pane.append(panel);
  return [name, panel];
}));
const tabs = Object.fromEntries(["output", "console", "errors"].map(name => {
  const tab = new Element();
  tab.dataset.mechConsoleTab = name;
  tab.dataset.mechConsoleBaseLabel = name;
  pane.append(tab);
  return [name, tab];
}));
const ordinaryQueryAll = root.querySelectorAll.bind(root);
root.querySelectorAll = selector => selector.includes("data-mech-console-toggle") ? [toggle] : ordinaryQueryAll(selector);

for (const lifecycle of ["new", "starting", "ready", "failed", "stopped"]) {
  run(`state.runtimeLifecycle = ${JSON.stringify(lifecycle)}`);
  root.dataset.mechConsoleOpen = "false";
  root.dataset.mechConsoleMode = "docked";
  pane.hidden = true;
  tabs.output.dataset.mechConsoleUnread = "true";
  assert.equal(api.showOutput(), true, `can reveal application output while ${lifecycle}`);
  assert.equal(root.dataset.mechConsoleOpen, "true");
  assert.equal(pane.hidden, false);
  assert.equal(toggle.getAttribute("aria-expanded"), "true");
  assert.equal(pane.dataset.mechConsoleActivePanel, "output");
  assert.equal(tabs.output.getAttribute("aria-selected"), "true");
  assert.equal(tabs.console.getAttribute("aria-selected"), "false");
  assert.equal(tabs.output.dataset.mechConsoleUnread, undefined);
  assert.equal(panels.output.hidden, false);
  assert.equal(panels.console.hidden, true);
  assert.equal(panels.errors.hidden, true);
}
// Existing fullscreen/workspace ownership and its multi-panel layout survive.
for (const mode of ["button", "drag"]) {
  root.dataset.mechConsoleMode = mode;
  root.dataset.mechOutputFullscreenActive = "true";
  assert.equal(api.showOutput(), true);
  assert.equal(root.dataset.mechConsoleMode, mode);
  assert.equal(root.dataset.mechOutputFullscreenActive, "true");
  for (const panel of Object.values(panels)) assert.equal(panel.hidden, false);
}
assert.equal(run("state.document"), null, "no runtime readiness requirement");
assert.equal(imports, 0);

// Static application DOM is not an implicit document-result region.
const application = new Element();
application.dataset.mechOutputRegion = "application";
const applicationControl = new Element();
application.append(applicationControl);
panels.output.append(application);
sandbox.entries = [{ address: { outputId: 7n }, rendered: { kind: "f64", blockHtml: "42" } }];
run("refreshOutputPanel(entries)");
const implicit = panels.output.querySelector('[data-mech-output-region="document"]');
assert.equal(implicit.children.length, 1, "default documents retain implicit output");
panels.output.dataset.mechOutputHost = "application";
run("refreshOutputPanel(entries)");
assert.equal(implicit.children.length, 0, "application hosts suppress and clear only implicit output");
assert.equal(application.firstElementChild, applicationControl);

// Explicit stdout/stderr and clear operations retain their ordinary regions.
run(`appendProgramOutput({stream: "stdout", source: "program", display_id: "example",
  operation: "replace", content: {kind: "text", data: {text: "explicit output"}}})`);
const explicit = panels.output.querySelector('[data-mech-output-region="repl"]');
assert.equal(explicit.children.length, 1);
assert.equal(explicit.firstElementChild.firstElementChild.children.join(""), "explicit output");
run(`appendProgramOutput({stream: "stderr", operation: "replace",
  content: {kind: "text", data: {text: "diagnostic"}}})`);
const errors = panels.errors.querySelector('[data-mech-error-region="program"]');
assert.equal(errors.children.length, 1);
assert.equal(application.firstElementChild, applicationControl);
run("appendProgramOutput({stream: 'stdout', operation: 'clear'})");
assert.equal(explicit.children.length, 0);
assert.equal(errors.children.length, 0);
assert.equal(implicit.children.length, 0);
assert.equal(application.firstElementChild, applicationControl, "clear output never removes the application");
delete panels.output.dataset.mechOutputHost;
run("refreshOutputPanel(entries)");
assert.equal(implicit.children.length, 1, "unmarked default behavior is unchanged");

// Legacy/custom shims gain a reachable Close control and visible fullscreen
// labels without duplicating controls when the component contract is refreshed.
const topbar = new Element("console-topbar");
const outputFullscreen = new Element();
outputFullscreen.dataset.mechOutputFullscreen = "";
const workspaceFullscreen = new Element();
workspaceFullscreen.dataset.mechConsoleFullscreen = "";
topbar.append(outputFullscreen, workspaceFullscreen);
pane.append(topbar);
root.dataset.mechConsoleMode = "docked";
root.dataset.mechOutputFullscreenActive = "false";
run("initializeReplComponentContract()");
const close = pane.querySelector("[data-mech-console-close]");
assert.ok(close, "legacy templates receive an internal Close control");
assert.equal(close.type, "button");
assert.match(close.textContent, /close/i, "Close has a visible text label");
assert.ok(close.getAttribute("aria-label"), "Close has an accessible label");
const outputLabel = outputFullscreen.querySelector("[data-mech-fullscreen-label]");
const workspaceLabel = workspaceFullscreen.querySelector("[data-mech-fullscreen-label]");
assert.ok(outputLabel?.textContent, "output fullscreen gains a visible label");
assert.ok(workspaceLabel?.textContent, "workspace fullscreen gains a visible label");
assert.equal(outputLabel.textContent, "Fullscreen output");
assert.equal(workspaceLabel.textContent, "Fullscreen workspace");
assert.notEqual(outputLabel.textContent, workspaceLabel.textContent, "fullscreen purposes are visibly distinct");
run("initializeReplComponentContract()");
assert.equal(pane.querySelectorAll("[data-mech-console-close]").length, 1);
assert.equal(pane.querySelector("[data-mech-console-close]"), close, "existing Close control is retained");
assert.equal(outputFullscreen.querySelectorAll("[data-mech-fullscreen-label]").length, 1);
assert.equal(workspaceFullscreen.querySelectorAll("[data-mech-fullscreen-label]").length, 1);
sandbox.fullscreenLabelProbe = outputFullscreen;
run('setFullscreenControlLabel(fullscreenLabelProbe, "Exit test fullscreen", "Exit test")');
assert.equal(outputFullscreen.getAttribute("aria-label"), "Exit test fullscreen");
assert.equal(outputFullscreen.getAttribute("title"), "Exit test fullscreen");
assert.equal(outputLabel.textContent, "Exit test");

const flushEvents = () => new Promise(resolve => setImmediate(resolve));
run("initializeConsoleToggle()");
window.scrollX = 13;
window.scrollY = 427;
page.scrollTop = 83;
sandbox.location.hash = "#retained-section";
run(`
  state.initialEncoded = 'retained-document-source';
  state.history = ['retained-command'];
  state.historyDraft = 'unfinished prompt';
  state.persistedLayout = { page: { owner: 'window', coordinateSpace: 'content-shell', x: 13, y: 427 } };
  state.pendingPagePosition = { x: 13, y: 427 };
`);
const preservedState = () => ({
  runtime: run("JSON.stringify({ source: state.initialEncoded, history: state.history, draft: state.historyDraft, layout: state.persistedLayout, pending: state.pendingPagePosition })"),
  scrollX: window.scrollX,
  scrollY: window.scrollY,
  pageScrollTop: page.scrollTop,
  hash: sandbox.location.hash,
});
for (const lifecycle of ["new", "starting", "ready", "failed", "stopped"]) {
  run(`state.runtimeLifecycle = ${JSON.stringify(lifecycle)}`);
  assert.equal(api.showOutput(), true);
  const before = preservedState();
  close.dispatchEvent(new Event("click"));
  await flushEvents();
  assert.equal(root.dataset.mechConsoleOpen, "false", `Close works while ${lifecycle}`);
  assert.equal(pane.hidden, true);
  assert.equal(toggle.getAttribute("aria-expanded"), "false");
  assert.deepEqual(preservedState(), before, `Close preserves document/navigation state while ${lifecycle}`);
  assert.equal(run("state.runtimeLifecycle"), lifecycle, "closing never stops or restarts the runtime");
  assert.equal(application.firstElementChild, applicationControl, "closing retains application DOM");
  toggle.dispatchEvent(new Event("click"));
  assert.equal(root.dataset.mechConsoleOpen, "true", "the existing opener can reopen the console");
}

// Closing serializes both fullscreen exits before hiding the pane. The output
// exit must not reveal its workspace as a side effect of an explicit Close.
const exits = [];
let finishOutputExit;
let finishWorkspaceExit;
sandbox.outputCloseProbe = {
  exit(options) {
    exits.push(["output", { ...options }]);
    return new Promise(resolve => { finishOutputExit = resolve; });
  },
};
sandbox.workspaceCloseProbe = {
  exit() {
    exits.push(["workspace"]);
    return new Promise(resolve => { finishWorkspaceExit = resolve; });
  },
};
run("state.outputFullscreenController = outputCloseProbe; state.consoleFullscreenController = workspaceCloseProbe");
const orderedClose = run("closeDocumentConsole()");
assert.deepEqual(exits, [["output", { revealWorkspace: false }]]);
assert.equal(pane.hidden, false, "pending output exit does not prematurely hide the pane");
finishOutputExit();
await flushEvents();
assert.deepEqual(exits, [["output", { revealWorkspace: false }], ["workspace"]]);
assert.equal(pane.hidden, false, "pending workspace exit does not prematurely hide the pane");
finishWorkspaceExit();
await orderedClose;
assert.equal(pane.hidden, true, "pane closes after both exits complete");

// A late Close continuation cannot mutate a replacement or disposed component.
for (const invalidation of ["generation", "disposed", "aborted"]) {
  for (const pendingExit of ["output", "workspace"]) {
    let finishPendingExit;
    let workspaceCalls = 0;
    const pending = () => new Promise(resolve => { finishPendingExit = resolve; });
    sandbox.outputCloseProbe = { exit: pendingExit === "output" ? pending : async () => {} };
    sandbox.workspaceCloseProbe = {
      exit() { workspaceCalls++; return pendingExit === "workspace" ? pending() : Promise.resolve(); },
    };
    run("state.runtimeLifecycle = 'ready'; state.runtimeEventController = new AbortController(); state.outputFullscreenController = outputCloseProbe; state.consoleFullscreenController = workspaceCloseProbe; setConsoleOpen(true)");
    const pendingClose = run("closeDocumentConsole()");
    await flushEvents();
    assert.equal(typeof finishPendingExit, "function");
    if (invalidation === "generation") run("state.runtimeGeneration++");
    if (invalidation === "disposed") run("state.runtimeLifecycle = 'disposed'");
    if (invalidation === "aborted") run("state.runtimeEventController.abort()");
    root.dataset.mechConsoleMode = "replacement-sentinel";
    const before = preservedState();
    finishPendingExit();
    await pendingClose;
    assert.equal(pane.hidden, false, `${invalidation} retires a Close awaiting ${pendingExit}`);
    assert.equal(root.dataset.mechConsoleOpen, "true");
    assert.equal(root.dataset.mechConsoleMode, "replacement-sentinel");
    assert.equal(workspaceCalls, pendingExit === "workspace" ? 1 : 0, "retired output exit never starts a workspace exit");
    assert.deepEqual(preservedState(), before);
  }
}

run("state.runtimeLifecycle = 'ready'; state.runtimeEventController = new AbortController(); state.outputFullscreenController = null; state.consoleFullscreenController = null");
root.dataset.mechConsoleMode = "docked";
run("initializeFullscreen(); initializeOutputFullscreen()");
root.dataset.mechPresentation = "output";
root.dataset.mechPresentationView = "output";
run("setOutputFullscreenVisualState(true)");
await run("closeDocumentConsole()");
assert.equal(root.dataset.mechPresentationView, "workspace", "closing leaves the forced-open output presentation");
assert.equal(root.dataset.mechOutputFullscreenActive, "false");
assert.equal(document.body.classList.contains("output-fullscreen"), false);
assert.equal(root.dataset.mechConsoleOpen, "false");
assert.equal(pane.hidden, true);
delete root.dataset.mechPresentation;
delete root.dataset.mechPresentationView;

// Real fullscreen controllers retain their separate labels and can both be
// retired through Close when the native Fullscreen API is unavailable.
run("setConsoleOpen(true)");
const dockedWorkspaceLabel = workspaceLabel.textContent;
const dockedOutputLabel = outputLabel.textContent;
workspaceFullscreen.dispatchEvent(new Event("click"));
await flushEvents();
assert.equal(root.dataset.mechConsoleMode, "button");
assert.equal(workspaceFullscreen.getAttribute("aria-pressed"), "true");
assert.notEqual(workspaceLabel.textContent, dockedWorkspaceLabel);
assert.equal(workspaceLabel.textContent, "Exit workspace");
assert.equal(workspaceFullscreen.getAttribute("aria-label"), "Minimize console workspace");
assert.equal(workspaceFullscreen.getAttribute("title"), workspaceFullscreen.getAttribute("aria-label"));
await run("closeDocumentConsole()");
assert.equal(root.dataset.mechConsoleMode, "docked");
assert.equal(workspaceFullscreen.getAttribute("aria-pressed"), "false");
assert.equal(workspaceLabel.textContent, dockedWorkspaceLabel);
assert.equal(pane.hidden, true);
run("setConsoleOpen(true)");
outputFullscreen.dispatchEvent(new Event("click"));
await flushEvents();
assert.equal(root.dataset.mechOutputFullscreenActive, "true");
assert.equal(outputFullscreen.getAttribute("aria-pressed"), "true");
assert.notEqual(outputLabel.textContent, dockedOutputLabel);
assert.equal(outputLabel.textContent, "Exit fullscreen");
assert.equal(outputFullscreen.getAttribute("aria-label"), "Exit fullscreen output");
assert.equal(outputFullscreen.getAttribute("title"), outputFullscreen.getAttribute("aria-label"));
await run("closeDocumentConsole()");
assert.equal(root.dataset.mechOutputFullscreenActive, "false");
assert.equal(outputFullscreen.getAttribute("aria-pressed"), "false");
assert.equal(outputLabel.textContent, dockedOutputLabel);
assert.equal(pane.hidden, true);

// Native requests that finish after Close must relinquish their browser
// fullscreen session without reopening the component or reclaiming ownership.
for (const [owner, control] of [["console", workspaceFullscreen], ["output", outputFullscreen]]) {
  let finishNativeEntry;
  let nativeExits = 0;
  pane.requestFullscreen = () => new Promise(resolve => {
    finishNativeEntry = () => {
      document.fullscreenElement = pane;
      document.dispatchEvent(new Event("fullscreenchange"));
      resolve();
    };
  });
  document.exitFullscreen = async () => {
    nativeExits++;
    document.fullscreenElement = null;
    document.dispatchEvent(new Event("fullscreenchange"));
  };
  run("setConsoleOpen(true)");
  control.dispatchEvent(new Event("click"));
  assert.equal(typeof finishNativeEntry, "function");
  assert.equal(run("state.fullscreenRequest?.owner"), owner);
  await run("closeDocumentConsole()");
  assert.equal(pane.hidden, true);
  assert.equal(run("state.fullscreenRequest"), null, "Close releases pending fullscreen ownership");
  finishNativeEntry();
  await flushEvents();
  assert.equal(nativeExits, 1, `${owner}: canceled native entry is relinquished exactly once`);
  assert.equal(document.fullscreenElement, null);
  assert.equal(pane.hidden, true, `${owner}: stale entry cannot reopen the console`);
  assert.equal(root.dataset.mechConsoleOpen, "false");
  assert.equal(root.dataset.mechConsoleMode, "docked");
  assert.equal(root.dataset.mechOutputFullscreenActive, "false");
  assert.equal(control.getAttribute("aria-pressed"), "false");

  // A subsequent click remains a fresh native entry; Close also retires an
  // already established native session through the owning controller.
  run("setConsoleOpen(true)");
  control.dispatchEvent(new Event("click"));
  finishNativeEntry();
  await flushEvents();
  assert.equal(document.fullscreenElement, pane);
  assert.equal(control.getAttribute("aria-pressed"), "true");
  await run("closeDocumentConsole()");
  assert.equal(nativeExits, 2, `${owner}: established native entry exits exactly once`);
  assert.equal(document.fullscreenElement, null);
  assert.equal(control.getAttribute("aria-pressed"), "false");
  assert.equal(pane.hidden, true);
}

// Backtick is the same Close operation as the visible button, including an
// established native workspace session; it must not leave a hidden owner.
let keyboardNativeExits = 0;
pane.requestFullscreen = async () => {
  document.fullscreenElement = pane;
  document.dispatchEvent(new Event("fullscreenchange"));
};
document.exitFullscreen = async () => {
  keyboardNativeExits++;
  document.fullscreenElement = null;
  document.dispatchEvent(new Event("fullscreenchange"));
};
sandbox.requestAnimationFrame = callback => { callback(); return 1; };
run("initializeConsoleKeyboardToggle(); setConsoleOpen(true)");
workspaceFullscreen.dispatchEvent(new Event("click"));
await flushEvents();
assert.equal(document.fullscreenElement, pane);
const closingBacktick = new Event("keydown", { cancelable: true });
closingBacktick.key = "`";
document.dispatchEvent(closingBacktick);
await flushEvents();
assert.equal(closingBacktick.defaultPrevented, true);
assert.equal(keyboardNativeExits, 1, "backtick exits native workspace ownership");
assert.equal(document.fullscreenElement, null);
assert.equal(root.dataset.mechConsoleMode, "docked");
assert.equal(root.dataset.mechConsoleOpen, "false");
assert.equal(pane.hidden, true);
const openingBacktick = new Event("keydown", { cancelable: true });
openingBacktick.key = "`";
document.dispatchEvent(openingBacktick);
await flushEvents();
assert.equal(root.dataset.mechConsoleOpen, "true", "backtick can reopen after closing native fullscreen");
assert.equal(pane.dataset.mechConsoleActivePanel, "console");
workspaceFullscreen.dispatchEvent(new Event("click"));
await flushEvents();
assert.equal(document.fullscreenElement, pane, "workspace can enter native fullscreen again");
assert.equal(workspaceFullscreen.getAttribute("aria-pressed"), "true");
await run("closeDocumentConsole()");
assert.equal(keyboardNativeExits, 2);
assert.equal(document.fullscreenElement, null);
delete pane.requestFullscreen;
delete document.exitFullscreen;
delete document.fullscreenElement;

// Output fullscreen hides the article and can make the browser clamp its
// scroll offset to zero. Restore the captured canonical content position,
// never that collapsed offset, even if the responsive scroll owner changes.
const shell = new Element("content-shell");
shell.scrollHeight = 3000;
shell.clientHeight = 600;
contentShell = shell;
const scrollWrites = [];
const articleVisible = () => root.dataset.mechOutputFullscreenActive !== "true" &&
  !(root.dataset.mechPresentation === "output" && root.dataset.mechPresentationView !== "workspace");
const writeScroll = (owner, x, y) => {
  const visible = articleVisible();
  scrollWrites.push({ owner, x, y, visible });
  if (owner === "window") {
    window.scrollX = visible ? x : 0;
    window.scrollY = visible ? y : 0;
  } else {
    shell.scrollLeft = visible ? x : 0;
    shell.scrollTop = visible ? y : 0;
  }
};
window.scrollTo = (x, y) => writeScroll("window", x, y);
shell.scrollTo = (x, y) => writeScroll("content-shell", x, y);
// A layout read after presentation hides the article exposes its clamped
// offset, so the tests also enforce capture before the visibility mutation.
sandbox.getComputedStyle = element => {
  if (element === shell && !articleVisible()) {
    window.scrollX = window.scrollY = 0;
    shell.scrollLeft = shell.scrollTop = 0;
  }
  return element.style;
};
let scheduledSaves = 0;
sandbox.setTimeout = () => { scheduledSaves++; return 1; };
sandbox.clearTimeout = () => {};
const prepareArticlePosition = (owner = "window") => {
  shell.style.overflowY = owner === "window" ? "visible" : "auto";
  shell.offsetLeft = 17;
  shell.offsetTop = 126;
  shell.scrollLeft = 8;
  shell.scrollTop = 700;
  window.scrollX = 25;
  window.scrollY = 826;
  scrollWrites.length = 0;
  run("state.pendingPagePosition = null; state.pagePositionSaveTimer = null; setConsoleOpen(true)");
};
const canonicalPosition = owner => ({ owner, coordinateSpace: "content-shell", x: 8, y: 700 });
const plain = value => JSON.parse(JSON.stringify(value));
for (const [exit, initialOwner, finalOwner, originShift] of [
  ["normal", "window", "window", 40],
  ["close", "window", "window", 0],
  ["close", "window", "content-shell", 0],
  ["normal", "content-shell", "window", 40],
]) {
  prepareArticlePosition(initialOwner);
  outputFullscreen.dispatchEvent(new Event("click"));
  await flushEvents();
  const captured = run("state.outputFullscreenPagePosition");
  assert.deepEqual(plain(captured), canonicalPosition(initialOwner));
  window.scrollX = window.scrollY = shell.scrollLeft = shell.scrollTop = 0;
  run("setOutputFullscreenVisualState(true); setOutputFullscreenVisualState(true)");
  assert.equal(run("state.outputFullscreenPagePosition"), captured, "repeated synchronization never recaptures clamped scroll");
  run("schedulePagePositionSave()");
  assert.equal(scheduledSaves, 0, "hidden-article scroll events do not queue a position save");
  assert.equal(run("state.pendingPagePosition"), null);
  run("savePagePosition()");
  assert.deepEqual(plain(run("state.persistedLayout.page")), canonicalPosition(initialOwner), "pagehide/default saves retain the pre-fullscreen position");
  assert.equal(scrollWrites.length, 0, "fullscreen does not restore while the article is hidden");
  shell.offsetTop += originShift;
  shell.style.overflowY = finalOwner === "window" ? "visible" : "auto";
  if (exit === "normal") {
    outputFullscreen.dispatchEvent(new Event("click"));
    await flushEvents();
  } else {
    await run("closeDocumentConsole()");
  }
  assert.deepEqual(scrollWrites, [{
    owner: finalOwner,
    x: finalOwner === "window" ? 25 : 8,
    y: finalOwner === "window" ? 826 + originShift : 700,
    visible: true,
  }], `${exit}: canonical coordinates restore onto the current ${finalOwner} owner`);
  assert.equal(run("state.outputFullscreenPagePosition"), null);
  assert.equal(root.dataset.mechConsoleOpen, exit === "normal" ? "true" : "false");
  assert.deepEqual(plain(run("state.persistedLayout.page")), canonicalPosition(initialOwner));
}

// Output-first presentation can still hide the article after its fullscreen
// flag clears. Keep the snapshot until the workspace is genuinely visible.
for (const exit of ["normal", "close"]) {
  root.dataset.mechPresentation = "output";
  root.dataset.mechPresentationView = "workspace";
  prepareArticlePosition();
  outputFullscreen.dispatchEvent(new Event("click"));
  await flushEvents();
  const captured = run("state.outputFullscreenPagePosition");
  assert.deepEqual(plain(captured), canonicalPosition("window"), "presentation entry captures before hiding the article");
  window.scrollX = window.scrollY = 0;
  run("setOutputFullscreenVisualState(false)");
  assert.equal(root.dataset.mechPresentationView, "output");
  assert.equal(run("state.outputFullscreenPagePosition"), captured, "hidden presentation defers restoration");
  assert.equal(scrollWrites.length, 0);
  if (exit === "normal") {
    await run("state.outputFullscreenController.exit()");
  } else {
    await run("closeDocumentConsole()");
  }
  assert.equal(root.dataset.mechPresentationView, "workspace");
  assert.deepEqual(scrollWrites, [{ owner: "window", x: 25, y: 826, visible: true }]);
  assert.equal(run("state.outputFullscreenPagePosition"), null);
  assert.equal(root.dataset.mechConsoleOpen, exit === "normal" ? "true" : "false");
}
prepareArticlePosition();
run("setDocumentPresentationView('output')");
assert.deepEqual(plain(run("state.outputFullscreenPagePosition")), canonicalPosition("window"), "direct presentation changes capture before hiding the article");
window.scrollX = window.scrollY = 0;
run("retireFullscreenVisualState()");
assert.equal(root.dataset.mechPresentationView, "workspace");
assert.deepEqual(scrollWrites, [{ owner: "window", x: 25, y: 826, visible: true }], "retiring fullscreen restores article navigation too");
assert.equal(run("state.outputFullscreenPagePosition"), null);
delete root.dataset.mechPresentation;
delete root.dataset.mechPresentationView;
assert.equal(imports, 0, "all controls remain independent of WASM startup");

run("state.runtimeLifecycle = 'disposed'");
assert.throws(() => api.showOutput(), error => error.code === "MECH_DOCUMENT_DISPOSED");
console.log("PASS: output hosting, startup-independent Close controls, fullscreen exit ownership and labels, navigation preservation, and explicit output streams.");
