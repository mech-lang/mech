# U16 arithmetic, host allocation and source experiments

This packet preserves 5,063 completed timing/resource observations from six prospective cohorts, plus 3,040 qualification outputs. The fixed benchmark goal is met against the fastest verified unchanged repository example at upstream commit `fc2982b0c929b1b701209d401a1dc30c773ef4ab`: `rust_con`, retained as `rust-preinitialized`.

| Implementation | Primary processing interval | × original rust_con | Independent interval | × original rust_con |
|---|---:|---:|---:|---:|
| Original rust_con | 1.870584 ms | 1.0000× | 1.866250 ms | 1.0000× |
| Managed IR355, guarded U16 | 1.433000 ms | 0.7661× | 1.400292 ms | 0.7503× |
| Compact IR353, guarded U16 | 1.224125 ms | 0.6544× | 1.174375 ms | 0.6293× |

The original Rust example initializes its eight-thread Rayon pool before its processing timer. Mech creates, runs and joins four native workers inside its continuous post-JSON interval, including input admission and result materialization. Those original timer boundaries are preserved. The formal provenance archive identifies four unchanged repository-source/build variants and twenty local custom Rust source experiments. The latter are additional data; they are not the completion gate. Historical raw labels and eligibility rules remain unchanged, including the misleading historical `original_20_serial_Rust` reference name.

Managed IR355 U16 improves on managed IR334 in both cohorts: paired median −1.3908% (95% bootstrap interval [−2.8526%, −0.1296%]) and −1.8318% ([−3.3268%, −1.1355%]). Managed IR353's primary improvement did not confirm. Compact IR353 U16 improves on its own U32-source control, while its comparison with prior compact IR349 crosses zero in the independent cohort. No confirmed compact IR355 gain or unconditional compact checkpoint promotion is claimed. Both mask-reuse source designs and their combinations regressed and remain unselected.

The post-JSON gains have costs. Managed native code grows from 1,021,740 to 1,117,996 bytes, root stack metadata from 1,568 to 1,696 bytes, and executable size from 17,095,936 to 18,164,960 bytes. Primary peak RSS rises from 27,017,216 to 30,015,488 bytes. Independent whole-process medians are 40.379292 ms for managed IR334, 48.851250 ms for managed IR355 U16, 19.878875 ms for compact IR353 U16, and 17.980292 ms for original rust_con. Whole-process time is retained separately and is not the acceptance gate. Compact IR353's executable is 930,512 bytes and its native Object is 200,352 bytes; compact and managed stack metadata are not treated as equivalent.

The six cohorts contain 1,457 + 336 + 611 selected-increment/lazy-map observations and 1,612 + 371 + 676 U16/host/mask observations. All raw samples, predeclared orders, exact outputs, worker peaks, work variation, resource counters and failed diagnostic/wrapper attempts are retained. No samples were removed or selected from the resource runs. Every ordinary managed observation is checked against its own source's work bill: 22 source labels across 1,377 observations, four tasks, one batch and eight numeric regions; scheduling peaks of one through four are retained.

Qualification accounting is 95 full-plus-31-edge adapter suites: 2,912 later authoritative outputs plus 128 genuine additional initial IR350 outputs from four separately executed suites. Thirty additional focused scoring outputs and 120 guard/cast helper calls are separate. Final IR355 coherent V3 tests have 79 distinct focused passes and a broad result of 1,108 passes, the same sixteen inherited failure names, eleven ignored tests and thirty-five filtered tests. Initial historical-backend IR351 tests, unsafe image-test assumption failures, preflight failures and corrected test-only snapshots remain labeled in their original receipts.

Nine source snapshots are reconstructed against the single published IR349 Git base `7a6686c6790b0d8e0a282a5619bb884f8ffe08d0`. `report.json.source_closure_roles` distinguishes production, diagnostic CLIF, test-only and superseded snapshots: IR355 compiler-v2 is a superseded test-assumption snapshot with no native qualifications; final compiler-v3 supplies the IR355 qualification/test authority. The full 246-file V3 vendor closure and 164 literal includes are checked. Compilers use Cranelift. Executables and libraries are omitted with their build/hash receipts; native Objects and necessary source/artifact bytes remain archived.

`records.json` maps logical names to content-addressed gzip members in two indexed ZIP_STORED shards, each below 45 MB. Python's standard library reads them directly. Original absolute paths in receipts are historical provenance, not verifier dependencies. `report.json` includes complete timing/resource tables, fixed-reference comparisons, selection limits and material tradeoffs. The frozen independent QA tree is archived under `u16-host-source-independent-audit-v1/`.

From a checkout containing the published IR349 base, run:

```sh
python3 benchmarks/related-post-gen/integration/u16-host-source-20261112/verify.py
python3 benchmarks/related-post-gen/integration/u16-host-source-20261112/verify-fresh-git.py
```

Both commands are offline and execute no native program or compiler. The second verifies using isolated real Git objects from the published base. `verification.json`, `fresh-git-verification.json` and `FREEZE.json` record the final local verification and payload pins. `assemble.py` documents archival construction from the retained scratch inputs; running it is not required for verification.
