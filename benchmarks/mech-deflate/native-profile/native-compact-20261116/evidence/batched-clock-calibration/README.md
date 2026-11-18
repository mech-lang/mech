# Batched clock calibration for the pinned compact encoders

The original seven-stored-prefix follow-up measured 98.8% of control calls
as zero or one 24 MHz clock tick. At its mean of 31.18 ns, the 0.5% resolution
represents 0.16 ns. This diagnostic tests timer amortization with a separate
checked adapter over the exact 71,960- and 71,768-byte native objects.

The adapter prepares one or 256 independent U8 input views, borrowed output
buffers, arguments and result slots outside the interval. The clock encloses
the batch loop, preallocated result writes, fresh invocation-local heap and
owner storage, native call, sole final Vec transfer, counter reads and
private heap drop. After the clock stops it checks exact output bytes,
capacity, independent inflation, unchanged aliases and zero remaining
private payload for every invocation. It then refreshes each lane's prior
output for the next reading. The additional live caller buffers and result
writes are protocol effects. This diagnostic compares the two exact objects
under the same adapter and documents the effect of grouping. Existing
qualification decisions remain bound to their original linked workers and
timing protocol.

A/A compares two byte-identical worker files with distinct inodes. A/B
compares the candidate against the geometric mean of both controls. Each
actor block contains 2,048 invocations with either 2,048 clock reads or
eight reads of 256 calls each. Two epochs independently shuffle all
six actor orders, with 32 repeats per order. The clock arms and case
orders are predeclared for empty, one-stored, seven-stored-prefix and
random-64. Validation and warmups take place serially and outside the
recorded timings. The replay uses the existing 90% paired-block bootstrap
with 8,000 draws, stratified by epoch and physical order. Each A/A interval
must fit ±0.5% in both epochs and their pooled replay. Every block is kept.

The all-fixture check verifies one call per actor for the 81 canonical cases,
with process cleanup receipts independent of the timing acquisition.

## Results

All 18,874,368 timed invocations and 243 smoke invocations passed. All
workers reported zero remaining private payload and owner records at exit.
The plan was frozen as 6c5e52ab87999ae5b39db22a0d53ab51893840f7f83ad14783be1ecedcdf5901.

| Case | Pooled A/A interval, one call per reading | Pooled A/A interval, 256 calls per reading | Both batched epochs meet ±0.5% |
| --- | ---: | ---: | --- |
| empty | +1.385% to +1.826% | +0.702% to +1.017% | No |
| one-stored | −1.928% to +1.063% | +1.520% to +1.962% | No |
| seven-stored-prefix | −0.485% to +0.019% | −0.075% to +0.278% | Yes |
| random-64 | −1.255% to +0.199% | −1.824% to −0.440% | No |

For seven-stored-prefix, 99.2% and 99.8% of the single-call control
readings in the two epochs were zero or one clock tick. Every 256-call
reading exceeded one tick. With the batched protocol, the 71,768-byte
encoder was 1.274% slower on this case, with a 90% interval from +1.157%
to +1.388%. The per-epoch intervals were +0.860% to +1.319% and +1.407%
to +1.505%. The other batched cases remain unresolved because their
replicate controls differ by more than the resolution. This is evidence
from the diagnostic relink. The [original v1 runtime receipt](../bound-71768-runtime/README.md)
still records 81/81 Roc and 61/81 libdeflate in both orders with an
Unresolved per-case decision.

## Replay

Run `python3 -B replay.py` from this directory. The script checks the frozen
plan, previous receipt, canonical fixture bytes, exact encoder objects,
metadata, diagnostic worker bytes, smoke calls and cleanup. It unpacks
the encoder objects, adapters, logs and raw readings to a temporary directory,
runs the recorded replay and checks the result against report.json. Replay
uses the Python standard library and reads the archived workers as data.
The pinned link commands and direct input hashes are in plan.json.
