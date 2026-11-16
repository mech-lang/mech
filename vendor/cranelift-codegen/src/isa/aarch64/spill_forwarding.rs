//! Reuse register values that still match a private register-allocation spill slot.

use super::inst::Inst;
use crate::machinst::VCode;
use alloc::{collections::VecDeque, vec, vec::Vec};
use regalloc2::{
    Allocation, Block, Edit, Function, OperandKind, Output, PReg, PRegSet, ProgPoint, RegClass,
};

const REGISTERS: usize = 32;

#[derive(Clone, Copy, Default)]
struct Facts([Option<usize>; REGISTERS]);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Rewrite {
    Keep,
    Omit,
    FromRegister(PReg),
}

impl Facts {
    fn forget_slots(&mut self, start: usize, size: usize) {
        for fact in &mut self.0 {
            if fact.is_some_and(|slot| start <= slot && slot < start + size) {
                *fact = None;
            }
        }
    }

    fn intersect(&mut self, other: Self) -> bool {
        let mut changed = false;
        for (fact, incoming) in self.0.iter_mut().zip(other.0) {
            if fact.is_some() && *fact != incoming {
                *fact = None;
                changed = true;
            }
        }
        changed
    }
}

struct Forwarding<'a> {
    code: &'a VCode<Inst>,
    output: &'a Output,
    registers: PRegSet,
}

impl Forwarding<'_> {
    fn register(&self, allocation: Allocation) -> Option<usize> {
        allocation
            .as_reg()
            .filter(|&reg| self.registers.contains(reg))
            .map(PReg::hw_enc)
    }

    fn apply_move(&self, facts: &mut Facts, edit: &Edit) -> Rewrite {
        let Edit::Move { from, to } = *edit;
        let source = self.register(from);
        if let Some(slot) = to.as_stack() {
            let slot = slot.index();
            if source.is_some_and(|reg| facts.0[reg] == Some(slot)) {
                return Rewrite::Omit;
            }
            let class = from.as_reg().expect("spill has a source register").class();
            facts.forget_slots(slot, self.code.spillslot_size(class));
            if let Some(reg) = source {
                facts.0[reg] = Some(slot);
            }
        } else if let Some(reg) = self.register(to) {
            if let Some(slot) = from.as_stack() {
                let slot = slot.index();
                if facts.0[reg] == Some(slot) {
                    return Rewrite::Omit;
                }
                let source = facts.0.iter().position(|&fact| fact == Some(slot));
                facts.0[reg] = Some(slot);
                if let Some(source) = source {
                    return Rewrite::FromRegister(PReg::new(source, RegClass::Int));
                }
            } else {
                let value = source.and_then(|source| facts.0[source]);
                if value.is_some() && facts.0[reg] == value {
                    return Rewrite::Omit;
                }
                facts.0[reg] = value;
            }
        }
        Rewrite::Keep
    }

    fn transfer(
        &self,
        block: Block,
        mut facts: Facts,
        mut rewrite: impl FnMut(usize, Rewrite),
    ) -> Facts {
        let instructions = self.code.block_insns(block);
        let first = ProgPoint::before(instructions.first());
        let mut next = self
            .output
            .edits
            .partition_point(|&(point, _)| point < first);
        for inst in instructions.iter() {
            let before = ProgPoint::before(inst);
            while let Some((point, edit)) = self.output.edits.get(next) {
                if *point > before {
                    break;
                }
                rewrite(next, self.apply_move(&mut facts, edit));
                next += 1;
            }
            for (operand, &alloc) in self
                .code
                .inst_operands(inst)
                .iter()
                .zip(self.output.inst_allocs(inst))
            {
                if operand.kind() == OperandKind::Def {
                    if let Some(reg) = self.register(alloc) {
                        facts.0[reg] = None;
                    } else if let Some(slot) = alloc.as_stack() {
                        facts.forget_slots(slot.index(), self.code.spillslot_size(operand.class()));
                    }
                }
            }
            for reg in self.code.inst_clobbers(inst) & self.registers {
                facts.0[reg.hw_enc()] = None;
            }
        }
        facts
    }
}

pub(super) fn forward(code: &VCode<Inst>, output: &mut Output) {
    // Only integer registers allocated from the machine's normal allocatable set
    // participate. The IP scratch registers may be written by instruction expansion.
    if output.num_spillslots == 0 || code.spillslot_size(RegClass::Int) != 1 {
        return;
    }
    let env = code.abi.machine_env();
    let mut registers = env.preferred_regs_by_class[RegClass::Int as usize];
    registers.union_from(env.non_preferred_regs_by_class[RegClass::Int as usize]);
    let pass = Forwarding {
        code,
        output,
        registers,
    };

    // These relations concern physical bits after edge moves, so at a join we
    // retain a relation only when every reached predecessor establishes it.
    let entry = code.entry_block();
    let mut incoming = vec![None; code.num_blocks()];
    incoming[entry.index()] = Some(Facts::default());
    let mut pending = VecDeque::from([entry]);
    let mut queued = vec![false; code.num_blocks()];
    queued[entry.index()] = true;
    while let Some(block) = pending.pop_front() {
        queued[block.index()] = false;
        let exit = pass.transfer(block, incoming[block.index()].unwrap(), |_, _| {});
        for &successor in code.block_succs(block) {
            let changed = match &mut incoming[successor.index()] {
                Some(facts) => facts.intersect(exit),
                old @ None => {
                    *old = Some(exit);
                    true
                }
            };
            if changed && !core::mem::replace(&mut queued[successor.index()], true) {
                pending.push_back(successor);
            }
        }
    }

    let mut rewrites = vec![Rewrite::Keep; output.edits.len()];
    for (index, facts) in incoming.into_iter().enumerate() {
        if let Some(facts) = facts {
            pass.transfer(Block::new(index), facts, |index, change| {
                rewrites[index] = change
            });
        }
    }
    output.edits = core::mem::take(&mut output.edits)
        .into_iter()
        .zip(rewrites)
        .filter_map(|((point, edit), rewrite)| match rewrite {
            Rewrite::Keep => Some((point, edit)),
            Rewrite::Omit => None,
            Rewrite::FromRegister(reg) => {
                let Edit::Move { to, .. } = edit;
                Some((
                    point,
                    Edit::Move {
                        from: Allocation::reg(reg),
                        to,
                    },
                ))
            }
        })
        .collect::<Vec<_>>();
    output.stats.edits_count = output.edits.len();
}
