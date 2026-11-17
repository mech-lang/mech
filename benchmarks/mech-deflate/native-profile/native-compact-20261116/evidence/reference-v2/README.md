# Fixed reference runtime replay under policy v2

The two archived complete orders of the exact 71,768-byte worker pass the
v2 runtime gate. Both orders are 81/81 against Roc and 61/81 against
libdeflate with every immediate-control peer win preserved. No timing
samples were acquired for this replay. Historical v1 retains its
Unresolved result and documented duplicate-control uncertainty.
[Replayed decision and plans](runtime-replay.json)
· [Historical raw observations](../bound-71768-runtime/README.md)

Replay from the review packet directory.

~~~sh
python3 -B evidence/reference-v2/replay.py . /tmp/mech-reference-v2.json
~~~

The replay covers the runtime gate. Current integration, explicit resource
requirements, and the observed later control drift retain their own
evidence in [the scorecard](../../CURRENT.md).
