import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";

// Execute the shared scene renderer without starting the document runtime.
const source = readFileSync(new URL("../../../include/document.js", import.meta.url), "utf8");
const start = source.indexOf("function renderSceneSvg(scene) {");
const end = source.indexOf("function outputContentElement(content)", start);
assert.ok(start >= 0 && end > start);
assert.match(source, /MechDocumentController = Object\.freeze\(\{\s*renderSceneSvg,/);
assert.match(source, /body\.append\(renderSceneSvg\(scene\)\)/);

class SvgElement {
  constructor(tag) {
    this.tag = tag;
    this.attributes = new Map();
    this.dataset = {};
    this.children = [];
    this.classNames = new Set();
    this.classList = { add: name => this.classNames.add(name) };
  }
  setAttribute(name, value) { this.attributes.set(name, String(value)); }
  getAttribute(name) { return this.attributes.get(name); }
  append(element) { this.children.push(element); }
}
const context = vm.createContext({ document: {
  createElementNS(namespace, tag) {
    assert.equal(namespace, "http://www.w3.org/2000/svg");
    return new SvgElement(tag);
  },
} });
vm.runInContext(source.slice(start, end), context);
const strip = {
  id: "trail", positions: [[1, 2], [3, 4]], stroke: "#8a794d", stroke_width: 1,
  line_cap: "round", line_join: "round", opacity: 0.7, closed: false,
};
const scene = { width: 200, height: 100, background: "#0c1114", line_strips: [strip] };
const svg = context.renderSceneSvg(scene);
assert.equal(svg.tag, "svg");
assert.equal(svg.getAttribute("viewBox"), "0 0 200 100");
assert.equal(svg.dataset.mechScenePointerSurface, "true");
assert.equal(svg.dataset.mechRichScene, "true");
assert.ok(svg.classNames.has("mech-repl-scene"));
assert.equal(svg.children[0].getAttribute("fill"), "#0c1114");
assert.equal(svg.children[1].getAttribute("fill"), "none", "older snapshots remain unfilled");
assert.equal(svg.children[1].getAttribute("stroke-dasharray"), "", "older snapshots remain solid");
assert.equal(svg.children[1].getAttribute("points"), "1,2 3,4");

strip.fill = "rgba(183, 126, 182, 0.22)";
strip.stroke_dasharray = [0, 5];
strip.closed = true;
const styled = context.renderSceneSvg(scene).children[1];
assert.equal(styled.getAttribute("fill"), strip.fill);
assert.equal(styled.getAttribute("stroke-dasharray"), "0 5");
assert.equal(styled.getAttribute("stroke-linecap"), "round");
assert.equal(styled.getAttribute("points"), "1,2 3,4 1,2");
assert.equal(strip.positions.length, 2, "rendering does not mutate the scene snapshot");
strip.stroke_dasharray = [0];
assert.equal(context.renderSceneSvg(scene).children[1].getAttribute("stroke-dasharray"), "0");
console.log("PASS: shared scene SVG rendering, pointer surfaces, fill, dotted paths, and legacy defaults.");
