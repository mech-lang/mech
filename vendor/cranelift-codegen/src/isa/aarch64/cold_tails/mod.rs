//! Share identical tails after allocation without changing any warm instruction.
//!
//! Only straight-line words followed by a jump to the same destination qualify.
//! A shared tail contains no call, relocation, trap, label in its interior, or
//! PC-relative instruction. Its allocated register/stack operands match exactly.

use crate::binemit::{CodeOffset, Reloc};
use crate::machinst::{FinalizedRelocTarget, MachBufferFinalized, Stencil};
use alloc::{collections::BTreeMap, vec, vec::Vec};
use core::ops::Range;

mod frame_copies;

const B: u32 = 0x1400_0000;
const MAX_TAIL: usize = 40;

#[derive(Clone, Copy)]
struct Branch {
    target: usize,
    shift: u32,
    bits: u32,
}

impl Branch {
    fn encode(self, word: u32, from: usize, to: usize) -> Option<u32> {
        let offset = i64::try_from(to).ok()? - i64::try_from(from).ok()?;
        let limit = 1_i64 << (self.bits - 1);
        if !(-limit..limit).contains(&offset) {
            return None;
        }
        let mask = (1_u32 << self.bits) - 1;
        Some((word & !(mask << self.shift)) | (((offset as u32) & mask) << self.shift))
    }
}

#[derive(Clone, Copy)]
enum Flow {
    Plain,
    Barrier,
    End,
    Branch(Branch),
}

fn flow(word: u32, at: usize) -> Option<Flow> {
    let field = if word & 0x7c00_0000 == B {
        // Includes BL: its target must move even though calls cannot be shared.
        Some((0, 26))
    } else if word & 0xff00_0010 == 0x5400_0000 || word & 0x7e00_0000 == 0x3400_0000 {
        Some((5, 19))
    } else if word & 0x7e00_0000 == 0x3600_0000 {
        Some((5, 14))
    } else {
        None
    };
    if let Some((shift, bits)) = field {
        let extend = 32 - bits;
        let displacement = (((word >> shift) << extend) as i32) >> extend;
        return Some(Flow::Branch(Branch {
            target: at.checked_add_signed(displacement as isize)?,
            shift,
            bits,
        }));
    }
    if word & 0xffff_fc1f == 0xd65f_0000
        || matches!(word, 0xd65f_0bff | 0xd65f_0fff)
        || word & 0xffe0_001f == 0xd420_0000
        || word & 0xffff_0000 == 0
    {
        return Some(Flow::End);
    }
    if word & 0xffff_fc1f == 0xd63f_0000 {
        return Some(Flow::Barrier); // BLR: the return address remains distinct.
    }
    if word & 0x1f00_0000 == 0x1000_0000 // ADR/ADRP
        || word & 0x3b00_0000 == 0x1800_0000 // literal loads/prefetch
        || word & 0xff00_0000 == 0x5400_0000 // unknown conditional encoding
        || word & 0xfe00_0000 == 0xd600_0000
    // computed/unknown branch
    {
        return None;
    }
    Some(Flow::Plain)
}

#[derive(Clone, Copy)]
struct Replacement {
    target: usize,
    width: usize,
    emit_branch: bool,
    redirect_branches: bool,
}

#[derive(Clone, Copy)]
enum TailKind {
    Cold,
    FrameCopies,
}

/// Returns bytes removed, or `None` without modifying unsupported input.
pub(super) fn apply(
    buffer: &mut MachBufferFinalized<Stencil>,
    cold: Range<CodeOffset>,
) -> Option<usize> {
    apply_kind(buffer, cold, TailKind::Cold)
}

pub(super) fn share_frame_copies(
    buffer: &mut MachBufferFinalized<Stencil>,
    cold: Range<CodeOffset>,
) -> Option<usize> {
    apply_kind(buffer, cold, TailKind::FrameCopies)
}

