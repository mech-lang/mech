# Shared heap call setup screen

The Apple AArch64 trial shares checked call setup for the earlier ordered releases plus allocation, writable backing and a byte append. It redirects 260 checked calls through eight private entries. The encoder emits **64,668 B**, **368 B** below the 65,036 B selected opt-in worker. The trial matched all 162 linked successes and six checked failures at **4,784 B** stack. [ABI](abi-proof.json) · [Linked checks](linked-checks.json)

| Order | Roc named / distinct | libdeflate named / distinct |
| --- | ---: | ---: |
| Forward | 80/81 · 73/74 | 62/81 · 56/74 |
| Reverse | 81/81 · 74/74 | 62/81 · 56/74 |

The forward sweep lost `prefix-natural-32769` against Roc under the point-estimate gate. V3 rejected the build. The 65,036 B opt-in result keeps its published peer qualification. [Full screen](full-screen.json) · [Per-case intervals](full-replay.json.gz)

The same cohort measured **2.183 s** through final link, **0.733× Roc** and **4.546× libdeflate**. The median paired build took **2.7 ms** longer than the 65,036 B control. [Matched timing](deployment.json) · [Raw trials](build-comparison.json.gz)

The byte-identical control trace measured about **411 ms** of checked-root cleanup, including other proof passes, and **93 ms** from the machine-code marker to completion. A separate diagnostic broke out **76 ms** of guard derivation and **44 ms** of retirement proofs. [Control stages](control-profile.json) · [Ownership detail](ownership-profile.json)

The archived compiler patch replays the experimental object, canonical bytecode and metadata. [Source pin](source.json) · [Recipe](candidate.json)
