Private-v1 stopped at its first C build and remains unchanged. It verified the
official compiler archive/version and privately fetched dependencies, but ran
zero native applications. Its complete compiler stdout/stderr/cache and stopped
report are preserved; no measured or qualified native result is inferred.

The diagnostic is the standard wrapper's optional usePcreHeader mode: Pcre and
ExtraData remain Nim object types while imported procedures acquire pcre.h
prototypes. LLVM 22 correctly rejects the incompatible pointer types. The
ordinary default wrapper instead loads the PCRE library dynamically and uses
its own matching procedure-pointer declarations.

The additive v2 runner changes only the optional header-mode build flag and its
isolated fixture/proof module selection. It uses the default binding without
changing compiler, standard library, public packages, warnings or any of the
twenty application sources. Explicit pinned real PCRE 8.45 library directories
and both install-name lookup/target pins remain in effect.

An added unscored primitive contract checks the actual dynamic PCRE version,
LP64 ExtraData size/alignment, nonoverlapping matching, case sensitivity and
ordered replacements. The prior generic BigInts/GMP, ORC/ASAN and actual seven
map proofs remain. All full timing/admission gates remain closed.

The unexecuted proposed next metadata phase is guarded_prepare_v2.py --seal
with a new proposals/private-v2.json path. It runs ten mocked fixtures against
the v2 guard and creates new receipts, preserving all v1 bytes. Root must review
that concrete source and resulting exact digest before a separate --run with
an explicit fresh private-v2 revision. No retry or new queue has launched.
