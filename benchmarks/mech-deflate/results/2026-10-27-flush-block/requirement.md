# Connected block writer qualification

The first block source was prepared on `ac7f16c8aa4477783251a44ecbb0fe47267b19d1` (#960). The final source is published with the complete compressor above #967. The receipts preserve their actual historical revisions and configurations; they are not relabeled as runs of the final compiler.

Reference package: `671077457ff7ef3a3d44caf715a3fc595addf379`, compiler `d99c7b838fb05bb1047eb0c18d4d06d0f86f345d`. Attribution is in `hosts/gpu/tests/fixtures/native/deflate-huffman-LICENSES.md`.

`block-flush` connects precode construction, stored/static/dynamic costing and selection, full-length code tables, path/sequence payload writing, EOB and returned buffer/bit state. Three maintained tests execute a fixed A block, consecutive blocks producing AA, and a supplied-code dynamic block for 200 A bytes. Each covers source-derived original/decoded artifacts and both sequence and path representations. Expected byte/bit state comes from the preserved oracle programs. Independently draining the remaining bits produces the original input.

The consecutive case preserves output and pending bits across its two calls. The dynamic case emits 37 whole bytes and four pending bits. `flush-dynamic-connected.json` records the passing native dynamic qualification. Existing component allowances were retained; complete compressor execution separately uses the explicit maximum-budget mode.

The initial packed-table padding expression exposed the selected-row computed-extent gap: one branch returned a concatenation and the other the unchanged row. That generic compiler fix is already reviewed in the preceding stack. The source here keeps the reference conditional padding and packed representation. Earlier failed and successful selected-matrix receipts remain historical evidence of that integration path; they are not new unresolved compiler work.

These are writer-component cases with supplied models and paths, not complete compression or final benchmark timings. Full collection/model selection/controller qualification is documented separately in `../2026-10-27-complete-compression/requirement.md`. Whole-compressor dynamic/stored selection and multi-block correctness remain to be established on larger inputs.

The canonical dynamic golden is `hosts/gpu/tests/fixtures/native/deflate-flush-dynamic-reference.json`, SHA256 `139383b0ee76722af577c26eaf18516da96713a4c3a93538521113b62375b247`; the duplicate generated copy is omitted.
