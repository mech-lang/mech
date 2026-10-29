# Separately attributed footprint/accounting timing

Measured compiler: `bf850c333e29c14812b78ae232950282edba1602`. Base: `aa93a9e3fbba56c54eb8c48648570d54c8da7500`. The original shape recipe/constructor is restored; the footprint/accounting optimization is otherwise the same as the first combined implementation.

Fresh Roc = 1× reference: 0.000288000 seconds, 20000 samples. All controls run in the same protected campaign, same eight-byte input, pinned mimalloc2.1.7, unchanged source/bytecode and evaluate_into boundary. Three primary observations each; warm-up/diagnostic rows excluded.

| Compiler | Median seconds | Multiple of Roc |
| --- | ---: | ---: |
| Round13 baseline | 0.072489792 | 251.700667× |
| Footprint/accounting only | 0.063903667 | 221.887733× |
| Same footprint/accounting + direct shapes | 0.061392583 | 213.168691× |

Footprint-only after/before: 0.881554012, **11.845% less elapsed**. Direct shapes combined/footprint-only: 0.960705166, **3.929% further reduction**. Three observations do not establish a confidence interval; this is not larger-input throughput.

Every observation verifies exact ten-byte output, independent decompression, capacity13 and complete cleanup. Physical output backing changes for every control and remains a timed cost. All controls spend exactly 527732164684 logical work. No algorithm, component, table, allocation site, search/EOF bound, work price, resource policy, timer boundary, IR118, ABI35 or bytecode14 change.

Twelve focused tests pass and all 120 recorded work/error/retry observations match baseline. All twelve static contracts pass. Broader component/scalar qualification is pending, as is the requested fixed-scalar/schema-proof consolidation; this is the measured footprint-only substep, not a claim that the whole new brief is complete.

The first campaign attempt stopped in preflight because the old temporary reference checkout lost fourteen pinned source files and Git HEAD/config. No measurement was taken. The exact blobs were recovered from the surviving pinned object database into a new private directory and all fourteen existing SHA256 pins were verified. The original reference executable and every control executable retained their expected hashes. No source or reference binary was replaced with another revision. The preflight failure and recovery receipt are retained. Exact deletion cause is not established.

The separate 63-repetition diagnostic profile is excluded from primary timings. Original samples, executable/library/source hashes and build commands are retained. The historical round-10 scalar ratio 1.0526036 versus 1.05 remains unresolved.
