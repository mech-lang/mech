//! Reorder the blocks of an emitted leaf according to measured branch use.

use crate::binemit::Reloc;
use crate::machinst::{FinalizedRelocTarget, MachBufferFinalized, Stencil};
use alloc::{vec, vec::Vec};

const WORD_BYTES: usize = 4;
const B: u32 = 0x1400_0000;

#[derive(Clone, Copy)]
struct Branch {
    target: usize,
    shift: u32,
    bits: u32,
}

impl Branch {
    fn with_target(self, word: u32, source: usize, target: usize) -> Option<u32> {
        let relative = i64::try_from(target).ok()? - i64::try_from(source).ok()?;
        let bound = 1_i64 << (self.bits - 1);
        if !(-bound..bound).contains(&relative) {
            return None;
        }
        let mask = (1_u32 << self.bits) - 1;
        Some((word & !(mask << self.shift)) | (((relative as u32) & mask) << self.shift))
    }

    fn invert(self, word: u32) -> Option<u32> {
        if self.bits == 19 && word & 0xff00_0010 == 0x5400_0000 {
            // AL and NV do not form a reversible condition pair.
            (word & 15 < 14).then_some(word ^ 1)
        } else {
            Some(word ^ (1 << 24))
        }
    }
}

#[derive(Clone, Copy)]
enum Control {
    Next,
    End,
    Branch(Branch),
}

fn control(word: u32, at: usize) -> Option<Control> {
    let field = if word & 0xfc00_0000 == B {
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
        let target = at.checked_add_signed(displacement as isize)?;
        return Some(Control::Branch(Branch {
            target,
            shift,
            bits,
        }));
    }
    let returns = word & 0xffff_fc1f == 0xd65f_0000 || matches!(word, 0xd65f_0bff | 0xd65f_0fff);
    let traps = word & 0xffe0_001f == 0xd420_0000 || word & 0xffff_0000 == 0;
    if returns || traps {
        return Some(Control::End);
    }
    // A relative data reference, unknown branch variant or call may use the
    // old location after reordering. Only copy position-independent words.
    if word & 0x1f00_0000 == 0x1000_0000
        || word & 0x3b00_0000 == 0x1800_0000
        || word & 0xfc00_0000 == 0x9400_0000
        || word & 0xff00_0000 == 0x5400_0000
        || word & 0xfe00_0000 == 0xd600_0000
    {
        return None;
    }
    Some(Control::Next)
}

fn add_branch(code: &mut Vec<u32>, pending: &mut Vec<(usize, Branch)>, word: u32, branch: Branch) {
    pending.push((code.len(), branch));
    code.push(word);
}

fn add_jump(code: &mut Vec<u32>, pending: &mut Vec<(usize, Branch)>, target: usize) {
    add_branch(
        code,
        pending,
        B,
        Branch {
            target,
            shift: 0,
            bits: 26,
        },
    );
}

