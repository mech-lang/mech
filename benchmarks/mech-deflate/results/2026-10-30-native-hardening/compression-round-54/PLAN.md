# Count every allocation in the complete tiny compressor

The user prioritizes the 95-versus-13,484 allocation-request gap. Use the exact round-53 optimized libraries and its same-build Object/image; no production or benchmark source change. Capture every successful Rust global allocation/reallocation during the second complete eight-byte level-12 evaluate_into call. Keep the pinned allocator and all correctness, work, output-capacity, alias and cleanup checks.

Capture call stacks and request sizes in bounded static C storage, outside Rust allocation. Abort on event overflow or truncated stack. Print after the measured window, symbolize with the exact executable/load address, and reconcile event count, bytes and reallocation count with the independent existing counters. Instrumented elapsed and memory are diagnostic only. Archive raw evidence immediately. Find the most frequent generic runtime allocation sites before selecting a production change; large requested capacity is not evidence of equal copied bytes.

Round 53's frame transfer and qualification are retained and remain a separate change. No source-specific sizes, algorithms or weakened admission/lifetime checks may be introduced.
