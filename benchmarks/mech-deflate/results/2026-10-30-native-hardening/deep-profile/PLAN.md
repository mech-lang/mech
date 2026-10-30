# Executable comparison: where the remaining cost comes from

Use only the established complete eight-byte `abcdefgh` level-12 computation,
exact ten raw bytes, pinned allocator and source/algorithm. Keep diagnostic
observations separate from primary elapsed-time measurements and commit every
cohort immediately.

Produce a side-by-side scorecard for the frozen reference and Mech, recording:

- Actual retired CPU instructions, cycles and instructions per cycle. Use the
  available macOS process counters at compression boundaries; measure empty
  boundary overhead and keep it visible. Static instruction counts are a
  separate code-size measure.
- Allocation/reallocation counts and requested bytes, live allocator bytes and
  peak live bytes during compression. Identify large payload allocations and
  relocations. Instrumentation uses the same pinned mimalloc; instrumented
  elapsed times are not primary performance results.
- Resident/physical memory and whole-executable peak, with preparation/JIT
  memory distinguished from compression memory. Report the current test
  runner's embedded compiler separately from generated compressor code.
- Native code, executable and metadata sizes; reached callback/operation counts;
  sampled function hotspots and hardware bottleneck evidence where available.
- Whether input is streamed, retained by reference or copied, and where large
  compressor buffers are initialized, cloned or reallocated. Distinguish an
  immutable handle clone from a payload copy.

Starting published compiler is round 36, source 9047e634e5964ad23ae19449bc425c2a3cf325b2.
The pending round-37 code-layout experiment is source
3a54f1a1d64db9eee01db97763a3f95a138db93a. Its four-pair median changes only
4.6680415 → 4.6539165 ms (0.30%, mixed pairs); no confirmed new latency gain.
Its helper shrinks from 6294 to 4901 static instructions and 2416 to 1952 bytes
of Rust entry stack. Measure before claiming those reductions save CPU work.

The result must identify concrete, ranked simplifications, the evidence behind
each, and a small set of repeatable counters that can be tracked after future
changes. No additional compiler optimization is started before this comparison.
