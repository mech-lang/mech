# Zig scalar conditional ownership proposal

This is additive static review of the existing `boundary-audit-15/candidate-audit.json` and bounded original source slices. No source bytes, hashes, timings, collector data, frozen jobs, or admission decisions were changed. The working parser/fixtures have not been executed by this review.

The proposed grammar proves a complete `if (condition) [|capture|] value else [|error_capture|] value` within one surrounding binding, return, or call expression. Conditions and simple captures are structural. Calls, casts, member/index access, operators, and syntactic data constructors do not receive a callee-purity test. Bare executable blocks, declarations, assignments, transfers, loops, switch arms, and anonymous functions do not qualify. Complete surrounding values retain one executable composition, plus the original physical rows of any wrapped data constructors or literals. Statement windows stop before their semicolon or surrounding function close; the value event also owns its semicolon as a structural terminator, including a separately written terminator row.

## Existing exact source memberships

All paths below are relative to `benchmarks/clbg/zig-odin-roc-elm-comparison/20261010/sources/`. Counts are the **saved, gated job15 proposal**, not new results or admitted chart LOC.

| Source | Job15 source key | Physical / proposed LOC | Exact ownership review |
| --- | --- | --- | --- |
| `fannkuchredux-zig-2-m-faithful.zig` | `9509b7bb4bfe23e80e8bcfe15d4c6dd54732889f2e9c45d5a2cd2100116f0f7f:zig` | 142 / 150 | Lines 60 and 84 are imperative `if (i == n) return ...` within `inline while (...):(i += 1)`. An earlier loop-update assignment triggered the existing value-prefix diagnostic. They have no complete scalar else and remain gated in this phase. |
| `fannkuchredux-zig-2-faithful.zig` | `1668efcd04d4ac761bad809dfa286ba0fa357675544f114c591e529bb48e072a:zig` | 83 / 90 | Line 49 is the same imperative inline-loop/return form; it remains gated. |
| `knucleotide-zig-1-m-faithful.zig` | `88e5bbd6db9676219bd7b31d16fdcfff68b7f7f97eadf8f33c77f9fcce504b22:zig` | 177 / 207 | Lines 29 and 53 are scalar **switch** values and keep their switch-policy gates. Line 151 is imperative `if (m.get(code)) |count| total += count;`: a header/capture plus an assignment, with no implied close. Its existing capture gate stays. |
| `knucleotide-zig-1-faithful.zig` | `b99158c914eb8117470d41043f55de3a876ccd2535f99ec69edd0703fc3b9675:zig` | 170 / 204 | Lines 28 and 53 are scalar switch values and remain gated. Line 165 is a scalar captured conditional: `const count = if (map.get(Code.fromStr(s))) |x| x else 0;`. Only that complete surrounding binding is targeted. |
| `spectralnorm-zig-2-m-faithful.zig` | `4bafc2670d85538646976e90fc97a9f2ea97ca2a00300faceafeb01a1bd81aa4:zig` | 113 / 119 | Line 46 has `vec1to4(...) + if (transpose) vec4{...} else vec4{...}` within one `var` binding. Both vector braces are data, not executable closes. Line 48 is another complete scalar binding with call/arithmetic arms. |
| `pidigits-zig-gmp.zig` | `5a95d65236c8be8cfb4c1c52cc3a344b655e7631e133697d96aabb55bd38f490:zig` | 66 / 71 | Line 47 is `const used = if (produced % 10 == 0) 10 else produced % 10;`: one scalar binding. |
| `binarytrees-zig-arena-compact.zig` | `81af06556379389ee22cfdd45c50dab1e6134769f6def30ca67763d3d8415594:zig` | 51 / 54 | Line 17 returns `1 + (if (node.left) |left| check(left) else 0) + (if (node.right) |right| check(right) else 0)`. Both captures are formals inside one surrounding return expression. The following function close remains separate. |
| `binarytrees-zig-arena-parallel.zig` | `e73f078c62f6b500f9d6d22e8e750be5df5cf5ca100bd5182b6d58291ab32de9:zig` | 70 / 77 | Line 17 uses the same scalar return composition and structural captures as the compact source. |

The working fixtures reproduce these grammar forms rather than dispatching on file, benchmark, input, function, or callee names. The source keys above require a later locked parser/fixture audit and source-specific admission review; this note grants neither.
