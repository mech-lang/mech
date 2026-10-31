# Prepared scalar footprints and shared schema proof

Measured compiler: `d6675140a14d3ac1570cb92b0a7264d286409fd8`; base: `2427adf7b52e9957ea2b50a24e3a52ed872aa731`. The existing schema-proof owner and exact maintained scalar measurement prepare fixed logical footprints and work once. Runtime scalar fields perform bit validity checks without generic scalar visitors. Packed buffers still use current Core metadata and retained capacity. Original shape reconstruction stays on both footprint controls.

Fresh Roc = 1× reference: 0.000286000 seconds, 20000 samples. Three primary observations per immutable control; same eight-byte input, selected-operation release profile, pinned mimalloc object and evaluate_into boundary. Warm-up and separate 63-repetition profile excluded.

| Compiler | Median seconds | Multiple of Roc |
| --- | ---: | ---: |
| Round13 baseline | 0.072490500 | 253.463287× |
| Initial footprint-only | 0.064234208 | 224.595133× |
| Consolidated prepared footprints | 0.063870875 | 223.324738× |

Consolidated after/baseline: 0.881093040, **11.891% observed reduction**. Consolidated/initial footprint implementation: 0.994343621, **0.566% observed reduction**. This small additional difference is treated as roughly unchanged, not a new large speedup. The architectural consolidation retains the main gain. No confidence interval or larger-input throughput is established.

Every observation has exact ten-byte raw output, independent decompression, capacity13, identical logical work527732164684 and zero final bytes/nodes. Physical output backing still changes on every repetition and remains timed. All pinned source/bytecode/allocator hashes agree; no compressor, allocation-site, search/EOF, policy, timing-boundary, price or ABI change.

Validation before timing: 100 actual Engine budget/worker/projection/shape tests pass, including three new differential footprint tests; all 12 frame tests pass. All 120 baseline admission observations agree exactly, with 24 new competing-error cases passing as well. Twenty scalar-word cases cover widths, extrema, negative zero and a NaN payload; twenty packed-owner cases cover four unsigned widths, zero/rectangular/changing shapes, aliases and real retained capacities. Expected unwind panic messages in the Engine log belong to passing cleanup tests, not failures. The Engine test root was built with warnings denied.

The next separate step is to restore the previously measured direct shape checks, retaining constructor fallback and adding constructor-parity coverage. Broader component/scalar/linked qualification remains pending. The historical round10 scalar ratio1.0526036 against1.05 remains unresolved. No optional optimization is started before committing this timing packet.
