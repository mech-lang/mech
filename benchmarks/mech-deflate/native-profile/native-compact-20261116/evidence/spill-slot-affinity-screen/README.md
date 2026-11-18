# Stack-slot affinity screen

This opt-in allocator change uses bounded exact live-range footprints and
prefers a spilled predecessor's slot when its contents cannot interfere.
The AArch64 allocator checker validates allocation before and after existing
spill forwarding. Only the affinity option differs between the two
unpadded workers. A separate build reproduces the exact published
71,192-byte object and worker with the original padding and options.

Both unpadded variants emit **71,144 bytes** of encoder text,
**1.233× Roc** and **1.919× libdeflate**. Each has a 50,632-byte root and a
3,728-byte root frame. The backend records 163 candidate spill edge
occurrences and zero preferred slot assignments. Both outputs retain 304
consecutive scalar stack copies. The candidate changes some stack-slot
offsets and one unconditional branch target. All three linked checks pass
162 exact calls. The size screen rejects the change. Performance and
stack high-water were outside this static screen.
[Exact artifact and command receipts](proof.json) · [Prototype patch](source.patch.gz)
