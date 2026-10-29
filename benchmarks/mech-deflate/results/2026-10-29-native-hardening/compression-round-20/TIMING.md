# Nested private-frame composition: committed timing

Prior published head96721e45d454f507595c54e445d18266eff05ced; baseline8c6d6e097; production **e721febbcd557040ce8866e2ebbf61f09a9e4b5b**; seven passing focused regressions **d9db1f234**. IR122/work93/ABI35/bytecode14. Full seven-test output is retained by the closure packet. Production release builds successfully in444.741s with the unchanged selected operation profile. No production source changed during the build or measurement.

**Roc=1×: 106.602389078× → 99.868245734×.** Complete eight-byte medians 31.234500ms → 29.261396ms; reference 0.293000ms (20000 observations). 6.317066% less elapsed; 1.067430× speedup. This is a modest gain, not a large-input result or evidence of parity. The prior round's106.514339× used another fresh reference and is not one continuous paired series.

Fixed four alternating-order process pairs, three primary observations per control/process, twelve per control. All40 process observations are retained:24 primary,8 warm-ups,8 excluded profile-tagged. A separate sampled process and all its observations are excluded. No compiler/test/other benchmark overlapped primary. Same mimalloc2.1.7 and evaluate_into-only harness. Setup, JIT compilation, seed/clear/rebind, validation and drop stay outside the timing boundary.

|Pair|Order|Before ms|After ms|After/before|
|---|---|---:|---:|---:|
|1|before→after|32.482125|29.059584|0.894633094|
|2|after→before|31.203083|29.533500|0.946493012|
|3|before→after|31.189208|29.236292|0.937384880|
|4|after→before|31.256333|29.112917|0.931424585|

All observations produce the exact10-byte raw reference output and independently inflate; output capacity13 and final cleanup0 bytes/nodes hold. Output backing changes and those copies remain timed. Work 307973385655→306669833472; the selected private path removes canonical record construction and transport while existing prices/allowances remain. No equal-debit claim is made.

Pinned source671077457ff7ef3a3d44caf715a3fc595addf379; reference compiler d99c7b838fb05bb1047eb0c18d4d06d0f86f345d; unchanged bytecode a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7 and source331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9. Algorithm, tables, search limits, buffer sizes, source allocation sites and timing boundary are unchanged. Mech performs compression itself. No larger input was executed.

The fresh raw executable profile, command, all process rows, binary/dependency hashes and artifact pins are committed here before subsequent qualification or optimization. No confidence interval or removable-time percentage is claimed. The first preflight failed before any execution because the reference subdirectory was absent; its setup receipt is retained and the complete pinned package hash check then passed unchanged. No current timing failure or resource denial remains.
