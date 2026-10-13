# Exact ARM call rendering and preserved source-work evidence

The frozen work-v2 run passed 17 actual fixture methods, 14 Pi operand/quotient probes and 24 complete-map probes. It then stopped at the compact original scored tree ARM gate. The saved creator calls `calloc` at 0x100002314 before any control, but otool prints `bl 0x100010bf4 ; symbol stub for: _calloc`. The former direct-symbol regex missed this real allocation. Both child calls are direct `_clbg.binarytrees.make`; the new parser also accepts a numeric self-call only when its target is the scoped creator's actual first instruction address 0x100002300. It accepts no arbitrary target/comment as proof. Exact labels, one allocator, both child calls, allocation before all control/return and the postallocation leaf branch remain required.

The scoped saved-ARM inspection pins the original failed report, native otool receipt, full disassembly and extracted creator. `work_reuse.py` requires that exact failed report digest and 17-method receipt, reconstructs all four observer sources/binaries/build commands/native images/dependency pins, binds every original fingerprint/command/environment/raw/supervisor/output/input/trace, and reconciles all 38 traces against both unchanged old and current observer readers. No successful native source-work probe is repeated. The old work-v2 report lacked terminal pins because it stopped before that producer's terminal evidence collection. This packet explicitly marks the preservation hashes as newly captured saved evidence; it does not fabricate a historical pin.

Work-v3 retains 38 original proof/attempt identities in a separate pinned packet. Its fresh attempts are only 8 tree allocation/retention cases and 2 generic ABI cases. The original report, 38 records, observer programs, codegen, raw disassembly, failed 17 receipt and original sources remain immutable. All 20 original scored C3 binaries and 154 bounded cases remain unchanged. The fresh 19 fixture methods are the prior 17 plus 2 meaningful preservation/fingerprint/schema methods; actual ARM stub/self-target negative subcases extend the existing lowering method. No fixtures/imports/hash/native work were executed while authoring.

After root and peer source review, proposed guarded command from the cohort directory:

```
python3 -B revisions/work-full-v3/qualify_work.py --qualified-revision private-v3 --revision work-v3
```

It acquires the same common mutex before every import of preserved readers, fixture/hash/build/source-work/native action, and every owned child/sampler inherits that descriptor through cleanup. The 19 discovered/started fixture IDs and zero skips/failures/errors must pass. Every terminal available output byte is pinned on success or failure. Root must manually review the actual original scored compact and optimized LLVM/ARM allocation plus complete tree evidence before the later full sealer:

```
python3 -B revisions/work-full-v3/seal_full_studies.py --work-revision work-v3 --study-revision full-v3
```

The full sealer requires exact 48 proof keys, 38 unchanged retained proofs and 10 distinct fresh attempts, 20 source reviews, the actual 19 fixture receipt, and complete private-v3 bounded qualification. Full index/digest root review still precedes 90 timing processes. Canonical LOC remains separate and null until its independent qualification; no chart admission or data/source/timing changes are made by this repair.
