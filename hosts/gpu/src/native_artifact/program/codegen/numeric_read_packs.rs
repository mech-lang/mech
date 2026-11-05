//! Confined source byte expressions share one guarded native word load.
//! This is an emission choice, not another numeric representation or proof.
use crate::native_artifact::codegen::common::ssa_words::Words;
use crate::native_artifact::program::prepare::readonly_numeric::{Operation, Proof};
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{Block, Endianness, InstBuilder, MemFlags, types};
use cranelift_frontend::FunctionBuilder;
use mech_engine::native_artifact::{
    NativeIterationAction as Action, NativeIterationTerminator as Term,
    NativeMatchAction as Scalar, NativeMatchTerminator as Branch, NativeMatchValue,
    NativeProgramFacts, NativeProgramReadRole as Read, NativeScalarOp as Op,
};
use std::collections::{BTreeMap, BTreeSet};
#[path = "numeric_pack_window.rs"]
mod window;

pub(super) struct Pack {
    pub first: usize,
    pub last: usize,
    pub next: u32,
    pub width: u8,
    pub source_window: bool,
    array: usize,
    base: NativeMatchValue,
    offset: u64,
    end: u64,
    output: NativeMatchValue,
    sites: BTreeSet<usize>,
}

struct Finder<'a, 'p> {
    facts: &'a NativeProgramFacts<'p>,
    proof: &'a Proof,
    constants: BTreeMap<u32, u64>,
    sites: BTreeSet<usize>,
    boundaries: BTreeSet<usize>,
}
impl Finder<'_, '_> {
    fn definition(&self, value: NativeMatchValue) -> Option<usize> {
        let [definition] = self.facts.definition_sites(value.get()).ok()? else {
            return None;
        };
        let site = definition.site()? as usize;
        self.sites.contains(&site).then_some(site)
    }

    // Resolve checked additions only. A whole-window guard bounded by logical
    // length proves every nonnegative partial sum, not just a wrapped endpoint.
    fn index(
        &self,
        mut value: NativeMatchValue,
        sites: &mut BTreeSet<usize>,
    ) -> Option<(NativeMatchValue, u64)> {
        let mut offset = 0u64;
        let mut seen = BTreeSet::new();
        loop {
            if !seen.insert(value.get()) {
                return None;
            }
            let Some(site) = self.definition(value) else {
                return Some((value, offset));
            };
            let Action::Scalar(Scalar::Primitive {
                op: Op::U64Add,
                operands,
                ..
            }) = self.facts.plan().blocks()[site].action()
            else {
                return Some((value, offset));
            };
            let [left, right] = operands.as_ref() else {
                return None;
            };
            let (base, amount) = if let Some(n) = self.constants.get(&right.get()) {
                (*left, *n)
            } else if let Some(n) = self.constants.get(&left.get()) {
                (*right, *n)
            } else {
                return Some((value, offset));
            };
            offset = offset.checked_add(amount)?;
            sites.insert(site);
            value = base;
        }
    }

