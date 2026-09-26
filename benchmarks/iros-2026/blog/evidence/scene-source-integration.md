# Mech scene source integration checks

The workshop's `source/scene.mec` supplies simulated camera bearings and scene
tables. JavaScript supplies input packets and renders the scene host's snapshot;
it does not implement the observation model, covariance geometry, or trail
recurrence.

The native integration test is
`scene_program::tests::workshop_camera_and_scene_execute_from_mech_source` in
`src/wasm/src/scene_program.rs`. It passed under:

```sh
cargo test -p mech-wasm --no-default-features --features browser_compute_canary scene_program::tests --lib -j 2
```

All eight scene/interface tests passed. The additional transport checks cover
one-element matrices, signed zero, and scene batches of 1, 256, 4,096, and 65,536
lanes.

The source test checks the deterministic bearing formula, the 256-lane output,
prepare-turn idempotence, one advancement per accepted commit, independent
350-point truth and estimate trails, landmark selection, the inclusive camera
range boundary, loss of visibility, and reentry. Browser/WASM execution is a
separate check and is not established by this native test alone.

The built WebAssembly package also passed the local Node smoke test:

```sh
node benchmarks/iros-2026/blog/test-document-runtime.mjs
```

That test executes the shipped Mech camera/scene source for 40 accepted turns
with 256 filter lanes, checks scene advancement and the dotted trail table, and
compares the live numerical kernel with the retained stabilized reference. The
original historical fixtures keep their original input interfaces and source
hash checks; their equivalence and invalid-input rollback/recovery checks also
pass. This exercises the browser package in Node, not browser rendering or
WebGPU.

The retained static-illustration generator also passed against the rebuilt WASM
package with one filter:

```sh
node benchmarks/iros-2026/blog/hero.mjs /private/tmp/iros-scene-hero.svg
```

It executes 40 accepted Mech sensor/filter/scene turns and embeds the numerical
and scene source hashes in the generated SVG. This diagram is not the article's
current Pittsburgh hero photograph.

The exact downloadable Rust programs passed their native execution check against
the updated EKF source:

```sh
node benchmarks/iros-2026/blog/test-rust-examples.mjs --native --target-dir /private/tmp/mech-iros-workshop-20260924/target
```

The three programs compile and run the JIT interface, save an AOT bundle, and
reload that bundle. Their four synchronous `μ` results agree within the test's
f32 tolerance. They bind `bearing`, while the source's `u` and `m` declarations
provide the default motion, visible-measurement flag, and landmark. The native
archived example files were not modified.

## Integration issues found

- A source expression match over the range predicate selected its arm during
  construction in this runtime. Changing the range input did not switch the
  selected arm on later resident turns. The final scene source does not use
  that expression for its live visibility gate. This observation concerns the
  tested expression-match path, not all pattern matching or state machines.
- The public type-conversion policy rejects implicit Boolean-to-number
  conversion. No conversion policy was relaxed for this example.
- The dynamically sized `1..=instances` range was not accepted by this resident
  layout. The host instead supplies a fixed-shape vector of lane indices at
  construction; the Mech source calculates all bearing and noise values from
  that data. Changing batch size constructs a new scene instance.
- Mutable trail states require independent initializer expressions; binding
  both directly to the same immutable matrix register is rejected during
  artifact construction. The final source uses separate initializers.

The range gate uses supported numerical nodes. For signed distance excess
`e = distance - range`, the positive part is `p = (e + abs(e)) / 2` and the gate is
`1 - ceil(p / (abs(e) + 1))`. It is exactly one for `e <= 0` and zero for `e > 0`
over the example's finite world-coordinate inputs. There is no fade or added
range tolerance. Native tests exercise the boundary and subsequent input changes.
