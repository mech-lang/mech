# Share owner-release call setup in the checked entry

The Apple AArch64 checked entry saves its invocation heap pointer in an explicit stack slot. Five private assembly entries recover that pointer and tail-branch to the original ordered release functions. The ABI used by the runtime stays the same. The compiler only redirects direct imports with the original checked heap parameter and an exact release signature.

The checked entry redirects 193 release calls and saves 324 B. The five shared entries add 84 B for a net saving of 240 B. Every other existing encoder function matches the control in machine bytes and relocations. All 162 linked success observations and six checked failures match, with a peak stack of 4,784 B. [Emitted functions](size.json) · [ABI check](abi-proof.json) · [Linked checks](linked-checks.json) · [Stack analysis of the control](control-stack-accesses.json)

| Runtime order | Roc wins | libdeflate wins | Natural 8 KiB vs libdeflate | 320-byte overlap vs libdeflate |
| --- | ---: | ---: | ---: | ---: |
| Forward | 81/81 | 62/81 | 0.96371× | 0.98533× |
| Reverse | 81/81 | 62/81 | 0.97824× | 0.98711× |

Both orders use 96 balanced blocks per case and time compression through heap cleanup. [V3 qualification](full-screen.json) · [Forward observations](full-forward-observations.tar) · [Reverse observations](full-reverse-observations.tar)

Encoder text is **65,036 B**, **1.127× Roc** and **1.754× libdeflate**. MechDeflate source through final executable link takes **2.175 s**, **0.732× Roc** and **4.553× libdeflate**. The measurements use eight matched fresh builds on Apple M1 with each compiler and fixed deployment support prebuilt. [Deployment and phase timings](deployment.json) · [Raw build data](build-comparison.json.gz)

Apply the compiler patch to the pinned parent, build the compiler, and run the replay. The opt-in flags and existing portable compiler overlays reproduce the exact linked encoder and canonical bytecode. [Compiler source pin](source.json) · [Compiler command](compiler-build.json) · [Flags and profiles](candidate.json) · [Verified replay](verified-replay.json)
