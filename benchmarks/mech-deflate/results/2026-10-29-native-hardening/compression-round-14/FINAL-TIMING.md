# Complete prepared-frame admission result

Measured compiler: `5a46c85935279a61b934222331b643b81d254d6d`; base: `d840ea83797e8a47ebf16dbaf86816b0bc86710a`. This adds direct shape relations only after successful prepared flat-field admission, retaining the original constructor on a miss. The preceding two footprint implementations and the initial combined experiment retain their own immutable receipts; this campaign is the final implementation's paired result.

Fresh Roc = 1× reference: 0.000288000 seconds, 20000 samples. Three primary observations each; unchanged eight-byte input, selected-operation optimized profile, pinned mimalloc2.1.7 object and evaluate_into timer. Warm-up and the separate 66-repetition sampling process are excluded.

| Compiler | Median seconds | Multiple of Roc |
| --- | ---: | ---: |
| Round13 baseline | 0.073249792 | 254.339556× |
| Prepared footprints, original shapes | 0.063995666 | 222.207174× |
| Final prepared admission and direct shapes | 0.061212458 | 212.543257× |

Final/baseline: 0.835667329, **16.433% observed reduction**. Final/prepared-footprint-only: 0.956509430, **4.349% further reduction**. The scope is this complete eight-byte program; no confidence interval or larger-input throughput is claimed. This remains far from the 1× goal.

Exact ten-byte raw output, independent decompression, capacity13 and final zero bytes/nodes pass on every observation. All controls spend exactly 527732164684 logical work. Physical output backing changes in all controls, retaining the same timed boundary transport. Source/bytecode/allocator hashes, algorithm, allocation sites, table sizes, search/EOF rules, diagnostic policy and timing boundaries are unchanged. Typed IR118/work93, ABI35 and bytecode14 are unchanged.

Before timing: 101 actual Engine budget/worker/projection/shape tests and 12 frame tests pass. All 144 admission cases pass, including exact agreement with the 120 baseline observations and 24 competing-error cases. Tests cover prepared scalar bits/debits, four packed unsigned widths, real capacities, changing rectangular/zero extents, aliases, repeated/constant axes, u64::MAX×0, old/decoded artifacts and constructor parity. All twelve static contracts pass. No current arguments, extents or owners are cached in the immutable recipe.

The new recipe reuses the existing borrowed-schema proof and budget/footprint owner. Native scalar footprints are prepared with the maintained scalar meter once. Runtime words validate bits and return fixed logical footprints; current packed metadata remains measured by Core. The same-account full preview combines only successful debits; unsupported, denied or shared-worker cases keep the ordered visitor. Direct shapes are used only for current fields validated by that success path.

Broader component/scalar/Object qualification and final publication are pending at this timing checkpoint. Historical round10 decoded-U64 scalar ratio1.0526036 against1.05 remains unresolved. This packet is committed before starting any further optimization.
