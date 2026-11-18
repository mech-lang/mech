# Source-emission experiment

The optional Rust draft uses the finalized AArch64 layout, after the existing
cold-tail pass. It uses that pass's branch decoding and offset remapping for
external relocations, traps and call return addresses. Original unwind offsets
must map to themselves. The path requires the checked Apple AArch64 frame,
default source locations and supported relocation and body shapes. It declines
bodies with stack maps, patchable calls, exception handlers, debug ranges,
indirect branches or unresolved PC-dependent code.

The instruction classifier matches exact non-writeback 64-bit LDR and STR
operations relative to SP within the active frame. Candidates must reach the
same immediate successor as a copy sequence that already falls into it.
Internal targets and protected records delimit candidate intervals. Leading
instructions at multi-entry heads keep their own labels. The body proof
requires exact machine words, including architectural register and frame
displacement fields, so each old path performs the same operations in the
same order. All rewrites use the existing fixed-width branch range checks.
Compilation leaves a rejected pattern's original bytes in place.

The raw predicates identify the same seven suffixes and 152 instruction
deletions from the pinned object. The optional NOP fill in the new tail profile
reproduces the static prototype's unreachable function alignment. With no
setting, existing profile behavior is unchanged. This draft still needs to
compile and reproduce the measured bytes.
[Raw selection check](source-selection-check.log) ·
[Draft patch](source-draft.patch.gz)

Build the optimized diagnostic emitter with the patch and use the original
pinned bytecode, fixed runtime, adapters and allocator. Enable
MECH_COMPACT_SHARE_FRAME_COPIES and point MECH_COMPACT_ROOT_TAIL_PROFILE at
profiles/checked-root-frame-copies.json in the packet. Acceptance of the
source translation requires the exact 71,192-byte encoder and relocated
object, with identical non-size metadata and worker inputs. Reproduce the
older encoder with the option unset to check the control.

The focused 4 KiB random observations remain attached to their exact
worker. The source translation can establish a reviewable candidate.
Runtime qualification still requires both complete orders at the frozen
win floors and the declared per-case regression policy.
