# Closed native library contract

The proposed entry accepts a checked closed Program and an exact target/runtime
contract. It retains the existing checked Program and native-plan
representations. Cranelift emits the object. This design describes what a later
library wrapper must guarantee to callers.

## Invocation

The caller supplies initialized, correctly aligned storage for each admitted input
layout. The library borrows it for one synchronous call and releases the borrow
before returning. An input descriptor covers only the initialized region even
when the underlying allocation has spare capacity. The library may reuse an
owner only when the invocation has established exclusive ownership under the
checked runtime contract.

Initial layout evidence covers scalars and contiguous initialized numeric
views, with the level-12 codec exercising borrowed and owned U8 views.
Named ports and their scalar or element kinds, element widths and layout checks
come from the canonical artifact. A later wrapper must define an admitted subset
and reject incompatible kinds, alignment, lengths and sizes before dereferencing
input or publishing output. Aggregate or strided transport, foreign allocators,
and resource capabilities need explicit evidence before admission.

On success, one result owns each transferred allocation. The owner retains the
allocation identity, logical initialized length, capacity and the release
operation for the same runtime and allocator. Scalar results are values. A
returned view cannot extend the borrow of caller-owned storage. The release
operation retires the transferred owner exactly once and must be usable after
the invocation frame has gone away.

Caller-visible input and borrowed-output aliases remain readable throughout the
call and retain their contents. Copy-on-write or checked exclusive ownership
must precede a mutation that could affect an older alias. The wrapper must
reject prohibited overlap before the first mutation. A caller remains
responsible for the validity and lifetime of the storage it supplies.

An invocation stages output until every selected operation, check and
finalization succeeds. Failure returns the first applicable ordered status and
source origin where available. Cleanup retires temporary owners, drops only
initialized values and leaves no partial result accessible. Allocation denial,
size overflow and partial initialization use the same cleanup and publication
rules. A later valid call can reuse the caller-facing context after an ordinary
checked failure.

Independent compatible calls use separate invocation heaps and mutable
accounting. They may share immutable code and constants while the caller
maintains the documented alias and lifetime restrictions. Sharing or concurrently
reusing a mutable context requires a separately specified contract.

Closed entry supports admitted computation, checked local mutation and its
bounded private allocations. Provider I/O, callbacks with host authority and
unbounded or unsupported effects require another entry contract. Each admitted
target and runtime pair must establish the stack, layout, work, allocation and
publication properties of the exact linked artifact. Load and entry admission
verify target, object/descriptor binding and runtime ABI compatibility before
invocation.

## Guarantee and test-gap matrix

| Area | Existing evidence | Proposed guarantee | Implementation and test gap |
| --- | --- | --- | --- |
| Input borrowing | Compact closed workers use initialized immutable U8 views. Output and alias fixtures exercise input preservation. | Inputs stay borrowed for the call and remain unmodified. The library retains no external borrow. | Exercise foreign buffers, empty input, lifetime boundaries, misalignment, invalid geometry and checked size multiplication through the library entry. |
| Output ownership | Compact runtime `Heap::take_u8` transfers only a validated owner. The timed deployment exercises encoder release. | Committed outputs have one documented releasing owner and keep their exact logical length. | Define the caller-facing ownership token and release operation. Test transfer, rollback, release after return, and rejection of mismatched runtime or allocator. |
| Layout | Canonical port metadata identifies kinds and sizes. The retained six-word U8 transport is exercised by linked codec tests. | Entry admits only its declared checked layout set and preserves element kinds and initialized extents. | Choose the public layout subset and test each supported layout, zero length, incompatible kind, alignment, size and overflow. |
| Aliasing | Retained codec gates check borrowed input and prior output aliases. Compact owner checks cover adjacent permitted moves and refcounts. | Aliases to borrowed and older values survive copy-on-write and checked failure. Invalid overlaps are rejected before writes. | Specify and exercise every allowed and denied overlap, including slices and prior output ownership. |
| Allocation failure | Compact runtime tests cover bookkeeping and supported allocation errors. The linked native gate includes checked failure/recovery. | Denied allocation or overflow exposes the first checked failure with complete cleanup. | Inject failure at each allocation and transfer boundary, including after partial initialization and before publication. |
| Initialization and cleanup | Native preparation verifies all-path initialization of exposed lanes. The worker checks that temporary private owners retire. | Failed execution exposes no partial result and releases each initialized temporary exactly once. | Exercise partial aggregate initialization and nested failure cleanup under the closed library boundary. |
| Retry | Linked codec tests exercise failed and successful calls on reusable state. The reconciled source and Wasm tests cover atomic set rejection and recovery. | A valid call can follow a checked failure without carrying over private owners, budgets or staged outputs. | Test repeated denial/success and failures during release and final publication across admitted layouts. |
| Concurrency | Compiler jobs own separate contexts and publish in input order. Runtime alias tests use synchronous invocations. | Calls with independent mutable state and lawful input borrows can execute concurrently against immutable code. | Specify context sharing and reentrancy. Stress parallel invocation, teardown ordering and reclamation while another call remains active. |
| Effects | Canonical plans distinguish managed provider execution from closed computation. | Entry admits only the declared checked effects and reserves authority for the host contract. | Test admission denial for unsupported providers, callbacks and unbounded capabilities. |
| Resource bounds | The managed Program route carries finite stack, work and runtime limits. Compact diagnostics record measured stack and selected implementation sizes. | Entry enforces specified per-call resource bounds for its target, runtime and admitted layout. | Define the compact entry's finite resource admission and establish failure accounting and bounds under nested execution. |
| Target/runtime | The current packet verifies the Apple arm64 object, descriptor and runtime support. Scalar emission has separate direct native coverage. | Linkage requires the exact compatible target, codegen ABI, runtime and allocation contract. | Define library admission based on those identities and test mismatches, including other operating systems and runtime versions. |

The compact diagnostic remains useful for isolated compiler and codec evidence.
That evidence leaves the library's caller-facing ABI, finite compact resource
admission and compatibility tests open. Managed execution retains its existing
admission and recovery contract. [Native compiler architecture](native-compiler-architecture.md)
