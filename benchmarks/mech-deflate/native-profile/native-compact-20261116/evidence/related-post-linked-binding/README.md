# Related-post benchmark same-Object binding

The hosted architecture job for PR #993 at b72b7419 stops on the compiled
module restore at line 104 of the after-4 comparison host. Formatting and
compiler-planning quarantine got through their checks before this failure.
[Exact hosted failure](https://github.com/mech-lang/mech/actions/runs/44080079325/job/134028828788)

The comparison host restores the module at the start of its separate setup
timer. Rust resolves the declarations, factory catalog, and source include
from a build directory beside the runner. The local host builder takes an
independent bundle argument, hashes the files in that bundle into a
post-build receipt, and tells the linker to consume the Object in that
bundle. The builder has no pre-link binding between the included directory
and the linked bundle and no producer pin to check these inputs against.
The recorded after-4 build used the same directory for both, as shown by
its immutable source and Object receipt. Four comparison hosts have the
same source digest. Both historical inventories pin those exact bytes.
[Reviewed host](https://github.com/mech-lang/mech/blob/b72b7419b39b22765fe1eed6b5c636e63964b905/benchmarks/related-post-gen/hosts/after-4.rs#L100)
· [Host builder](https://github.com/mech-lang/mech/blob/b72b7419b39b22765fe1eed6b5c636e63964b905/benchmarks/related-post-gen/build-host.py#L74)
· [Historical receipt](https://github.com/mech-lang/mech/blob/b72b7419b39b22765fe1eed6b5c636e63964b905/benchmarks/related-post-gen/review/qualification/after-4/runner-aot.build.json)

The supported application generator already derives the module symbol from
the emitted Object, pins the source, bytecode, Object, and generated inputs,
and checks those pins before linking. Its checked binding is consumed by a
managed launcher with typed JSON drafts and canonical snapshot output. The
related-post adapter instead prepares its own immutable input values and
post views, then measures native evaluation and RelatedPosts materialization
under its documented boundaries. The currently public binding API exposes
the managed launcher. Restoration of the separate artifact is private.
[Required module provenance](https://github.com/mech-lang/mech/blob/b72b7419b39b22765fe1eed6b5c636e63964b905/hosts/gpu/src/native_artifact/program/linked.rs#L14)
· [Generated pins](https://github.com/mech-lang/mech/blob/b72b7419b39b22765fe1eed6b5c636e63964b905/hosts/gpu/src/native_artifact/templates/linked_build.rs.in)
· [Managed launch](https://github.com/mech-lang/mech/blob/b72b7419b39b22765fe1eed6b5c636e63964b905/hosts/gpu/src/native_artifact/launch/managed.rs#L80)

A producer-owned migration needs an explicitly named checked adapter
entry that uses the generated pin verification and the already-established
compiled module binding. The adapter must keep the catalog selected by
that same build. It can then preserve the benchmark's restore, input,
evaluation, and publication boundaries while the original runner and
receipts remain separately verifiable. A new binary would need its own
linked identity and benchmark receipt.

The focused source check recorded five comparison hosts, the tracking
allocator, 22 old-Object probes, and three runtime-only qualification
sources matching the same audit rule. The old-Object probes exercise the
historical failure-to-enter behavior through both loaders. Those
independently active checks and archive identities need their own
migration. The recorded results here come from the hosted check and
direct Git-blob inspection.
[Exact source findings](blocker.json)

~~~sh
python3 -B inspect.py /path/to/mech
~~~
