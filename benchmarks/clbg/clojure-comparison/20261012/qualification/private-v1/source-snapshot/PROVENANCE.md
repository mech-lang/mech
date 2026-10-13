# Source and runtime provenance

All twenty applications are locally authored Clojure adaptations of the benchmark specifications and the retained independent Python references. The compact and optimized source forms are both complete applications; producer LOC is null. All local application/support source is scored after canonical normalization. No foreign benchmark program is called.

binarytrees makes a fresh two-slot object array before testing depth, including each leaf. It traverses real child references and uses the retained root after all temporary tree work. optimized divides each depth's prescribed iteration count into at most eight disjoint intervals.

fannkuchredux enumerates every permutation with classic prefix rotation/carry, copies each permutation for actual prefix flips, and computes parity checksum and maximum. optimized initializes exact factorial-ranked intervals then carries through each interval; all worker results are joined and reduced.

nbody uses the five canonical initialized bodies, offsets momentum, updates both velocities for all ten pairs in each step, moves positions after all pair impulses, and computes before/after energy. optimized stores all actual pair displacements/magnitudes in reusable arrays; it retains canonical update order.

spectralnorm evaluates the actual triangular coefficient for each dense row/column operation, with forty full matrix-vector products from ten two-phase power rounds, then reduces the final norm. optimized partitions only output rows and joins each phase before its dependent transpose/product.

mandelbrot performs the actual fifty-step escape recurrence for every pixel, writes MSB-first PBM bytes including zero padding, and handles n=0. optimized partitions disjoint rows and shares immutable real coordinates.

pidigits uses ordinary Java BigInteger storage/arithmetic from Clojure. Both forms independently compute (3*q+r)/t and (4*q+r)/t for every extraction; equal quotients alone admit digits, with simultaneous recurrence updates. optimized caches small coefficients and buffers output, never answer digits.

fasta produces every base using the canonical ALU cycle or seed-42 LCG/cumulative probabilities, with shared seed across random records, exact 2n/3n/5n lengths, headers, sixty-column lines and tails. optimized buffers byte IO.

revcomp parses full FASTA records with CRLF/CR/LF, comments, EOF and empty records, ignoring preheader sequence. It performs every actual reverse/complement operation over all uppercase/lowercase IUPAC symbols including U. optimized mutates a fresh byte buffer rather than delegating reversal.

knucleotide selects only the first exact >THREE token, then builds all seven complete histograms for widths 1,2,3,4,6,12,18. compact uses transient persistent HashMap storage over all string windows; optimized uses seven growing java.util.HashMap objects with Clojure rolling-key windows and private worker ownership through join. Queries never replace complete histograms.

regexredux uses the default case-sensitive java.util.regex Pattern/Matcher engine for nine nonoverlapping counts and five ordered literal substitutions. Input is decoded ISO-8859-1 so every byte, including NUL, remains one code point for this byte-oriented ASCII specification. optimized schedules all nine independent counts on the capped generic pool, then runs ordered replacements.

support/clbg/support.clj supplies argument parsing, whole-stream byte-preserving input, Locale.ROOT formatting with explicit Writer flush and a generic Callable pool. Its entire authored source counts in every application. The optimized buffered Pi output also explicitly flushes the Clojure Writer. Clojure's [generated main](https://github.com/clojure/clojure/blob/clojure-1.12.6/src/clj/clojure/genclass.clj#L448) invokes the application without a flush; [RT.java](https://github.com/clojure/clojure/blob/clojure-1.12.6/src/jvm/clojure/lang/RT.java#L178) binds an OutputStreamWriter, and [core print](https://github.com/clojure/clojure/blob/clojure-1.12.6/src/clj/clojure/core.clj#L3499) does not flush. This source-only correction ensures complete stdout at normal fresh-process exit. Clojure core, clojure.string, Java standard libraries and the JDK are pinned vendor dependencies.

Official runtime release: https://github.com/adoptium/temurin25-binaries/releases/tag/jdk-25.0.4.1%2B1 . Planned archive: OpenJDK25U-jdk_aarch64_mac_hotspot_25.0.4.1_1.tar.gz, aarch64 mac HotSpot full JDK. Publisher digest and actual native/runtime identity remain unverified until guarded receipts. Existing MATLAB results/java-install.json and results/builds.json describe its separate ARM64 JRE; this candidate preserves them.
