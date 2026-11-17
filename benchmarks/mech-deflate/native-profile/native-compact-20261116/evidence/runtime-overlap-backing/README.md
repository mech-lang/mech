# Overlap backing on the fixed encoder

Two three-second profiles per worker put **58.5%** of the
bottom-tested worker's native encoder samples in the checked root and its
runtime calls on `320-overlap`. The allocation profile records 59 payload
requests. One 328,960 B word buffer uses anonymous mapping. The older
62-win worker uses the allocator for that request. [Stack samples](sampling/result.json) ·
[Earlier backing trace](../halfword-zero-page-screen/profile-observations.json.gz)

A fixed-encoder comparison raises the Darwin mapping threshold from 16
to 32 pages for four-byte-aligned allocations. The encoder object and
metadata are exact in both slots. The change passes all 16 runtime tests,
162 linked checks, and six checked failures. Both workers meet the
5,280 B measured stack bound. [Patch](word-page-threshold.patch) ·
[Linked checks](checks/result.json)

| Case | Order | Control vs C | 32-page threshold vs C |
| --- | --- | ---: | ---: |
| `320-overlap` | Forward | 0.99181× | 0.99695× |
| `320-overlap` | Reverse | 0.99276× | 0.99821× |
| `prefix-natural-8192` | Forward | 0.98940× | 0.99113× |
| `prefix-natural-8192` | Reverse | 1.00682× | 1.00472× |
| `prefix-periodic-1024` | Forward | 0.97800× | 0.97859× |
| `prefix-periodic-1024` | Reverse | 0.98117× | 0.98087× |
| `prefix-natural-4096` | Forward | 1.08437× | 1.08686× |
| `prefix-natural-4096` | Reverse | 1.10132× | 1.09985× |

Both workers keep their 320-byte wins here. The threshold change takes
about 0.5% longer on that case in each paired cohort and the natural 8 KiB
case finishes above parity in reverse. The study records a focused result.
[Focused replay](focus/screen.json) · [Raw observations](focus/)