    fn pack(&self, root: NativeMatchValue) -> Option<Pack> {
        let last = self.definition(root)?;
        let mut sites = BTreeSet::new();
        let mut pending = vec![(root, 0u64)];
        let mut seen = BTreeSet::new();
        let mut bytes = Vec::new();
        while let Some((value, shift)) = pending.pop() {
            if !seen.insert(value.get()) {
                return None;
            }
            let site = self.definition(value)?;
            sites.insert(site);
            match self.facts.plan().blocks()[site].action() {
                Action::Scalar(Scalar::Copy { source, .. }) => pending.push((*source, shift)),
                Action::Scalar(Scalar::Primitive {
                    op: Op::U64BitOr,
                    operands,
                    ..
                }) => {
                    let [left, right] = operands.as_ref() else {
                        return None;
                    };
                    pending.extend([(*left, shift), (*right, shift)]);
                }
                Action::Scalar(Scalar::Primitive {
                    op: Op::U64ShlWrap,
                    operands,
                    ..
                }) => {
                    let [word, amount] = operands.as_ref() else {
                        return None;
                    };
                    let amount = *self.constants.get(&amount.get())?;
                    // Never interpret wrapping shifts as rotations or sum
                    // their counts modulo64. Each byte must survive exactly.
                    let shift = shift.checked_add(amount)?;
                    if amount >= 64 || shift > 56 {
                        return None;
                    }
                    pending.push((*word, shift));
                }
                Action::Scalar(Scalar::Primitive {
                    op: Op::U8ToU64,
                    operands,
                    ..
                }) => {
                    let [byte] = operands.as_ref() else {
                        return None;
                    };
                    let read = self.definition(*byte)?;
                    let Operation::Read {
                        array,
                        index,
                        output,
                    } = self.proof.action(read)?
                    else {
                        return None;
                    };
                    if *output != byte.get() || self.proof.arrays.get(*array)?.width != 1 {
                        return None;
                    }
                    let Action::LibraryCall { operands, .. } =
                        self.facts.plan().blocks()[read].action()
                    else {
                        return None;
                    };
                    let index_value = *operands.get(1)?;
                    if index_value.get() != *index {
                        return None;
                    }
                    sites.insert(read);
                    let (base, offset) = self.index(index_value, &mut sites)?;
                    bytes.push((shift, *array, base, offset));
                    if bytes.len() > 8 {
                        return None;
                    }
                }
                _ => return None,
            }
        }
        let width = u8::try_from(bytes.len()).ok()?;
        if !matches!(width, 2 | 4 | 8) {
            return None;
        }
        bytes.sort_by_key(|b| b.0);
        let (_, array, base, offset) = bytes[0];
        for (i, &(shift, current_array, current_base, current_offset)) in bytes.iter().enumerate() {
            if shift != (i * 8) as u64
                || current_array != array
                || current_base != base
                || current_offset != offset.checked_add(i as u64)?
            {
                return None;
            }
        }
        let end = offset.checked_add(u64::from(width) - 1)?;
        let first = *sites.first()?;
        let iteration = self.facts.plan().blocks()[first].iteration();
        let mut walked = BTreeSet::new();
        let mut current = first;
        let mut previous = None;
        let next =
            loop {
                if !sites.contains(&current)
                    || !walked.insert(current)
                    || self.boundaries.contains(&current)
                {
                    return None;
                }
                let block = &self.facts.plan().blocks()[current];
                if block.iteration() != iteration {
                    return None;
                }
                if let Some(previous) = previous {
                    if self.facts.predecessors(current as u32).ok()? != [previous] {
                        return None;
                    }
                }
                let output = match block.action() {
                    Action::Scalar(
                        Scalar::Copy { output, .. } | Scalar::Primitive { output, .. },
                    )
                    | Action::LibraryCall { output, .. } => *output,
                    _ => return None,
                };
                // The current base must already exist when the first action runs.
                // Dead intermediate scalar homes need no speculative definitions.
                if output == base {
                    return None;
                }
                if output != root && self.facts.read_roles(output).ok()?.iter().any(|role| {
                !matches!(role, Read::Action(site) if sites.contains(&(*site as usize)))
            }) { return None; }
                let Term::Scalar(Branch::Jump(next)) = block.terminator() else {
                    return None;
                };
                if current == last {
                    break next;
                }
                previous = Some(current as u32);
                current = next as usize;
            };
        if walked != sites || sites.contains(&(next as usize)) {
            return None;
        }
        if self.proof.exits.iter().any(|exit| {
            sites.contains(&exit.from) || (exit.to != first && sites.contains(&exit.to))
        }) {
            return None;
        }
        let mut pack = Pack {
            first,
            last,
            next,
            width,
            source_window: false,
            array,
            base,
            offset,
            end,
            output: root,
            sites,
        };
        pack.source_window = window::derive(self, &pack);
        Some(pack)
    }
}

