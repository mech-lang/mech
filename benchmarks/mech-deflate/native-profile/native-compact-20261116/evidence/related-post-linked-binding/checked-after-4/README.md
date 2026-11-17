# Checked Related Posts after-4 prototype

The patch reuses the managed Cranelift generator and its pre-link pins for
the source, bytecode, Object, catalog and adapter. Its adapter prepares the
catalog before the restore timer and retains the input, native evaluation
and publication boundaries.
[Patch, including reproduction commands](source.patch.gz)

The unchanged architecture checker clears the prototype's after-4 source
and reaches after-8. The exact archived after-4 source pin remains valid.
Compile, mismatched-input rejection, and output qualification are pending.
The proposal preserves its own limited scope and identifies the remaining
historical and live paths separately.
[Static receipts](static-checks.json)
· [Remaining boundary findings](remaining-static-findings.json)
