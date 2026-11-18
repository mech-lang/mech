//! These tests inspect Apple ARM64 machine buffers without executing them, so
//! the maintained Linux backend job exercises the pass as well as Apple hosts.

use super::{B, Branch, apply, share_frame_copies};
use crate::binemit::Reloc;
use crate::ir::{ExternalName, LibCall, TrapCode};
use crate::isa::aarch64::inst::Inst;
use crate::isa::unwind::UnwindInst;
use crate::machinst::{
    MachBuffer, MachBufferFinalized, MachBufferFrameLayout, Stencil, VCodeConstants,
};
use alloc::vec::Vec;
use cranelift_control::ControlPlane;

fn checked_routes() -> MachBufferFinalized<Stencil> {
    // Four distinct origins followed by the same ADD/STR/jump suffix. The warm
    // dispatch includes CBZ, TBZ, B.cond and B; the join calls an external helper
    // and records a trap. Offsets here are bytes, branch immediates are words.
    let words = [
        0xb40000a0, 0x36000100, 0x54000160, 0x1400000e, 0xd65f03c0, 0xd2800221, 0x91001c00,
        0xf9000440, 0x1400000d, 0xd2800441, 0x91001c00, 0xf9000440, 0x14000009, 0xd2800661,
        0x91001c00, 0xf9000440, 0x14000005, 0xd2800881, 0x91001c00, 0xf9000440, 0x14000001,
        0x94000000, 0xd4200000, 0xd65f03c0,
    ];
    let mut buffer = MachBuffer::<Inst>::new();
    for (index, word) in words.into_iter().enumerate() {
        if index == 21 {
            buffer.add_reloc(Reloc::Arm64Call, &ExternalName::LibCall(LibCall::Memset), 0);
        }
        if index == 22 {
            buffer.add_try_call_site(Some(96), core::iter::empty());
            buffer.add_trap(TrapCode::user(7).unwrap());
        }
        buffer.put4(word);
    }
    buffer.finish(&VCodeConstants::default(), &mut ControlPlane::default())
}

#[test]
fn cold_tails_remap_dispatch_and_metadata_without_merging_origins() {
    let mut candidate = checked_routes();
    let control = candidate.clone();
    assert_eq!(apply(&mut candidate, 20..96), Some(24));
    let expected = [
        0xb40000a0_u32,
        0x36000100,
        0x54000120,
        0x1400000a,
        0xd65f03c0,
        0xd2800221,
        0x91001c00,
        0xf9000440,
        0x14000007,
        0xd2800441,
        0x17fffffc,
        0xd2800661,
        0x17fffffa,
        0xd2800881,
        0x17fffff8,
        0x94000000,
        0xd4200000,
        0xd65f03c0,
    ];
    let expected = expected
        .into_iter()
        .flat_map(u32::to_le_bytes)
        .collect::<Vec<_>>();
    assert_eq!(candidate.data(), expected);
    let mut metadata = control;
    metadata.data = candidate.data.clone();
    metadata.relocs[0].offset = 60;
    metadata.call_sites[0].ret_addr = 64;
    metadata.traps[0].offset = 64;
    // Every other field, including relocation identity and addend, call frame
    // offset, trap identity, alignment and unwind information, stays exact.
    assert_eq!(candidate, metadata);
}

#[test]
fn cold_tails_keep_branch_targets_inside_otherwise_shared_suffixes() {
    let mut candidate = checked_routes();
    // A warm edge enters directly at the second STR, so that instruction must
    // remain an independently reachable entry, with no preceding ADD inserted.
    candidate.data[0..4].copy_from_slice(&0xb4000160_u32.to_le_bytes());
    assert_eq!(apply(&mut candidate, 20..96), Some(16));
    assert_eq!(&candidate.data()[0..4], &0xb4000160_u32.to_le_bytes());
    assert_eq!(&candidate.data()[44..48], &0xf9000440_u32.to_le_bytes());
    assert_eq!(candidate.relocs[0].offset, 68);
    assert_eq!(candidate.traps[0].offset, 72);
}