pub(super) fn apply(
    buffer: &mut MachBufferFinalized<Stencil>,
    layout: &[u32],
    literal_start: Option<u32>,
    thread_jumps: bool,
) -> Option<()> {
    if buffer.data.len() % WORD_BYTES != 0
        || !buffer.call_sites.is_empty()
        || !buffer.patchable_call_sites.is_empty()
        || !buffer.exception_handlers.is_empty()
        || !buffer.debug_tags.is_empty()
        || !buffer.user_stack_maps.is_empty()
        || buffer
            .srclocs
            .iter()
            .any(|location| !location.loc.is_default())
    {
        return None;
    }
    let code_end = literal_start.map_or(Some(buffer.data.len()), |offset| {
        let offset = usize::try_from(offset).ok()?;
        // Preserving this alignment preserves every 4/8/16-byte pool entry's
        // original alignment when the entire trailing pool moves together.
        (offset > 0 && offset % 16 == 0 && offset < buffer.data.len()).then_some(offset)
    })?;
    let mut original: Vec<_> = buffer
        .data
        .get(..code_end)?
        .chunks_exact(WORD_BYTES)
        .map(|bytes| u32::from_le_bytes(bytes.try_into().unwrap()))
        .collect();
    if literal_start.is_some() {
        // A terminal return can be followed by alignment NOPs before its
        // literal pool. Drop only that suffix; any branch into it is rejected
        // when all branch destinations are validated against instruction heads.
        while original.last() == Some(&0xd503_201f) {
            original.pop();
        }
    }
    if original.is_empty() {
        return None;
    }
    let mut external = vec![false; original.len()];
    for reloc in &buffer.relocs {
        if !matches!(reloc.target, FinalizedRelocTarget::ExternalName(_)) {
            return None;
        }
        let at = usize::try_from(reloc.offset).ok()?;
        if at % WORD_BYTES != 0 || core::mem::replace(external.get_mut(at / WORD_BYTES)?, true) {
            return None;
        }
        let word = original[at / WORD_BYTES];
        // External page/offset relocations remain owned by the linker. Only
        // their instruction positions move; their targets/addends stay exact.
        if !matches!(reloc.kind,
            Reloc::Aarch64AdrGotPage21 | Reloc::Aarch64AdrPrelPgHi21
                if word & 0x9f00_0000 == 0x9000_0000)
            && !matches!(reloc.kind,
                Reloc::Aarch64Ld64GotLo12Nc if word & 0xffc0_0000 == 0xf940_0000)
            && !matches!(reloc.kind,
                Reloc::Aarch64AddAbsLo12Nc if word & 0xffc0_0000 == 0x9100_0000)
        {
            return None;
        }
    }
    let mut literals = Vec::new();
    let flow: Vec<_> = original
        .iter()
        .enumerate()
        .map(|(index, &word)| {
            if external[index] {
                return Some(Control::Next);
            }
            if word & 0x3b00_0000 == 0x1800_0000 {
                literal_start?;
                let width = match ((word >> 26) & 1, word >> 30) {
                    (0 | 1, 0) | (0, 2) => 4,
                    (0 | 1, 1) => 8,
                    (1, 2) => 16,
                    _ => return None, // PRFM and unallocated encodings.
                };
                let displacement = (((word >> 5) << 13) as i32) >> 13;
                let target = index.checked_add_signed(displacement as isize)?;
                let byte = target.checked_mul(WORD_BYTES)?;
                if byte < code_end || byte.checked_add(width)? > buffer.data.len() {
                    return None;
                }
                literals.push((index, target));
                return Some(Control::Next);
            }
            control(word, index)
        })
        .collect::<Option<_>>()?;
    if matches!(
        flow.last()?,
        Control::Next | Control::Branch(Branch { bits: 14 | 19, .. })
    ) {
        return None;
    }

    let mut heads = vec![false; original.len()];
    heads[0] = true;
    for (at, &entry) in flow.iter().enumerate() {
        if let Control::Branch(branch) = entry {
            *heads.get_mut(branch.target)? = true;
        }
        if !matches!(entry, Control::Next) && at + 1 < original.len() {
            heads[at + 1] = true;
        }
    }
    let mut ends = vec![0; original.len()];
    let mut head = 0;
    for (at, &boundary) in heads.iter().enumerate().skip(1) {
        if boundary {
            ends[head] = at;
            head = at;
        }
    }
    ends[head] = original.len();
    let mut order = Vec::with_capacity(layout.len());
    for &byte in layout {
        let byte = usize::try_from(byte).ok()?;
        if byte % WORD_BYTES != 0 || !core::mem::take(heads.get_mut(byte / WORD_BYTES)?) {
            return None;
        }
        order.push(byte / WORD_BYTES);
    }
    // Prologue unwind instructions must remain attached to their original
    // positions in the entry block. Other offset-bearing metadata is handled below.
    if order.first() != Some(&0)
        || heads.into_iter().any(|unlisted| unlisted)
        || buffer
            .unwind_info
            .iter()
            .any(|&(offset, _)| offset as usize > WORD_BYTES * (ends[0] - 1))
    {
        return None;
    }

    // The validated leaf has no indirect transfers or interior code
    // references. A non-entry block containing only B can therefore be
    // bypassed by rewriting its taken and fallthrough predecessors.
    let jump_targets = thread_jumps.then(|| {
        let only_jump = |begin: usize| {
            (ends[begin] == begin + 1)
                .then_some(flow[begin])
                .and_then(|control| match control {
                    Control::Branch(branch) if branch.bits == 26 => Some(branch.target),
                    _ => None,
                })
        };
        let mut targets: Vec<_> = (0..original.len()).collect();
        for &begin in &order[1..] {
            if let Some(target) = only_jump(begin)
                && only_jump(target).is_none()
            {
                // Resolve one hop. Keeping jumps into jump-only blocks
                // avoids erasing an unconditional cycle.
                targets[begin] = target;
            }
        }
        order.retain(|&begin| targets[begin] == begin);
        targets
    });
    let target = |at| jump_targets.as_ref().map_or(at, |targets| targets[at]);

    let mut code = Vec::with_capacity(original.len());
    let mut targets = vec![0; original.len()];
    let mut copied = vec![None; original.len()];
    let mut branches = Vec::new();
    for (index, &begin) in order.iter().enumerate() {
        let following = order.get(index + 1).copied();
        let end = ends[begin];
        let last = end - 1;
        targets[begin] = code.len();
        for at in begin..last {
            copied[at] = Some(code.len());
            code.push(original[at]);
        }
        match flow[last] {
            Control::Next | Control::End => {
                copied[last] = Some(code.len());
                code.push(original[last]);
                if matches!(flow[last], Control::Next) && following != Some(target(end)) {
                    add_jump(&mut code, &mut branches, target(end));
                }
            }
            Control::Branch(branch) if branch.bits == 26 => {
                let taken = target(branch.target);
                if following != Some(taken) {
                    add_jump(&mut code, &mut branches, taken);
                }
            }
            Control::Branch(mut branch) => {
                let fallthrough = target(end);
                branch.target = target(branch.target);
                if branch.target == fallthrough {
                    if following != Some(fallthrough) {
                        add_jump(&mut code, &mut branches, fallthrough);
                    }
                } else if following == Some(branch.target) {
                    branch.target = fallthrough;
                    add_branch(
                        &mut code,
                        &mut branches,
                        branch.invert(original[last])?,
                        branch,
                    );
                } else {
                    add_branch(&mut code, &mut branches, original[last], branch);
                    if following != Some(fallthrough) {
                        add_jump(&mut code, &mut branches, fallthrough);
                    }
                }
            }
        }
    }
    for (at, branch) in branches {
        code[at] = branch.with_target(code[at], at, *targets.get(branch.target)?)?;
    }
    if literal_start.is_some() {
        while code.len() % 4 != 0 {
            code.push(0xd503_201f); // NOP padding before the aligned literal pool.
        }
        let pool = code.len();
        for (old, target) in literals {
            let at = copied.get(old).copied().flatten()?;
            let target = pool.checked_add(target.checked_sub(code_end / WORD_BYTES)?)?;
            code[at] = Branch {
                target,
                shift: 5,
                bits: 19,
            }
            .with_target(code[at], at, target)?;
        }
        code.extend(
            buffer.data[code_end..]
                .chunks_exact(WORD_BYTES)
                .map(|bytes| u32::from_le_bytes(bytes.try_into().unwrap())),
        );
    }
    let mut relocs = buffer.relocs.clone();
    for reloc in &mut relocs {
        let old = usize::try_from(reloc.offset).ok()? / WORD_BYTES;
        reloc.offset = u32::try_from(WORD_BYTES * copied.get(old).copied().flatten()?).ok()?;
    }
    relocs.sort_by_key(|reloc| reloc.offset);
    let mut traps = buffer.traps.clone();
    for trap in &mut traps {
        let offset = trap.offset as usize;
        if offset % WORD_BYTES != 0 {
            return None;
        }
        trap.offset =
            u32::try_from(WORD_BYTES * copied.get(offset / WORD_BYTES).copied().flatten()?).ok()?;
    }
    traps.sort_by_key(|trap| trap.offset);
    let end = u32::try_from(code.len().checked_mul(WORD_BYTES)?).ok()?;
    buffer.relocs = relocs;
    buffer.traps = traps;
    buffer.srclocs.truncate(1);
    if let Some(location) = buffer.srclocs.first_mut() {
        location.start = 0;
        location.end = end;
    }
    buffer.data = code.into_iter().flat_map(u32::to_le_bytes).collect();
    Some(())
}

