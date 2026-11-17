# Owner exchange and the empty input placement test

The first owner-exchange worker remains a v1 **Reject**. Its matched 19-case acquisition ran both orders. Empty took **+1.187%** longer on the combined estimate, with the 90% paired interval **+0.630% to +1.728%**. The original decision and raw order receipts are pinned in path-proof.json.

The empty fixture sends a zero-length input and prefix with capacity five through the existing terminal path in mech_compact_closed_v2. It emits the five exact stored-block bytes through mech_compact_finish_byte_appends at the same linked address. The checked controller, where the owner exchange runs, is bypassed.

The wrapper's 41 selected instruction words, the host's 96 setup/output-transfer/cleanup words, and the selected small-allocation code agree at the same addresses. The finish helper loads the same 16-byte inline owner initializer through differently relocated instructions. The two original orders have 49,152 samples per Mech actor per order with the same one five-byte payload allocation, zero copy and zeroing, and zero nonreturned private bytes. The measured empty path touched 1,520 stack bytes in both original linked workers.

| Worker | _clock_gettime import stub | Address mod 64 | Address mod 128 | Twelve-byte stub crosses either boundary |
| --- | --- | ---: | ---: | --- |
| Published retained | 0x10007485c | 28 | 92 | No |
| Unpadded size control | 0x10007484c | 12 | 76 | No |
| Owner exchange, rejected | 0x10007467c | 60 | 124 | Yes |
| Owner exchange, +80 B | 0x1000746cc | 12 | 76 | No |

Both Instant::now and Instant::elapsed call Timespec::now through the stub at fixed site 0x1000260c8 in the linked executable. The first stub instruction and following pair lie in different address regions only in the rejected worker. All four contain identical stub bytes. This is a concrete placement mechanism, independent of owner-exchange instructions or allocation work. A timed ablation is needed to test whether it accounts for the observed slowdown.

The placement experiment uses the compiler's existing exact checked-root tail profile and the preserved compiler. It adds 80 B after the 50,708-byte controller and restores all shifted import stubs to the same addresses modulo 128 as the matched size control. The new executable has **71,368 B** of encoder text, **384 B** below the unpadded size control. The exact bytecode and metadata reproduce the rejected variant. Its new linked worker starts with SHA-256 **2df2496ee60f** and passed all **162 exact calls**. Runtime qualification remains pending for that worker.

Reproduce with a fresh label in the same study directory:

```sh
python3 -B /private/tmp/mech-root-transport.A9Qtzt/empty-placement/emit.py padded-128-reproduction
python3 -B /private/tmp/mech-root-transport.A9Qtzt/empty-placement/trace.py padded-128-reproduction
```

The source/relink commands, options and artifact identities are in padded-128/proof.json. The trace preserves the original Reject and never assigns that receipt to the new worker.
