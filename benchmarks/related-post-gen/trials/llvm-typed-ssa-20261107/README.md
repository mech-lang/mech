# Typed native SSA to LLVM

Compiler IR lowering and machine compilation are implemented; no LLVM Mech
executable image is qualified or timed. The selected compiler remains IR242.

Both benchmark sources compile through the owned typed SSA recorder and LLVM O3.
The closed machine audit accepts55 original and53 mask-reuse definitions, with
actual code/constants/frames. Mask reuse has555436B of machine code/constants,
root656B and largest function frame912B. These are size observations only.

Three real native helper tests pass, including10000 integer/Boolean cases and
10000 ABI cases. Full GPU896pass plus exactly two inherited failures; three LLVM
native tests are explicitly run separately rather than counted as default passes.
All399 CI-selected names are actually listed and executed. Both default bundles
retain all13 files byte-for-byte, including the complete Object and descriptor.

The initial verifier API error, stale SDK setup, C remainder-name conflict and
successive unsupported ABI/rotate/minmax refusals are retained. No failed attempt
is relabeled passed. Unsupported lowering fails explicitly.

Remaining: complete backend/toolchain/target identity, final machine admission,
same-Object assembly, independent loader and complete source/finite/ownership/
publication/runtime/old-Object/forgery qualification. The original descriptor is
never linked beneath this machine code. No resource or latency result is claimed.

Latest fetched DEFLATE5f5740 changes evidence/design only; production remains
equivalent to the already integrated b6a42. Its scalar low-word candidate was
rejected. The exact path/code-equivalence receipt is included.

Run `python3 verify.py` to verify all files and rederive machine metadata.
Executables stay local. LLVM IR and Rust source evidence are losslessly gzipped.
