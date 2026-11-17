# Terminal owner publication experiment

The terminal byte-append helper initializes a new owner slot with one
reference. The prototype removes a repeated assignment after the byte
copy, sets the count once for the fresh heap fallback, and publishes
the output at the original write. Two workers relink the same fixed
support with that runtime source, one with the 71,768-byte encoder and
one with the [cold-origin table](../cold-origin-tables/README.md).
[As-run source and link commands](as-run/prepare.py) · [Worker identities](as-run/worker-builds.json)

| Apple M1 focused candidate | Encoder text | × Roc text | × libdeflate text | Empty-input change | 90% paired interval |
| --- | ---: | ---: | ---: | ---: | ---: |
| Owner publication | 71,768 B | 1.244 | 1.935 | +1.146% | [+0.571%, +1.717%] |
| Owner publication and packed origins | 71,256 B | 1.235 | 1.922 | +2.375% | [+1.834%, +2.915%] |

Each worker passes 162 exact linked calls. Each preserves all individual
peer wins from its simultaneous control in both nine-case orders.
The smaller prototype records nine Roc and six libdeflate wins in
each screen. Both variants exceed the 0.5% regression resolution on
empty input. The packed variant also has conflicting one-stored timing
bounds. The candidate decisions are **Reject**.
[Raw observation references and per-case bounds](report.json)

Replay the archived focused orders from this benchmark packet.

```sh
python3 -B evidence/terminal-owner-publication/replay-archive.py
```
