# Scene line-strip styles

Scene `line-strips` accept records, tuples of records, or tables. Each row retains
its `positions` matrix as one path. In addition to the existing required fields,
two optional columns are available:

| Mech field | Value | Default |
| --- | --- | --- |
| `fill` | Paint string or 24-bit numeric RGB color | `"none"` |
| `stroke-dasharray` | Non-negative, finite f64 row or column vector | Solid stroke |

Dash lengths alternate between painted and unpainted segments, measured in scene
coordinates. For example, `[0 5]` with `line-cap: "round"` produces dots. An odd
number of lengths repeats to form an even pattern, and an all-zero pattern is
solid, following SVG and canvas behavior. A fill uses the path interior; `closed`
also connects the final point back to the first for the outline. Both paints use
the element's `opacity`; an RGBA fill can make the interior translucent while
leaving the outline opaque.

The browser's SVG and canvas backends and the document's generic scene output
renderer support both fields. Native scene registries retain them in snapshots.
Serialized scene JSON uses `stroke_dasharray`, consistent with other Rust field
names such as `stroke_width`. Older JSON snapshots that omit these additions
remain unfilled with solid strokes.

Browser applications can present a snapshot using
`MechDocumentController.renderSceneSvg(scene)`, the same renderer used by rich
scene output. This returns an SVG element; geometry and style are supplied by the
Mech scene tables rather than recomputed by this rendering helper.

Focused checks:

```sh
cargo test -p mech-scene --features browser,native
node hosts/scene/tests/document-renderer.mjs
```
