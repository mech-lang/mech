# Terminal output owner slot pilot

The candidate pops a terminal output slot after the existing ownership and geometry checks. Interior publication continues to recycle its slot through the free list.

| Input | Candidate / R355 | Candidate / merged hybrid | Candidate / matched control | Faster matched blocks |
| --- | ---: | ---: | ---: | ---: |
| empty | 1.030502× | 0.885408× | 0.887921× | 24/24 |
| one-stored | 1.003983× | 0.888236× | 0.903441× | 24/24 |
| seven-stored-prefix | 1.022369× | 0.891444× | 0.897653× | 24/24 |
| eight-literals | 1.009629× | 1.000258× | 0.999839× | 10/24 |
| prefix-natural-262144 | 1.001506× | 1.000060× | 0.996174× | 14/24 |
| Geometric mean | 1.013537× | 0.931496× | 0.935685× |  |

The three stored-input cases change by **10.37% faster** in geometric mean against the matched control.
**0/5** cases meet or beat exact R355 in this screen. Slower cases are empty, one-stored, seven-stored-prefix, eight-literals, prefix-natural-262144.

The linked control calls `RawVec<usize>::grow_one` at `0x100016748`. On a fresh free list, the growth floor is four eight-byte slots and the zero-capacity path calls `__rust_alloc` at `0x100185430`. The timer covers the corresponding `mi_free` at `0x100006034`. The three stored cases have one payload allocation per call and publish it from a fresh Heap, so they take this metadata allocation route. Payload counters exclude the free-list buffer.

In the linked candidate, the terminal branch at `0x100016728` through `0x100016740` updates the owners length and jumps to common output publication. The interior branch still calls `grow_one` at `0x10001678c` if needed. The 77-instruction timed wrappers match after normalizing relocated call addresses.

The four workers use the same R355 Object `803a2d556384a12f3732b17953152e54982b3f2f23be36e55cd04f5d909a4094` and pinned allocator. The two matched workers also share the exact worker source, external transfer call, and full timing boundary. Both runtime variants passed 13 tests, including alias rejection and retry, unchanged output backing and capacity, interior reuse, and cleanup.

The screen retained 153,696 complete calls in all 24 balanced orders per case. Every call checked exact bytes, independent inflation, aliases, output capacity, and the same six payload counters. Replay keeps the original zero timer observations. The 81-case acceptance and separate hardware counters remain pending.

[Raw observations, linked disassembly, unit receipts and offline replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/terminal-owner-slot-8ca8324e56f6.tar.gz).
