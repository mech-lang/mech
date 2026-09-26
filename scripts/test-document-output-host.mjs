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
    this.classList = {
      contains: name => this.className.split(/\s+/).includes(name),
      add: name => { if (!this.classList.contains(name)) this.className += ` ${name}`; },
      remove: name => { this.className = this.className.split(/\s+/).filter(value => value !== name).join(" "); },
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
    return this.children.flatMap(child => child instanceof Element
      ? [...(child.matches(selector) ? [child] : []), ...child.querySelectorAll(selector)] : []);
  }
  querySelector(selector) { return this.querySelectorAll(selector)[0] || null; }
  closest(selector) { return this.matches(selector) ? this : this.parentElement?.closest(selector) || null; }
}

const root = new Element();
const page = new Element();
const controller = { dataset: { mechDocumentMode: "presentation" } };
const window = new EventTarget();
const document = {
  documentElement: page,
  body: new Element(),
  querySelector: selector => selector === "script[data-mech-document-controller]" ? controller
    : selector === ".mech-root, .mech-document" ? root : null,
  querySelectorAll: () => [],
  getElementById: () => null,
  createElement: () => new Element(),
};
let imports = 0;
const sandbox = vm.createContext({
  document, window, AbortController, CustomEvent, Node: { TEXT_NODE: 3 }, console,
  history: { scrollRestoration: "auto" },
  location: { origin: "https://example.test", pathname: "/article", search: "" },
  localStorage: { getItem: () => null },
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

run("state.runtimeLifecycle = 'disposed'");
assert.throws(() => api.showOutput(), error => error.code === "MECH_DOCUMENT_DISPOSED");
console.log("PASS: application output hosting, readiness-independent drawer selection, workspace preservation, and explicit output streams.");