fn apply_kind(
    buffer: &mut MachBufferFinalized<Stencil>,
    cold: Range<CodeOffset>,
    kind: TailKind,
) -> Option<usize> {
    // Constant islands, exceptions and instruction-specific debug metadata need
    // a more general remapper. Keep those functions untouched. Apple unwind
    // records must all be in the unchanged warm prefix.
    if cold.start >= cold.end
        || cold.start % 4 != 0
        || cold.end % 4 != 0
        || cold.end as usize > buffer.data.len()
        || buffer.data.len() % 4 != 0
        || !buffer.patchable_call_sites.is_empty()
        || !buffer.exception_handlers.is_empty()
        || !buffer.debug_tags.is_empty()
        || !buffer.user_stack_maps.is_empty()
        || buffer.srclocs.iter().any(|loc| !loc.loc.is_default())
        || buffer
            .unwind_info
            .iter()
            .any(|&(offset, _)| offset >= cold.start)
    {
        return None;
    }
    let words: Vec<_> = buffer
        .data
        .chunks_exact(4)
        .map(|word| u32::from_le_bytes(word.try_into().unwrap()))
        .collect();
    let mut external = vec![false; words.len()];
    for reloc in &buffer.relocs {
        if !matches!(reloc.target, FinalizedRelocTarget::ExternalName(_)) || reloc.offset % 4 != 0 {
            return None;
        }
        let at = reloc.offset as usize / 4;
        if core::mem::replace(external.get_mut(at)?, true) {
            return None;
        }
        let word = words[at];
        if !matches!(reloc.kind, Reloc::Arm64Call if word & 0x7c00_0000 == B)
            && !matches!(reloc.kind, Reloc::Aarch64AdrGotPage21 | Reloc::Aarch64AdrPrelPgHi21
                if word & 0x9f00_0000 == 0x9000_0000)
            && !matches!(reloc.kind, Reloc::Aarch64Ld64GotLo12Nc
                if word & 0xffc0_0000 == 0xf940_0000)
            && !matches!(reloc.kind, Reloc::Aarch64AddAbsLo12Nc
                if word & 0xffc0_0000 == 0x9100_0000)
        {
            return None;
        }
    }
    let flows: Vec<_> = words
        .iter()
        .enumerate()
        .map(|(at, &word)| {
            if external[at] {
                Some(Flow::Barrier)
            } else {
                flow(word, at)
            }
        })
        .collect::<Option<_>>()?;
    let mut heads = vec![false; words.len()];
    heads[0] = true;
    for (at, &kind) in flows.iter().enumerate() {
        if let Flow::Branch(branch) = kind {
            *heads.get_mut(branch.target)? = true;
        }
        if !matches!(kind, Flow::Plain) && at + 1 < words.len() {
            heads[at + 1] = true;
        }
    }
    let mut protected = external;
    for trap in &buffer.traps {
        if trap.offset % 4 != 0 {
            return None;
        }
        *protected.get_mut(trap.offset as usize / 4)? = true;
    }
    // Finalization may append deferred traps after the last emitted block.
    // Accept only individually recorded terminal traps there, never a pool or
    // unclassified veneer. They remain distinct and retain their trap records.
    for at in cold.end as usize / 4..words.len() {
        if !matches!(flows[at], Flow::End)
            || !buffer
                .traps
                .iter()
                .any(|trap| trap.offset as usize == 4 * at)
        {
            return None;
        }
    }

    let substitutions = match kind {
        TailKind::Cold => plan_cold(&words, &flows, &heads, &protected, cold.start as usize / 4),
        TailKind::FrameCopies => {
            let frame_size = buffer.frame_layout.as_ref()?.frame_to_fp_offset;
            // Bound the frame copies below the caller frame and after any
            // prologue action. Every recorded unwind offset is verified again
            // after remapping and remains byte-for-byte unchanged.
            let first = buffer.unwind_info.iter().map(|&(offset, _)| offset).max()?;
            if first % 4 != 0 || frame_size == 0 {
                return None;
            }
            frame_copies::plan(&words, &flows, &heads, &protected, first as usize / 4, frame_size)
        }
    };

    let mut code = Vec::with_capacity(words.len());
    let mut moved = vec![None; words.len() + 1];
    let mut branches = Vec::new();
    let mut at = 0;
    while at < words.len() {
        moved[at] = Some(code.len());
        if let Some(replacement) = substitutions[at] {
            if replacement.emit_branch {
                branches.push((
                    code.len(),
                    Branch {
                        target: replacement.target,
                        shift: 0,
                        bits: 26,
                    },
                ));
                code.push(B);
            } else {
                moved[at] = None;
            }
            at += replacement.width;
        } else {
            if let Flow::Branch(branch) = flows[at] {
                branches.push((code.len(), branch));
            }
            code.push(words[at]);
            at += 1;
        }
    }
    moved[words.len()] = Some(code.len());
    let saved = (words.len() - code.len()) * 4;
    if saved == 0 {
        return Some(0);
    }
    for (at, branch) in branches {
        let target = match substitutions.get(branch.target).copied().flatten() {
            Some(replacement) if replacement.redirect_branches => replacement.target,
            _ => branch.target,
        };
        code[at] = branch.encode(code[at], at, moved.get(target).copied().flatten()?)?;
    }
    let remap = |offset: u32| -> Option<u32> {
        if offset % 4 != 0 {
            return None;
        }
        u32::try_from(4 * moved.get(offset as usize / 4).copied().flatten()?).ok()
    };
    if buffer
        .unwind_info
        .iter()
        .any(|&(offset, _)| remap(offset) != Some(offset))
    {
        return None;
    }
    let mut relocs = buffer.relocs.clone();
    for reloc in &mut relocs {
        reloc.offset = remap(reloc.offset)?;
    }
    let mut traps = buffer.traps.clone();
    for trap in &mut traps {
        trap.offset = remap(trap.offset)?;
    }
    let mut calls = buffer.call_sites.clone();
    for call in &mut calls {
        call.ret_addr = remap(call.ret_addr)?;
    }

    let end = u32::try_from(code.len() * 4).ok()?;
    // All validation succeeds before changing any field of the original buffer.
    buffer.relocs = relocs;
    buffer.traps = traps;
    buffer.call_sites = calls;
    buffer.srclocs.truncate(1);
    if let Some(loc) = buffer.srclocs.first_mut() {
        loc.start = 0;
        loc.end = end;
    }
    buffer.data = code.into_iter().flat_map(u32::to_le_bytes).collect();
    Some(saved)
}

