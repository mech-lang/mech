# Cache the match offset slot in ordinary Mech

Each variant records the existing offset slot once in spare bits of the
match record. The four match readers decode the original nine-bit length.
Headers, rewind counts and previous-match words keep their original format.
The proof enumerates all writers and readers and checks the bounded encoding.
[Source and proof](PROOF.md) · [Table version](offset-cache-slot.patch.gz)
· [Arithmetic version](offset-cache-slot-arithmetic.patch.gz)

The same prebuilt compiler builds both variants and the unchanged source
with phase IDs, controller windows and measured placement disabled on all
three. This configuration establishes an independent comparison against
the **91,556-byte generic control**. The exact **71,768-byte retained
baseline** uses additional seeded layout and phase selection.

| Generic source screen | Encoder text | Exact linked calls |
| --- | ---: | ---: |
| Control | 91,556 B | 162 |
| Record the current table result | 77,236 B | 162 |
| Record the equivalent arithmetic result | 90,868 B | 162 |

The table producer changes private phase partitioning. Its largest
matching phase's general and vector versions together emit 7,260 bytes,
compared with 22,644 bytes for the control's largest matching phase.
The arithmetic form retains the broad phase shape while reducing repeated
scorer lookups. These are static sizes and output checks. No throughput
or tuned-size improvement is established by this screen.
[Exact command, sizes and artifact hashes](screen.json)
