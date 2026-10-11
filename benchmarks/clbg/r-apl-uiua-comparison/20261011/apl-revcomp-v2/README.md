# GNU APL reverse-complement capacity revision

These additive candidates preserve the failed original applications and all their
evidence. The unchanged GNU APL runtime uses 16-byte cells, so the canonical
254,166,745-byte input cannot be retained as a single array below its macOS
workspace fallback. These sources keep all reverse-complement work in APL and
use ordinary temporary-file I/O to bound live arrays.

The compact source reads standard `fgets` fragments. Its line/header/comment
state survives fragments, including long unwrapped sequences, comments and
headers. The optimized source reads 245,760-byte chunks and uses general regex
positions to handle headers/comments while bulk-spooling sequence slices.
Neither retains a complete input, record, header or line in APL memory.

At each record boundary, APL seeks through the current sequence file backward
in slices of at most 245,760 bytes, reverses each slice, and applies the complete
upper/lower IUPAC complement mapping. Since 245,760 is divisible by60, all full
slices contain complete output lines; only the final slice can have a partial
line. All headers and sequence output are written. CRLF, semicolon comments,
empty records and missing final newlines are handled explicitly.

`run-apl-scratch.sh` is a general standard-I/O launcher. It allocates a private
scratch pathname, forwards interpreter arguments and removes the scratch file
on exit. It never reads input or transforms output. The counted APL application
opens/writes/seeks/reads/closes the file using GNU APL's ordinary `⎕FIO` APIs.
The separate metadata recipe pins the launcher and its standard shell utilities.

Run small/adversarial validation under the shared exclusive timing lock, then
create the additive manifest:

```sh
python3 validate.py --wait-for-lock
python3 prepare.py
```

Validation covers the canonical small output and21 adversarial cases per source,
including all IUPAC characters/cases, comments/CRLF, empty records, no final
newline, input/output chunk boundaries and multi-chunk headers/comments. The
reference is an independent complete-byte Python implementation and its canonical
small result is checked against the already independently pinned output hash.
Full measurements belong to the guarded parent harness; no extrapolation is used.
