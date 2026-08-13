# D4 production resident-routing contract

This directory freezes the first normal product route from Mech source or
bytecode-v1 into a runtime-owned resident program. It records the routing,
authority, host, native, browser, n-body, and failure boundaries introduced by
D4 without treating generated JSON as implementation authority.

Regenerate and check the projections with:

```bash
python3 scripts/generate-d4-contract.py --check
python3 scripts/check-d4-contract.py
```

The resident n-body architecture fixture is the exact D2 recurrence with a
packet-authoritative timer tick, an unchanged `0.01` step expressed as
`tick * 0.0 + 0.01`, and one accepted N×2 `scene/points` effect. Resident host
input follows an explicit sample-and-hold model: any packet containing a
relevant activated source may trigger a turn; values present in admitted
packets are authoritative, while activated observations absent from those
packets synchronously capture the latest value from their retained provider
binding. This is newest-complete-snapshot semantics, not strict ingress
causality or a requirement that every packet carry every observation. The
deterministic product test proves 4,096 source and bytecode turns against the
frozen D2 trajectory. Volatile production execution retains neither completed
input facts nor receipts nor delivered outbox entries, and the scene retains
only its latest 20-value position snapshot.

The public `examples/n-body` project separately preserves the original
fixed-Sun orbit-viewer equations. Its 4,096-turn regression and served-browser
smoke proof require a fixed `(300, 300)` Sun, stable orbital radii, motion, and
zero legacy turns.

The public bytecode format remains bytecode v1. D4 does not introduce a
pre-launch compatibility adapter or a second browser executor.
