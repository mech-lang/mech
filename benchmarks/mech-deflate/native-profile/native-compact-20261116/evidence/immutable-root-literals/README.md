# Scalar entry literals in compact SSA

The census identifies **1,389** declared scalar literal lanes in the
largest checked root with no source definitions and zero writes during traced
emission. They account for **1,831** provisional parameter
reads across **2,026** reads. Several reads may refer to the same
parameter. [Census program](lane-census.py) · [Counts](lane-census.json)

The patch uses the value defined in the dominating entry block for admitted
literals. Input aliases, composite values, owner lanes and source-defined values
use the usual path. A write to an admitted lane discards the attempted emission
and triggers the usual compiler path. Both focused SSA tests pass.
[Compiler patch](compiler.patch.gz) · [Tests](tests.log.gz)

All three compiler slots reproduce the exact bytecode, encoder and metadata.
The enabled and maintained slots produce the same linked worker, with 162
matching checked observations, six failure probes and a peak stack of **5,280 B**.
The exact deployment executable is **292,568 B** after stripping. The encoder
text is **71,192 B**. The existing complete
runtime qualification carries **81/81 Roc and 61/81 libdeflate** in both orders.
[Build identity](compiler-build.json) · [Linked checks](linked/result.json) ·
[Full runtime qualification](../frame-copy-v2/README.md)

| Apple M1, 10 balanced blocks | Source through final link | Native phase |
| --- | ---: | ---: |
| Maintained Mech | 2.53655 s | 1.38673 s |
| Probe compiler, literal path off | 2.53085 s | 1.38524 s |
| Probe compiler, literal path on | 2.49536 s | 1.34888 s |
| Roc | 2.98725 s | — |
| libdeflate | 0.48262 s | — |

The paired through-link saving is **40.63 ms** (90% interval 33.08 to 47.17 ms) against the maintained
compiler and **37.57 ms** (90% interval 28.53 to 38.93 ms) with the same compiler and the option off.
The latter comparison saves 36.37 ms (36.03 to 36.70 ms) in the
native phase and 38.39 ms (28.06 to 38.98 ms) of source compiler CPU.
The comparable deployment build is **0.835× Roc** and
**5.173× libdeflate** with fixed support prebuilt.
The frame-copy rebuild recipe uses the measured literal path.
[Schedule and pins](matched-plan.json) · [Per-trial timings](matched-observations.json.gz)

## Related Posts integration

GitHub cleared formatting and the warning audit before reaching the
`after-4` linked-program restore. Preparing the checked producer on this
revision exposes its Cargo example and advances the local boundary audit to
the separate `after-8` restore. Compiled and runtime validation of the
overlay is pending.
[GitHub job](https://github.com/mech-lang/mech/actions/runs/44200159860/job/134452599256) · [Static overlay](related-posts/static-overlay.json)
