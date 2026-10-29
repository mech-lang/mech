# Ordinary private-frame admission: baseline

Compiler checkpoint: `fae97122b0dff4506f8fcb93e7cee87b9a67e184`; production libraries are the unchanged round-13 libraries, including GPU production code at `1ec498a67fc862bac39188fdf5040eaacf6e954c`.

The added maintained test calls the actual prepared-frame API for an ordinary recursive two-buffer/U64/Bool argument tuple. It checks original and decoded artifacts, fresh lengths 0/0, 7/13, 262144/1500513, and 2/1, aliased owners, invalid Bool bits, managed-input bits, ten finite compute boundaries, the output-element ceiling, cleanup and retry. The actual baseline execution passed one test in 0.21 seconds; all 120 captured cases are retained. Successful work is 14328032 at every length. The original routine charges 1024 work for these four fields; the checked failures preserve the exact 64-work prefix. Final assertions for that prefix were checked against all captured baseline rows and will run against the changed implementation.

The first test build failed because its cleanup tuple moved a borrowed value alongside its observing closure. The next run exposed that its memory assertion included the live work-account allocation on only one side. Both were test-fixture mistakes, corrected before the successful capture; their original logs remain here. No production failure is claimed from either attempt.

The planned change belongs to the existing prepared call and frame-shape owners: combine only successful fixed-layout footprint debits, use the current actual packed-buffer capacities, and compare current already-validated shape axes without constructing an output shape. Any invalid input, relation miss, tight budget, or shared-worker account keeps the ordinary ordered path. Compressor source and resource policies remain unchanged.

These are regression receipts, not compression timings. The paired compression baseline for this round will be the retained round-13 executable (`59c15cf5021fa2947779334c0e4f8c17f1b393c3ca5d5e3dc955444f38e89bae`) and a fresh reference measurement, using the same eight-byte harness and allocator.
