import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";

// A presentation page deliberately has no encoded document, console, or WASM
// bindings. Starting it must still activate the shared navigation lifecycle.
const root = { dataset: {}, querySelector: () => null };
const controller = { dataset: { mechDocumentMode: "presentation" } };
const page = { dataset: {} };
const window = new EventTarget();
const document = {
  documentElement: page,
  querySelector(selector) {
    if (selector === "script[data-mech-document-controller]") return controller;
    if (selector === ".mech-root, .mech-document") return root;
    return null;
  },
  querySelectorAll: () => [],
  getElementById: () => null,
};
let readyEvents = 0;
let imports = 0;
window.addEventListener("mech:presentation-ready", () => readyEvents++);
const sandbox = vm.createContext({
  document,
  window,
  AbortController,
  CustomEvent,
  history: { scrollRestoration: "auto" },
  location: {
    origin: "https://example.test",
    pathname: "/article",
    search: "",
  },
  localStorage: { getItem: () => null },
  console,
});
new vm.Script(
  readFileSync(new URL("../include/document.js", import.meta.url), "utf8"),
  {
    importModuleDynamically() {
      imports++;
      throw new Error("a presentation page must not import WASM");
    },
  },
).runInContext(sandbox);
await new Promise(resolve => setImmediate(resolve));

assert.equal(imports, 0);
assert.equal(readyEvents, 1);
assert.equal(page.dataset.mechDocumentStatus, "ready");
assert.equal(root.dataset.mechDocumentStatus, "ready");
assert.equal(sandbox.history.scrollRestoration, "manual");
assert.equal(vm.runInContext("state.document", sandbox), null);
assert.equal(vm.runInContext("state.console", sandbox), null);
assert.equal(vm.runInContext("state.animationFrame", sandbox), null);
assert.equal(vm.runInContext("state.runtimeLifecycle", sandbox), "presentation");
assert.throws(
  () => sandbox.MechDocumentController.source(),
  error => error.code === "MECH_DOCUMENT_NOT_READY",
);
window.dispatchEvent(new Event("mech:document-layout-refresh"));
assert.equal(page.dataset.mechDocumentStatus, "ready");
console.log("Document presentation startup passed without a WASM runtime.");

// Exercise the real shared TOC controller at the coordinates produced by
// scrollIntoView, including each of its desktop/mobile scroll owners.
const classList = () => {
  const names = new Set();
  return {
    add: (...values) => values.forEach(value => names.add(value)),
    remove: (...values) => values.forEach(value => names.delete(value)),
    contains: value => names.has(value),
  };
};
class TocItem {
  constructor() { this.classList = classList(); }
  querySelector() { return this.link; }
  contains(link) { return link === this.link || link === this.nestedLink; }
}
class TocLink extends EventTarget {
  constructor(id, item) {
    super();
    this.id = id;
    this.item = item;
    this.classList = classList();
    this.attributes = new Map();
    item.link = this;
  }
  getAttribute(name) { return name === "href" ? `#${this.id}` : this.attributes.get(name); }
  setAttribute(name, value) { this.attributes.set(name, value); }
  removeAttribute(name) { this.attributes.delete(name); }
  closest(selector) { return selector === "li" ? this.item : toc; }
}
const items = [new TocItem(), new TocItem(), new TocItem()];
const links = items.map((item, i) => new TocLink(String(i + 1), item));
const nestedItem = new TocItem();
const nestedLink = new TocLink("2.1", nestedItem);
items[1].nestedLink = nestedLink;
nestedItem.parentElement = { closest: () => items[1] };
const toc = {
  scrollHeight: 200,
  clientHeight: 200,
  querySelector: () => ({ children: items }),
  querySelectorAll: () => [...items, nestedItem],
};
const shell = Object.assign(new EventTarget(), {
  scrollTop: 500,
  scrollHeight: 3000,
  clientHeight: 600,
  style: { overflowY: "auto", scrollPaddingTop: "0px" },
  getBoundingClientRect: () => ({ top: 52 }),
});
page.scrollHeight = 3000;
page.style = { scrollPaddingTop: "0px" };
window.scrollY = 500;
window.innerHeight = 600;
let contained = true;
const targets = [...links, nestedLink].map(link => ({
  id: link.id,
  top: 1000,
  style: { scrollMarginTop: "0px" },
  getBoundingClientRect() { return { top: this.top }; },
  closest: () => contained ? shell : null,
}));
targets[0].top = -400;
document.querySelectorAll = selector => selector.includes("a[href^='#']")
  ? [links[0], links[1], nestedLink, links[2]]
  : [];
document.getElementById = id => targets.find(target => target.id === id) || null;
document.addEventListener = () => {};
document.removeEventListener = () => {};
sandbox.HTMLLIElement = TocItem;
sandbox.getComputedStyle = element => element.style;
for (const owner of ["content-shell", "window"]) {
  contained = owner === "content-shell";
  const viewportTop = contained ? 52 : 0;
  const scrollStyle = contained ? shell.style : page.style;
  for (const [margin, padding, paddingPixels] of [
    [68, "0px", 0],
    [0, "48px", 48],
    [68, "10%", 60],
    [0, "auto", 0],
  ]) {
    targets[1].style.scrollMarginTop = `${margin}px`;
    scrollStyle.scrollPaddingTop = padding;
    const activationOffset = Math.max(20, margin + paddingPixels + 1);
    targets[1].top = viewportTop + activationOffset + 2;
    vm.runInContext("initializeToc()", sandbox);
    assert.equal(links[0].classList.contains("active"), true, `${owner}: preceding section before anchor`);
    targets[1].top = viewportTop + margin + paddingPixels;
    vm.runInContext("initializeToc()", sandbox);
    assert.equal(links[1].classList.contains("active"), true, `${owner}: landed heading becomes active`);
    assert.equal(links[1].getAttribute("aria-current"), "location");
  }
  targets[3].top = viewportTop + 150;
  vm.runInContext("initializeToc()", sandbox);
  assert.equal(nestedLink.getAttribute("aria-current"), "location", "nested activation is preserved");
  targets[3].top = 1000;
}
console.log("TOC activation follows scroll margin and padding for shell and window scrolling.");