#[test]
fn cold_tails_require_recorded_deferred_traps_and_decline_unsupported_relocations() {
    let mut deferred = checked_routes();
    deferred.data.truncate(92); // one recorded trap follows the emitted body
    assert_eq!(apply(&mut deferred, 20..88), Some(24));
    assert_eq!(deferred.traps[0].offset, 64);

    let mut unrecorded = checked_routes();
    unrecorded.data.truncate(92);
    unrecorded.traps.clear();
    let before = unrecorded.clone();
    assert_eq!(apply(&mut unrecorded, 20..88), None);
    assert_eq!(unrecorded, before);

    let mut unsupported = checked_routes();
    unsupported.relocs[0].kind = Reloc::Abs8;
    let before = unsupported.clone();
    assert_eq!(apply(&mut unsupported, 20..96), None);
    assert_eq!(unsupported, before);
}

#[test]
fn cold_tail_branches_retain_the_signed_arm64_limits() {
    for (word, shift, bits) in [(B, 0, 26), (0x54000001, 5, 19), (0x36000003, 5, 14)] {
        let branch = Branch {
            target: 0,
            shift,
            bits,
        };
        let limit = 1_usize << (bits - 1);
        let mask = ((1_u32 << bits) - 1) << shift;
        for (from, to, delta) in [
            (limit, 0, -(limit as i64)),
            (0, limit - 1, limit as i64 - 1),
            (limit, limit, 0),
        ] {
            let encoded = branch.encode(word, from, to).unwrap();
            assert_eq!(encoded & !mask, word & !mask);
            let extend = 32 - bits;
            let decoded = (((encoded >> shift) << extend) as i32) >> extend;
            assert_eq!(i64::from(decoded), delta);
        }
        assert_eq!(branch.encode(word, 0, limit), None);
        assert_eq!(branch.encode(word, limit + 1, 0), None);
    }
}


#[test]
fn exact_frame_copies_share_a_fallthrough_with_recorded_control_flow() {
    let mut buffer = MachBuffer::<Inst>::new();
    buffer.set_frame_layout(MachBufferFrameLayout {
        frame_to_fp_offset: 32,
        stackslots: Default::default(),
    });
    buffer.put4(0xd503_201f); // Keep the prologue offset and its unwind information.
    buffer.add_unwind(UnwindInst::StackAlloc { size: 32 });
    buffer.put4(0xb400_00a0); // cbz x0, copy2
    let copies = [0xf940_03e1, 0xf900_07e1, 0xf940_0be2];
    for word in copies {
        buffer.put4(word);
    }
    buffer.put4(0x1400_0004); // b continuation
    for word in copies {
        buffer.put4(word);
    }
    buffer.add_reloc(Reloc::Arm64Call, &ExternalName::LibCall(LibCall::Memset), 0);
    buffer.put4(0x9400_0000);
    buffer.add_try_call_site(Some(96), core::iter::empty());
    buffer.put4(0xd65f_03c0);
    let original = buffer.finish(&VCodeConstants::default(), &mut ControlPlane::default());

    let mut out_of_frame = original.clone();
    out_of_frame.frame_layout.as_mut().unwrap().frame_to_fp_offset = 16;
    let before = out_of_frame.clone();
    assert_eq!(share_frame_copies(&mut out_of_frame, 40..44), Some(0));
    assert_eq!(out_of_frame, before);

    let mut candidate = original.clone();
    assert_eq!(share_frame_copies(&mut candidate, 40..44), Some(12));
    let expected = [
        0xd503_201f,
        0xb400_0040,
        0x1400_0001,
        copies[0],
        copies[1],
        copies[2],
        0x9400_0000,
        0xd65f_03c0,
    ];
    let expected = expected
        .into_iter()
        .flat_map(u32::to_le_bytes)
        .collect::<Vec<_>>();
    assert_eq!(candidate.data(), expected);
    let mut metadata = original;
    metadata.data = candidate.data.clone();
    metadata.relocs[0].offset = 24;
    metadata.call_sites[0].ret_addr = 28;
    assert_eq!(candidate, metadata);
}
