# Elixir baseline source review packet

Authored source only. Ten algorithmic mains and one general helper; twenty explicit publication IDs are aliases of these ten baselines. Nothing is frozen, compiled, parsed, normalized, formatted, imported, or run. No source/compiler/ABI/runtime/work/LOC/timing/publication qualification is claimed. Root and peer review precede source freezing and any durable worker. Runtime research and dependency gates are in `../../RESEARCH_PLAN.md`.

Every source main is a separate `Clbg.Main` module, intended for its own exclusive per-form compilation directory with `Clbg.Support`. It has no top-level benchmark execution. Source input materialization remains false: physically separate compact/optimized copies are a later reviewed preparation step, and each needs its own compiler, work, bounded, and full receipts. The aliases make no optimization or timing-reuse claim. Include the complete shared helper and any subsequently authored runner in each actual count closure. Canonical/formatted owner grammars and actual source-count replays are pending.

| Main | Required work present in the authored body |
| --- | --- |
| binarytrees | Recursive fresh three-field tuple with `make_ref` for every node, including leaves; recursive traversal; stretch tree; every short tree at every even depth; long root passed through all iterations and traversed last. |
| spectralnorm | Full matrix coefficient loops, A then transposed A on each call, twenty AtA calls in ten rounds, final dot products and binary64 square root. |
| mandelbrot | Every pixel iterates up to 50 recurrences or escapes; authored per-row packing includes zero low-bit padding for widths not divisible by eight. |
| fannkuchredux | Complete factoradic carry and rotations, fresh tuple work copy for every permutation, all prefix reversal loops, signed rank checksum and maximum. |
| nbody | Five bodies, momentum offset, every pair and coordinate update for every requested step, two full energy traversals. Immutable tuple updates are explicit language work. |
| fasta | Exact ALU literal, authored repeated-sequence loop, cumulative probabilities and linear selection, RNG state continued from IUB into Homo, 60-column output. |
| revcomp | Every FASTA record preserved in order, authored reverse/complement work over bytes and all ambiguity pairs, upper-case output, wrapping, CRLF and comments handled. |
| knucleotide | Exact THREE token selected, ASCII upper-case validated, seven fresh growing maps for 1/2/3/4/6/12/18 and every sliding window, two frequency sort/output passes, five query lookups. No query-only map. |
| pidigits | Arbitrary integers in Elixir, independently evaluated `div(3*q+r,t)` and `div(4*q+r,t)` every extraction, complete spigot updates and padded ten-digit lines. |
| regexredux | Byte lengths, case-sensitive cleaning, all nine independent global match scans, all five ordered replacement passes. Vendor matching is a general primitive; no host loop. |

The shared helper owns argument handling, byte reads/writes, LF/CRLF/standalone-CR splitting, zero-length index ranges, and exact binary64 rational formatting. Latin-1 binary stdio configuration precedes input and output. Its bit decomposition uses sign/exponent/fraction, including subnormal values, exact powers of ten, quotient/remainder tie comparison and even rounding. It preserves negative zero. Runtime byte behavior, rounding, source syntax, and compiled semantics remain actual fixture gates.

Required meaning review includes the exact ALU/planet literals against the existing original-language baselines, evaluation order of nbody tuple updates, dynamic fresh tuple and reference allocation, scored compiler output showing original recursion/loop owners, original `make_ref` call behavior, both Pi divisions, seven map growth paths, all regex/replacement counts, and long-root retention through normal GC. No analytic node-count substitution, shared literal leaves, GC disabling, or helper omission is admissible.

Future evidence must bind immutable original and snapshots before importing services; preserve each failed command and its raw streams, started identity, sampler/native normal exits, errors, pins, work review, byte validation, and terminal integrity. Runtime support claims from Gleam do not qualify these source owners. Full timing remains blocked until all twenty exact source owners, bounded outputs, and runtime gates pass.

Independent source peer review on 2026-10-13 inspected all ten mains and the full helper without execution. Its findings were resolved: standalone CR support was added; vendor global regex scanning was replaced by single-match authored count and replacement cursor loops; fractional formatting digit arguments now fail the guard. The final FASTA line-chunk buffering was separately re-read for empty, full and partial line order/newlines and unchanged RNG continuity. No remaining source blocker was identified. This source review supplies no compiler/parser/fixture/runtime/owner-count/timing qualification; root review and all actual gates remain pending.
