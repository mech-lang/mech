# Inline the maintained library debit across the crate boundary

Status: **rejected and removed after a consistent 2.46% slowdown**.
See [the closure packet](CLOSURE.md); the earlier work-wrapper gain is retained.

Starting published/compiler checkpoint:
`a4b9ac5f4312cf822102dd394f9ea52c3add08b6`.
Keep the preceding work-wrapper hint and all earlier retained improvements.

The last fresh complete eight-byte profile has 68 exclusive samples in
`NativeLibraryWorkBudget::charge_compute_with_limit`, 54 in TLS lookup and 39
in its shared debit. The newly inlined GPU wrapper still calls this library
method across the crate boundary. The baseline machine code and the prior
reference disassembly are preserved in the preceding round's packet.

Add only an ordinary `#[inline]` hint to that existing public method. This
exposes its body to the Rust optimizer in native callers, allowing folding of
the common zero-comparison argument and caller-specific error handling.
Keep the caller ceiling/overflow check before foreign-scope rejection and
preserve root/call/worker caps, reserved fuel, atomicity and spent work. Do not
introduce a faster accounting API, independent account or new owner.

This is a measured candidate, with no gain claimed yet. Build the same selected
release configuration, preserving unrelated libraries and operation features.
Run existing Engine accounting tests, GPU runtime/accounting tests and the
protected scalar/frame controls. Freeze a fresh complete-program worker and
compare it with the retained previous binary over four balanced worker pairs.
Keep all raw observations and commit each timing cohort immediately. Require
identical work, raw output, independent inflation, aliases, capacity and cleanup.
Inspect the changed machine code, compare emitted program artifacts and collect
a separate fresh profile. Keep a demonstrated modest gain and retain any
counterevidence. No larger-input timing or semantic compiler expansion is part
of this experiment.
