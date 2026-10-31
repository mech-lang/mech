# Current-ledger lookup audit (read only)

R56's 42 exclusive `_tlv_get_addr` sample weights are not all an Engine budget
lookup: six immediately belong to mimalloc. The retained `tls-caller-samples.json`
reconciles all 42, with 23 under private frame actions. Largest direct caller is
NativeLibraryWorkBudget::charge_compute_with_limit (21); ResidentBudgetMeter's
constructor, compute debit and observe_control_work each contribute three.
These are sample weights, not invocation counts or retired instructions.

## Existing mechanisms

- `budget.rs:609` with_call_limits already enters CONTROL_WORK_CONTEXT once and
  keeps references to its active/scope/limit cells through the synchronous call.
  RAII guards restore each cell in reverse order on normal exit and unwind.
- `budget.rs:463` charge_in_context already takes `&ControlWorkContext` explicitly
  and performs current active-account identity checking, live call-limit reads,
  and the ordinary atomic two-dimension worker debit.
- `budget.rs:963,986` preview_control_work and observe_control_work each reacquire
  that same TLS address. The observer computes the current scope's delta and
  commits its high-water marks only after a successful account/worker debit.
- NativeProjectionIndexContext holds `&NativeLibraryWorkBudget`, call limits,
  prior live demand and the call-start counters. Each charge checks those current
  selected-call facts and then invokes work.charge, reacquiring TLS.
- NativeWorkDebitBatch is NOT an ordinary selected-call scoped capability. Its
  documented callback is allocation-free and callback-free; ordinary mode still
  delegates to work.charge. Its worker-lock reuse has already landed and should
  not be reopened for this experiment.

There is no existing externally usable lexical handle that safely exports the
already-borrowed ControlWorkContext. The building blocks are already private in
one module, so a narrow internal extension does not need another budget authority.

## Smallest first experiment: collapse duplicate lookups within each operation

ResidentBudgetMeter currently performs:
- for_native_projection: separate TLS lookups for active account and scope (2).
- preview_compute_and_nodes native branch: serial-worker check, scope identity
  check, then preview_control_work, each retrieving TLS (3).
- charge native branch: scope identity check then observe_control_work (2).

Factor private `check_*_in_context`, `preview_control_work_in_context`, and
`observe_control_work_in_context` helpers, retaining public wrappers for existing
callers. Each meter operation can use one CONTROL_WORK_CONTEXT.with closure and
pass the borrowed context through its exact existing sequence. All individual
charges, checks, comparisons and current limits remain; nothing is batched or
cached across calls. This needs no new public type or lifetime parameter.

Preserve short-circuit order, and release the immutable scope RefCell borrow
before the observer's mutable borrow. In particular, charge must still observe
work BEFORE checking the per-meter memory policy: reversing this order would
change the spent prefix of rejected calls. Shared worker preview must retain its
existing decline/ordered fallback rather than borrow worker credit.

Expected allocation delta: zero. Expected generated Object change: none. Accept
only with actual host instruction and matched elapsed measurement. Source-level
TLS-count reduction is not itself proof of machine-code reduction; inspect the
optimized function bodies and count surviving tlv_get_addr calls.

## Larger follow-up: lexical current-context handle inside one selected call

Introduce only if the small experiment establishes remaining benefit. A private,
non-Clone, non-Send `ScopedCallWork<'scope>` can borrow both the current work
account and &ControlWorkContext inside with_call_limits' existing closure.
An HRTB callback binds its lifetime to that invocation; no result can retain the
handle. No address, balance, call cap or permission is written into descriptors,
prepared calls, caches or subsequent visits.

Its charge operation calls the existing charge_in_context using live cells on
EVERY debit. Do not cache the current limit value: nested same-account calls must
narrow limits immediately, restore them afterward, and never refund counters.
Do not hold a RefMut<ControlWorkAccount> across arbitrary leaf construction or
nested calls; that would break existing reentrancy. A reference to the Context's
independent cells is sufficient and does not borrow their mutable contents.

NativeProjectionIndexContext already has a call lifetime and can borrow this
handle. Existing generic work APIs remain ordinary wrappers. This extension
would remove repeated TLS resolution within selected calls, while current
memory/account/shape/owner checks still run. It cannot remove the large caller-
side NativeWorkMeter::charge_compute_with_limit family by itself; claiming all
42 TLS sample weights as savings would be incorrect.

## Required evidence

Existing native_projection_meter_tests covers wrong/escaped scope, nested tight
caps, current node/byte limits, preview no-spend/overflow, comparison-denial
retry, all schema-precharge denial boundaries and shared workers. Existing
native_library_work_tests and native_worker_budget_tests cover same-ledger
nesting, foreign rejection, two-dimensional spend and limit restoration.

For a lexical handle add a compile-fail/lifetime or API-level non-escape check;
exercise outer handle inside narrower same-account nested scope, error return,
unwind and subsequent fresh call, proving live nested limits and unchanged
observer high-water/spent prefixes. Keep scalar original/decoded performance
controls and the pinned complete 8-byte compressor qualification. Requests,
instructions, cycles and elapsed are separate outcomes; retain adverse cohorts.
