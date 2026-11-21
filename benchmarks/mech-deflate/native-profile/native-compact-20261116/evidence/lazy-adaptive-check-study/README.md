# Conditional adaptive-block predicates

The source experiment preserves every block-decision threshold and compressed byte. It evaluates later pure predicates only when earlier ones permit the adaptive check. The original 2,719-line source and selected **63,804 B / 81 Roc / 63 C** default remain unchanged. The qualified HF-U32 source is its control.

Both 21-case, 24-block orders retain every observed Roc and C point win (**21 Roc / 9 C**). This establishes no new C win, no full 81-case qualification and no confidence claim. Retained for combinations: natural 1 KiB improves about 1.2%, random 1 MiB is essentially unchanged, and distribution change slows by 2–3%.

| Input | Candidate/control forward | Reverse | Candidate/C forward | Reverse |
| --- | ---: | ---: | ---: | ---: |
| 320-overlap | 1.003158× | 1.001674× | 0.968568× | 0.972863× |
| prefix-natural-1024 | 0.988221× | 0.988229× | 1.224614× | 1.230068× |
| prefix-natural-4096 | 0.995851× | 0.993774× | 0.979363× | 0.979482× |
| prefix-natural-8192 | 1.017928× | 1.017076× | 0.951965× | 0.956272× |
| prefix-random-4096 | 1.049744× | 0.992080× | 1.557419× | 1.317918× |
| prefix-random-16384 | 1.001534× | 1.001302× | 1.344763× | 1.311716× |
| prefix-random-1048576 | 1.000922× | 1.000503× | 1.259420× | 1.255873× |
| 16384-distribution-change | 1.031340× | 1.021736× | 1.233707× | 1.234604× |

Random 4 KiB has visible order/control variation; its two observations do not establish a stable gain. Small slowdowns remain acceptable when the named wins survive. This experiment is retained because its mechanism may combine with later matcher improvements, not because instruction counts predict a speedup.

Native text is **72,308 B**, 112 B larger than the qualified HF-U32 control. Measured peak stack remains **4,752 B**, below the 5,280 B cap. All 162 successes preserve exact output, inflation, input/output aliases and cleanup. Six source-mapped allocation errors and twelve failure/recovery pairs pass. The only resource difference from the original source is the qualified HF source’s existing 288-entry U32 counter allocation. Public normal/checked rebuilds reproduce object, full metadata and bytecode exactly.

| Source through final executable link, Apple M1 | Median | Paired × Roc | Paired × C |
| --- | ---: | ---: | ---: |
| Selected original source | 2.220953 s | 0.743370× | 4.590725× |
| Qualified HF-U32 | 2.348570 s | 0.785822× | 4.853143× |
| Conditional-check source | 2.350655 s | 0.785863× | 4.852382× |
| Roc | 2.988208 s | — | — |
| libdeflate | 0.483783 s | — | — |

Ten balanced serial fresh-process trials per actor include final executable linking, with compilers and fixed support equally prebuilt. Strip and compiler bootstrap are separate. Every generated Mech artifact hash is checked in every trial. All five actual stripped executables pass all 81 input/inflation checks (405 executions). This source change gives no build-time improvement.

The actual final matcher executes 199,755,647 instructions on random 1 MiB, versus 204,521,468 for the qualified control; stack loads fall from 21,120,522 to 19,010,173. Runtime nevertheless stays flat on that input. These are kernel counts, not whole-call elapsed-time budgets.

Reproduce native emission on Apple ARM64 with the immutable compiler in the receipt, or a compiler whose default behavior passes the identity checks:

```sh
python3 reproduce.py /path/to/compile_profile /tmp/lazy-normal
python3 reproduce.py /path/to/compile_profile /tmp/lazy-checked --checked
python3 verify.py
```

The complete generated source, exact replacement generator, source-specific admitted homes, three layout profiles, source-origin mapping, diagnostic counts, all individual timing calls, complete-build commands and every trial are archived. Profiles belong to this source; no profile is reused across an unmatched machine graph. Raw clocks used the shipping worker with diagnostics disabled.

Next work uses measured matcher frequencies to test register allocation across mixed inputs. Canonical constants already live in readonly data and rebasing already has a SIMD kernel, so those are not assumed missing optimizations. Matching Mech/C semantic work remains open. **64 C wins is next; 81 remains the goal.**