pub(super) fn derive(facts: &NativeProgramFacts<'_>, proof: &Proof) -> Vec<Pack> {
    let mut boundaries = BTreeSet::new();
    if let Some(phase) = &proof.phase {
        for debit in &phase.loops {
            boundaries.extend([
                debit.header,
                debit.initialize,
                debit.latch,
                debit.finish,
                debit.exit,
            ]);
        }
    }
    // All source loop entries/latches/finishes remain emitted, including an
    // interval's currently ordinary/funded transition outside the phase list.
    for iteration in facts.plan().iterations() {
        if let mech_engine::native_artifact::NativeControlLoop::Tail(tail) = iteration {
            boundaries.extend(
                [
                    tail.header(),
                    tail.initialize(),
                    tail.latch(),
                    tail.finish(),
                    tail.exit(),
                ]
                .map(|s| s as usize),
            );
        }
    }
    let finder = Finder {
        facts,
        proof,
        boundaries,
        sites: proof.sites.iter().copied().collect(),
        constants: facts
            .plan()
            .constants()
            .iter()
            .filter_map(|constant| {
                if let mech_core::ValueData::U64(word) = constant.constant().data() {
                    Some((constant.value().get(), *word))
                } else {
                    None
                }
            })
            .collect(),
    };
    let mut covered = BTreeSet::new();
    let mut packs = Vec::new();
    // Prefer complete outer expressions to their adjacent smaller subpacks.
    for &site in proof.sites.iter().rev() {
        if covered.contains(&site) {
            continue;
        }
        let Action::Scalar(Scalar::Primitive {
            op: Op::U64BitOr,
            output,
            ..
        }) = facts.plan().blocks()[site].action()
        else {
            continue;
        };
        if let Some(pack) = finder.pack(*output)
            && pack.sites.is_disjoint(&covered)
        {
            covered.extend(pack.sites.iter().copied());
            packs.push(pack);
        }
    }
    packs
}

pub(super) fn emit(
    b: &mut FunctionBuilder<'_>,
    words: &Words,
    shadow: usize,
    proof: &Proof,
    pack: &Pack,
    next: Block,
) {
    let view = shadow + proof.view(pack.array);
    let rows = words.read(b, view + 2);
    let columns = words.read(b, view + 3);
    let base = words.read(b, pack.base.get() as usize);
    let row = b.ins().icmp_imm(IntCC::Equal, rows, 1);
    let column = b.ins().icmp_imm(IntCC::Equal, columns, 1);
    let vector = b.ins().bor(row, column);
    // base+end<=logical length without overflowing either the source's
    // checked additions or this guard. Spare allocation capacity is irrelevant.
    let mut valid = vector;
    if !pack.source_window {
        let length = words.read(b, view + 1);
        let end = b.ins().iconst(types::I64, pack.end as i64);
        let enough = b.ins().icmp(IntCC::UnsignedGreaterThanOrEqual, length, end);
        let available = b.ins().isub(length, end);
        let fits = b
            .ins()
            .icmp(IntCC::UnsignedLessThanOrEqual, base, available);
        valid = b.ins().band(valid, enough);
        valid = b.ins().band(valid, fits);
        if pack.offset == 0 {
            let nonzero = b.ins().icmp_imm(IntCC::NotEqual, base, 0);
            valid = b.ins().band(valid, nonzero);
        }
    }
    let fast = b.create_block();
    let original = b.create_block();
    b.ins().brif(valid, fast, &[], original, &[]);
    b.switch_to_block(fast);
    let ordinal = b.ins().iadd_imm(base, pack.offset.wrapping_sub(1) as i64);
    let pointer = words.read(b, view);
    let address = b.ins().iadd(pointer, ordinal);
    let ty = match pack.width {
        2 => types::I16,
        4 => types::I32,
        8 => types::I64,
        _ => unreachable!(),
    };
    let value = b.ins().load(
        ty,
        MemFlags::new().with_endianness(Endianness::Little),
        address,
        0,
    );
    let value = if pack.width < 8 {
        b.ins().uextend(types::I64, value)
    } else {
        value
    };
    words.write(b, pack.output.get() as usize, value);
    b.ins().jump(next, &[]);
    b.switch_to_block(original);
}
