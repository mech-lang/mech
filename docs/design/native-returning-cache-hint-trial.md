# Prefetching the next private table write

The four-byte bucket hint reduced full-call time to **0.992619×** the paired control over 81 cases. It moved three random inputs around 64 KiB below Roc. The candidate beat at least one peer on **75/81 (92.6%)**, up from **72/81 (88.9%)**. The paired control is the normally linked executable from [reduction forwarding](native-zero-trip-parser-qualification.md). This experiment re-emits one routine at the same linked address.

| Result | Paired control | Four-byte hint |
| --- | ---: | ---: |
| Faster than Roc | 55/81 | 58/81 |
| Faster than libdeflate | 28/81 | 27/81 |
| Faster than both | 11/81 | 10/81 |
| Faster than at least one | 72/81 | 75/81 |

| Input | Control / Roc | Hint / Roc | Hint / libdeflate | Hint / control |
| --- | ---: | ---: | ---: | ---: |
| Random 65,535 bytes | 1.015161× | **0.991931×** | 1.688452× | 0.977117× |
| Random 65,536 bytes | 1.016032× | **0.988731×** | 1.677902× | 0.973130× |
| Random 65,537 bytes | 1.009717× | **0.989788×** | 1.674206× | 0.980264× |
| Distribution change, 16 KiB | 1.032158× | 1.015022× | 1.761947× | 0.983398× |
| Random 128 KiB | 1.077402× | 1.048261× | 1.629751× | 0.972953× |
| Random 1 MiB | 1.107293× | 1.074295× | 1.518749× | 0.970199× |
| Natural 16 KiB | 1.055487× | 1.047558× | 1.053377× | 0.992488× |
| Eight literals | 0.173789× | **0.172535×** | 2.192125× | 0.992786× |
| Repeated 32,769 bytes | 0.512411× | **0.516725×** | 1.005793× | 1.008418× |

The random crossings repeated at 0.988375×, 0.989355× and 0.985349× Roc, respectively. An additional repeat measured 0.990328×, 0.988829× and 0.980995×. The repeated 32,769-byte prefix went from 0.997397× to 1.005793× libdeflate in the full cohort and recovered to 0.993915× libdeflate in the additional repeat. The largest full paired increase was 1.32% on empty input. Both 1.05 gates passed in all measured cohorts, including the ten-case adverse repeat.

## Address and machine controls

The two peer binaries issue write-prefetch instructions for their hashed table addresses. Roc emits the search pair at `0x10005c518` and `0x10005c524`. libdeflate emits it at `0x10005e168` and `0x10005e170`. The corresponding skip paths also contain the pair. The prior [returning-edge trial](native-latch-prefetch-pilot.md) isolated a gain on 1 MiB random and found regressions in the full executable. The current control includes shorter comparison branches and a different machine register assignment.

The preflight reconstructs the exact current 9,956-byte matcher. The four-byte selector is an unsigned 32-bit word shifted right by 16. Its returning-edge argument reaches an existing U16 store at byte offset `131072 + 2 * selector` in a caller-admitted 262,144-element view. The independent SSA trace replays all nine argument links to the original store. The three-byte alternative uses `262144 + 4 * selector`. Both ranges fit the admitted view. The trial inserts the address calculation and one nontrapping `prfm pstl1keep` before the existing transfer.

The four-byte variant is 9,960 bytes with 410 static stack references versus 412 in the control. The three-byte variant is 9,984 bytes with 416 references. Both retain a 576-byte frame including saves. A two-hint variant requires 10,044 bytes and 429 stack references, so it could not be tested in the 9,984-byte slot. An earlier attempt using the older backend also failed exact-body preflight. No elapsed result comes from those refused builds.

All 1,276 named addresses are identical across the fixed-layout links. For each hint the corresponding no-op worker retains the same register allocation, text layout and address instructions. Its executable differs at the hint instruction, UUID and signature. The four-byte hint takes 0.993996× the same-layout no-op in the 18-case screen and 0.993978× in the eight-case repeat. Random 1 MiB takes 0.980366× and 0.983467× in those separate cohorts. The repeated 32,769-byte case is 1.007857× the no-op in the repeat. The three-byte candidate takes 0.997756× the actual control in its screen, and 0.992508× its own same-layout no-op. The compiler candidate will use the four-byte experiment's denser address form.

## Separate counters and samples

| Input | Retired instructions / control | Cycles / control |
| --- | ---: | ---: |
| Random 64 KiB | 1.002305× | **0.969208×** |
| Random 1 MiB | 1.003032× | **0.969673×** |
| Held-out random 65,537 bytes | 1.002391× | **0.971270×** |
| Periodic 1 MiB | 1.000978× | 1.006201× |
| Repeated 1 MiB | 0.999637× | 0.999678× |
| Eight literals | 0.999725× | 1.046022× |

The matcher accounts for 2,764 of 4,402 mapped running samples at 64 KiB random and 3,742 of 5,399 at 1 MiB random. The leading PCs include the short-bucket result consumer at `0x100062ea0` and the long-bucket result consumer at `0x100062fa4`. These samples include validation. The hardware-counter runs are separate from primary elapsed and their eight-literal cycle increase remains visible.

## Reproduction

The paired control worker is `0450aff179f6043872527efaadd42ce57fe7988d7f7caacd5e2d9026edf4dce9` and the candidate is `1bfd044d1218b68549bfbfd4e3a6b25d150762108dfdbb0ca3d680b9fed903f8`. Their descriptor, pinned level-12 program, peer workers, allocator and full-call timing boundary are shared. Fresh results matched the pinned output bytes and capacity and independently inflated. Alias preservation, tracked payload allocation and cleanup passed.

Run `python3 -B verify-round.py` in the extracted archive. It replays 304,224 timed calls, 648 linked checks, eight profiles and eight hardware-counter cases. The archive also records a corrected replay filename for the short hint. SHA-256 `0fd5b6c84300be53787938d6e6eca18c64b07d17cd11e07ea98da678ab7d0262`. [Verified receipts and disassembly](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-72c4c55557-returning-cache-hint.tar.gz).
