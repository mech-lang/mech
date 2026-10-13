# Kotlin/JVM baseline source review packet

Authored source only. Ten mains plus one general I/O/format helper; twenty compact/optimized IDs explicitly alias the ten baseline algorithms. No distinct optimizer is claimed. Nothing is frozen, compiled, parsed, normalized, formatted, imported, or run. All source/compiler/runtime/ABI/work/LOC/timing/chart gates remain false or null. Root and peer review are required before freezing or a durable worker. See `../../RESEARCH_PLAN.md` for exact official payload and proposed JVM lineage.

Compile each proposed form separately with `support/Support.kt` and one `sources/BENCHMARK/Main.kt`. Each main has a unique `clbg.BENCHMARK` package and ordinary top-level `main(Array<String>)`; final actual class owner/main dispatch must be verified from saved generated classes. Physically distinct per-form source inputs remain unmaterialized. Future byte copies preserve the baseline body, then receive independent compilation, count/work reviews, and fresh timings. Never copy one form's timing into the other alias.

| Main | Required authored work |
| --- | --- |
| binarytrees | `new Node` semantics on leaves and internal nodes; recursion on both children; real check traversal; stretch, all short tree repetitions, long root retained to last check. |
| spectralnorm | All n² coefficient/multiply loops, both A and transpose, ten pairs of AtA, complete dot products and binary64 sqrt. |
| mandelbrot | Every pixel and up to fifty recurrences; authored row bit packing and zero low-bit PBM padding. |
| fannkuchredux | All factoradic permutations and rotations; `copyOf` work per permutation; all reversal iterations; signed rank checksum and maximum. |
| nbody | Five separate mutable body arrays; momentum correction; all ten pairs, three axes and requested steps; full initial/final energy calculations. |
| fasta | Exact ALU data, repeat loop, cumulative and linear probability selection, all draws, seed continuity, three headers and wrapping. |
| revcomp | Every record, all reverse/complement positions and ambiguity characters, normalized upper-case byte output, CRLF/comments and wrapping. |
| knucleotide | Exact THREE token, ASCII validation, seven independent initially empty HashMaps with all substring windows for 1/2/3/4/6/12/18, two frequency rankings and all queries. |
| pidigits | General BigInteger values, both full divisions for three/four extraction every iteration, all language spigot updates and formatted lines. |
| regexredux | Reversible byte view, all nine case-sensitive Matcher.find loops, all five ordered replacement passes and byte-equivalent lengths. |

`Support.kt` is counted for every form in full. It owns argument parsing, raw byte input, ISO-8859-1 views, raw byte output, line normalization, exact binary64 HALF_EVEN formatting, and negative sign restoration for rounded zero. General JDK BigInteger, BigDecimal, Pattern and I/O implementations are vendor primitives with explicit version/tree provenance; all authored glue and algorithmic work stays Kotlin and must appear in source ownership counts. No handwritten Java/C benchmark kernel or vendor benchmark loop is used.

Peer review must examine actual bytecode owners from the exact saved class artifacts for recursion/newarray/new Node, real loops and maps, both BigInteger divisions, helper calls, and final long-root use. Original scored classes need bounded fresh-leaf identity and normal-GC retention checks; retain any required JIT/allocation evidence separately. JVM source-level construction and runtime image architecture alone do not prove scored native allocation or method JIT status. Default collector remains enabled.

Full acceptance requires all twenty exact source inputs and complete authored closure, reviewed canonical/formatted owner evidence, native ARM64 JVM identity/library/GC/flag proof, bounded outputs and work proof, then the sealed 90-record study. Preserve actual partial/failure evidence and exact original pins under the common lock; no retry/resume, inferred bytecode qualification, timing reuse, or partial dataset publication.

Independent source peer review on 2026-10-13 inspected all ten mains and the complete helper without execution. Its concrete standalone-CR FASTA parsing finding was resolved and re-read. The final regex cleanup/replacement path uses an authored Matcher.find loop with untouched-prefix/literal-replacement/cursor/suffix assembly and was also re-read; all five substitutions contain literal replacement text. The peer identified no remaining source blocker. Compiler syntax, output/bytecode/runtime/owner/LOC/timing qualification and root review remain pending.