#[cfg(test)]
mod tests {
    use super::apply;
    use crate::isa::aarch64::inst::Inst;
    use crate::machinst::{MachBuffer, VCodeConstants};
    use alloc::vec::Vec;
    use cranelift_control::ControlPlane;

    #[test]
    fn hot_loop_backedge_bypasses_empty_jump_block() {
        let mut buffer = MachBuffer::<Inst>::new();
        let instructions = [0x1100_0400, 0x5400_0040, 0x17ff_fffe, 0xd65f_03c0];
        for word in instructions {
            buffer.put4(word);
        }
        let mut buffer = buffer.finish(&VCodeConstants::default(), &mut ControlPlane::default());
        // Place the return next to the conditional. The continuing edge
        // points directly to the loop header and the return falls through.
        assert_eq!(apply(&mut buffer, &[0, 12, 8], None, true), Some(()));
        let expected: Vec<_> = [instructions[0], 0x54ff_ffe1, instructions[3]]
            .into_iter()
            .flat_map(u32::to_le_bytes)
            .collect();
        assert_eq!(buffer.data(), expected);
    }

    #[test]
    fn unconditional_cycle_keeps_its_behavior() {
        let mut buffer = MachBuffer::<Inst>::new();
        for word in [0x1400_0001, 0x1400_0001, 0x17ff_ffff] {
            buffer.put4(word);
        }
        let mut buffer = buffer.finish(&VCodeConstants::default(), &mut ControlPlane::default());
        assert_eq!(apply(&mut buffer, &[0, 4, 8], None, true), Some(()));
        assert_eq!(buffer.data(), 0x1400_0000u32.to_le_bytes());
    }
}
