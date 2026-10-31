# Checked-debit host code review

Immediate baseline compiler: `a27ea040845b41a63fe29f6837773860bea8607c`. Candidate compiler: `78209b2aab2175f1fad92ee4daae3d6b38669d36`.
Baseline linked worker SHA-256: `343d1ccefa8e9735616b62ce48f8e0c99d34baf6ecc75a6a098b59b907afec3e`.
Candidate linked worker SHA-256: `3a5bf4f088be3ded81b0fbc2e1d78a05c4dac527dd5b0b1f0710200dfe2ec590`.

The actual compute-ceiling caller no longer calls the outlined `charge_in_context` helper. Its constant comparison increment is specialized to zero. The previous post-helper 40-byte Result copy (two vector loads, vector store and eight-byte load/store) is absent. The candidate still writes the final Result and preserves the worker-loan Result handling; this does not eliminate every result transport.

Both current compute loads/additions remain, on opposite sides of the thread-local access. TLS resolution remains. The candidate also materializes increment/restore stores for the first immutable RefCell borrow that the baseline had optimized away. No repeated-compute-load, TLS or immutable-borrow saving is claimed.

| Static function measure | Baseline | Candidate |
|---|---:|---:|
| Compute-ceiling caller instructions, including cold paths | 73 | 158 |
| Ordinary charge caller instructions, including cold paths | 49 | 133 |
| Outlined shared debit helper instructions | 99 | absent |
| Compute-ceiling caller's own stack frame | 96 bytes | 128 bytes |

Inlining grows the individual callers and the ceiling caller's own frame by 32 bytes. Those function counts are not dynamic costs or the complete host ledger. The removed helper had its own 96-byte frame; no claim about complete simultaneous execution-stack depth follows from these excerpts.

Whole linked `__text`: 60,829,048 → 60,823,288 bytes (-5,760). Worker file size changes by -256 bytes. The freshly emitted compressor Object, descriptor and metadata remain byte-identical; generated entry stack stays 107,344 bytes.

Separate whole-call hardware measurements observe 41,623,860 → 41,081,843 retired instructions (−542,017 / −1.302179%). Primary elapsed is 0.997399942× baseline (−0.260006%): three of four pairs improve, with the first pair adverse by 0.012425%. Both allocation censuses remain 8,105 requests. Static assembly is evidence of the mechanism, not an allocation ledger or a count of successful dynamic visits.

Raw bounded excerpts, exact extraction commands, per-instruction records and image pins are in `host-assembly` receipt archives and `host-assembly-comparison.json`. Tests preserve ceiling/overflow before foreign-scope rejection, tighter call caps, paid prefixes, current accounts, failure state and recovery. The source diff is one inlining attribute.

All twelve scalar controls pass their existing 1.05 timing-ratio ceiling with unchanged allocation checks. The largest adverse observed ratio is 1.016217325 (decoded 64-iteration loop). These current observations remain in the raw scalar receipts; no scalar-wide latency improvement is claimed.
