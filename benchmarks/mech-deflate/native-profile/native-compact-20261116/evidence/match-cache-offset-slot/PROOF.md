# Cache the offset slot in each match record

This is a source-only experiment against PR #993 at
`78b064c5bb70937db72db21ebd3ab2fd556816c3`. Its public entry is
`deflate-compress-into(input, output)`. It changes the compression entry's
private cache representation while preserving the match search and scoring
rules. Direct observation of the matchfinder's private scratch array will
see the new encoding. Standalone fixtures and the public encoder ABI are
outside this patch.

## Exact cache roles

| Role | Consumer or producer | Geometry |
|---|---|---|
| Match record writes | `bt-search-step`, `bt-short-record` | Same store and cursor; cache length in bits 0–8, slot in 9–13, offset in 16–31 |
| Header writes | `optimal-collect-position`, `optimal-advance-position` | Same full low-half match count and high-half literal |
| Latest match length | `optimal-collect-position` reads `next-cache[next-pointer]` only on an advanced pointer | Mask 511 before deriving scan skip and returning to the prefix witness |
| Latest match length | `optimal-advance-position` reads `entry` only on an advanced pointer | Mask 511 before sampling, determining skip, and constructing the prior-match witness |
| Older position witness | `searched-match`, `bt-skip-prefix-step`, `bt-advance-skipped` | Separately reconstruct `(length << 16) | offset` from decoded last match or reuse a verified tuple. Preserve both field readers |
| Match score | `OptimalPathMatches` and `PackedOptimalPathMatches` | Both mask 511 and keep the original checked `3 <= length <= 258`. The legacy scorer uses the same caller-supplied slot table |
| Stored count/literal | `optimal-path-position`, `optimal-packed-path-position` | Keep both full-half fields |
| Rewind | `optimal-rewind-cache` | Keep its full low-half header count |
| Copy or retire cache | `optimal-move-carry`, `optimal-carry-rewound`, `optimal-carry-reset` | Copy complete U32 words or reset a cursor with identical capacity/layout |

The short recorder writes length three. The searched recorder starts only with
`length < maximum` and writes after a successful length extension greater
than the current best, initially three. The word branch only consumes bytes
within `maximum`; the first differing byte in a checked eight-byte load is
at most seven bytes into that load. The byte tail only advances until
`maximum`. The production collector passes
`min(remaining, 258)`; the separate prefix witness starts with `maximum=258`
and maintains the same bound. Thus every written match length fits nine bits,
including 258. Both source length assertions remain in place. The default
zero from a missing cache read still decodes to offset, length and slot zero,
so the existing invalid-length route is preserved. A raw old-format malformed
word is outside the new private cache contract.

The encoded offset is exactly the previous upper half, including truncation
to u16. The primary producer uses the original slot function on that offset
with the exact named immutable table passed by the compression controller.
The slot fits five bits, so the three fields cannot overlap. Bit extraction
uses the same offset, length and prices in the same match/candidate order.
This preserves the wrapping cost arithmetic, strict tie selection and
emitted item. The entire packed scorer then drops its unused slot-table
argument. Tallying the chosen path and writing the stream continue to decode
offsets from their separate, unchanged item representation.

## Arithmetic producer option

`offset-cache-slot-arithmetic.patch` avoids a new constant dependency in the
producer. For offset d in 1–32,768, set x=d−1. Slots are x for x<4. Otherwise,
let h=floor(log2(x)). The slot is 2h plus bit h−1 of x. The original algorithm
uses its 256-element table directly for d<=256. Above 256 its wrapping
subtraction produces shift 7 in the entire u16 domain. Right-shifting x
therefore removes only bits below the leading pair for x>=256, and adding 14
corrects the removed seven places in 2h. The original out-of-bounds defaults
are d=0 yielding zero and d>32,768 yielding 14. The CLZ form handles both.
`prepare.py` verifies equivalence for all 65,536 stored offsets and checked
source-length boundary round trips. The alternative does not add a table or
one more long-lived matcher capture.

## Cost evidence and qualification

The scorer's archived block counts on 8 KiB natural text record 86,904 visits
to its dependent cache offset, slot-byte lookup, offset-price load chain.
The two match stores in the archived matcher execute 7,892 long and 6,592
short times, totaling 14,484. The matcher symbol's 6,768 bytes are identical
in that archived object and the exact 71,768-byte control. The observed
producer and consumer counts show six slot lookups per stored match. Moving
the original lookup to the producer removes 72,420 repeated lookup requests
and the scorer's table capture. The scorer also emits a two-instruction
reacquisition of that table address in a block visited 43,020 times.
The new scorer still needs one bitfield extraction and one price load.

The existing table helper adds lookup and packing work to each of 14,484
stores. In its arithmetic form the normal path needs a count-leading-zeros,
index arithmetic, bit extraction and selection, followed by packing.
Compiler range proofs and instruction selection determine the actual cost.
The dependency eliminated on the scorer's hot path is a data load feeding
another data-load address. That difference can matter independently of
instruction count. The new producer is unmeasured, and the release recipe
may generate different matcher captures or table objects. Measure those
physical effects before making a runtime or size claim.

First check exact source compilation and range admission, and bind the new
object, origins and worker. Source-indexed diagnostic profiles need fresh
validation. Compare code size and private payload/stack before timing.
Exercise longest matches, both short-history and normal writes, boundary
positions, rebase, rewind, following blocks, and aliasing under both
correctness orders. Preserve the prior checked failures and origins.
Run the accepted 81-case two-order qualification, including empty/one-stored
and natural 8 KiB. No builds, links or timing were run for this draft.