fn plan_cold(
    words: &[u32],
    flows: &[Flow],
    heads: &[bool],
    protected: &[bool],
    first: usize,
) -> Vec<Option<Replacement>> {
    let mut groups: BTreeMap<(usize, Vec<u32>), Vec<usize>> = BTreeMap::new();
    for end in first..words.len() {
        let Flow::Branch(branch) = flows[end] else {
            continue;
        };
        if words[end] & 0xfc00_0000 != B || protected[end] {
            continue;
        }
        let mut start = end;
        while start > first && end - start + 1 < MAX_TAIL && !heads[start] {
            start -= 1;
            if protected[start] || !matches!(flows[start], Flow::Plain) {
                break;
            }
            groups
                .entry((branch.target, words[start..end].to_vec()))
                .or_default()
                .push(start);
        }
    }
    let mut groups: Vec<_> = groups
        .into_iter()
        .filter(|(_, sites)| sites.len() > 1)
        .collect();
    groups.sort_by_key(|((target, tail), sites)| {
        (
            core::cmp::Reverse((sites.len() - 1) * tail.len()),
            core::cmp::Reverse(tail.len()),
            *target,
            sites[0],
        )
    });
    let mut occupied = vec![false; words.len()];
    let mut substitutions = vec![None; words.len()];
    for ((_, tail), sites) in groups {
        let width = tail.len() + 1;
        let mut available = Vec::new();
        for start in sites {
            if !occupied[start..start + width].iter().any(|&used| used)
                && available
                    .last()
                    .is_none_or(|&previous| previous + width <= start)
            {
                available.push(start);
            }
        }
        if available.len() < 2 {
            continue;
        }
        for &start in &available {
            occupied[start..start + width].fill(true);
        }
        for &start in &available[1..] {
            substitutions[start] = Some(Replacement {
                target: available[0],
                width,
                emit_branch: true,
                redirect_branches: false,
            });
        }
    }

    substitutions
}

#[cfg(test)]
mod tests;
