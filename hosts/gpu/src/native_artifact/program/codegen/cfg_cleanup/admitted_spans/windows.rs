//! On an admitted cursor, cursor + min(length - cursor, cap) cannot wrap or
//! exceed length. Prove each clamped incoming value on its actual CFG edge.
use super::super::word_ranges::{constant_offset, literal};
use super::{comparison, fixed_word, offset, sole_predecessor};
use cranelift_codegen::Context;
use cranelift_codegen::ir::condcodes::{CondCode, IntCC};
use cranelift_codegen::ir::{Block, Inst, InstructionData as Data, Opcode, Value, ValueDef, types};
use std::collections::{BTreeMap, BTreeSet};

struct Proof<'a> {
    ctx: &'a Context,
    loads: &'a BTreeMap<Value, (Inst, usize)>,
    cursor: Value,
    body: Block,
    length_word: usize,
    fuel: usize,
    active: BTreeSet<(Value, Block)>,
}
impl Proof<'_> {
    fn same_cursor(&mut self, value: Value) -> bool {
        let f = &self.ctx.func;
        if offset(
            f,
            &self.ctx.cfg,
            value,
            self.cursor,
            &mut BTreeSet::new(),
            &mut self.fuel,
        ) == Some(0)
        {
            return true;
        }
        let ValueDef::Result(inst, 0) = f.dfg.value_def(f.dfg.resolve_aliases(value)) else {
            return false;
        };
        let Data::Binary {
            opcode: Opcode::Iadd,
            args: [a, b],
        } = f.dfg.insts[inst]
        else {
            return false;
        };
        // x + (cursor - x) is cursor modulo the word size, even when either
        // intermediate wraps. No range assumption about x is required.
        [(a, b), (b, a)].into_iter().any(|(base, delta)| {
            let ValueDef::Result(inst, 0) = f.dfg.value_def(f.dfg.resolve_aliases(delta)) else {
                return false;
            };
            matches!(f.dfg.insts[inst], Data::Binary { opcode: Opcode::Isub, args: [at, sub] }
                if constant_offset(f, base) == constant_offset(f, sub)
                    && offset(f, &self.ctx.cfg, at, self.cursor, &mut BTreeSet::new(), &mut self.fuel) == Some(0))
        })
    }
    fn length(&self, value: Value) -> bool {
        fixed_word(&self.ctx.func, &self.ctx.cfg, self.loads, value) == Some(self.length_word)
    }
    fn remaining(&mut self, value: Value) -> bool {
        let f = &self.ctx.func;
        let ValueDef::Result(inst, 0) = f.dfg.value_def(f.dfg.resolve_aliases(value)) else {
            return false;
        };
        matches!(f.dfg.insts[inst], Data::Binary { opcode: Opcode::Isub, args: [length, at] }
            if self.length(length) && self.same_cursor(at))
    }
    fn edge_bound(&mut self, candidate: Value, mut block: Block) -> bool {
        let f = &self.ctx.func;
        let mut visited = BTreeSet::new();
        while self.fuel != 0 && visited.len() < 64 && visited.insert(block) {
            self.fuel -= 1;
            let Some((prior, inst)) = sole_predecessor(&self.ctx.cfg, block) else {
                return false;
            };
            if let Data::Brif { arg, blocks, .. } = f.dfg.insts[inst] {
                let edge = blocks.map(|call| call.block(&f.dfg.value_lists) == block);
                if edge[0] == edge[1] {
                    return false;
                }
                let Some(test) = comparison(f, arg) else {
                    return false;
                };
                let normalized = match f.dfg.insts[test] {
                    Data::IntCompare {
                        cond, args: [a, b], ..
                    } => {
                        if constant_offset(f, a) == constant_offset(f, candidate)
                            && self.remaining(b)
                        {
                            Some(cond)
                        } else if constant_offset(f, b) == constant_offset(f, candidate)
                            && self.remaining(a)
                        {
                            Some(cond.swap_args())
                        } else {
                            None
                        }
                    }
                    Data::IntCompareImm { cond, arg, imm, .. }
                        if literal(f, candidate) == Some(imm.bits() as u64)
                            && self.remaining(arg) =>
                    {
                        Some(cond.swap_args())
                    }
                    _ => None,
                };
                if normalized.is_some_and(|cond| match (edge[0], cond) {
                    (
                        true,
                        IntCC::UnsignedLessThan | IntCC::UnsignedLessThanOrEqual | IntCC::Equal,
                    )
                    | (false, IntCC::UnsignedGreaterThan | IntCC::UnsignedGreaterThanOrEqual) => {
                        true
                    }
                    _ => false,
                }) {
                    return true;
                }
            } else if !matches!(f.dfg.insts[inst], Data::Jump { .. }) {
                return false;
            }
            block = prior;
        }
        false
    }
    fn bounded(&mut self, value: Value, block: Block) -> bool {
        if self.fuel == 0 || self.active.len() >= 64 {
            return false;
        }
        self.fuel -= 1;
        let f = &self.ctx.func;
        let value = f.dfg.resolve_aliases(value);
        if f.dfg.value_type(value) != types::I64 {
            return false;
        }
        if self.remaining(value) || self.edge_bound(value, block) {
            return true;
        }
        if !self.active.insert((value, block)) {
            return false;
        }
        let result = if let ValueDef::Param(owner, slot) = f.dfg.value_def(value) {
            // A clamp join must execute inside this admitted cursor's body.
            // Do not carry an earlier iteration's remaining length through
            // the cursor header and compare it with the new cursor.
            if !self.ctx.domtree.block_dominates(self.body, owner) {
                return false;
            }
            let mut incoming = Vec::new();
            for pred in self.ctx.cfg.pred_iter(owner) {
                for call in f.dfg.insts[pred.inst]
                    .branch_destination(&f.dfg.jump_tables, &f.dfg.exception_tables)
                {
                    if call.block(&f.dfg.value_lists) == owner {
                        let Some(value) = call
                            .args(&f.dfg.value_lists)
                            .nth(slot)
                            .and_then(|a| a.as_value())
                        else {
                            return false;
                        };
                        incoming.push((value, pred.block));
                    }
                }
            }
            !incoming.is_empty() && incoming.into_iter().all(|(v, b)| self.bounded(v, b))
        } else {
            false
        };
        self.active.remove(&(value, block));
        result
    }
    fn end(&mut self, value: Value, block: Block) -> bool {
        let f = &self.ctx.func;
        let ValueDef::Result(inst, 0) = f.dfg.value_def(f.dfg.resolve_aliases(value)) else {
            return false;
        };
        let Data::Binary {
            opcode: Opcode::Iadd,
            args: [a, b],
        } = f.dfg.insts[inst]
        else {
            return false;
        };
        [(a, b), (b, a)]
            .into_iter()
            .any(|(at, width)| self.same_cursor(at) && self.bounded(width, block))
    }
}

pub(super) fn checks(
    ctx: &Context,
    loads: &BTreeMap<Value, (Inst, usize)>,
    body: Block,
    cursor: Value,
    length_word: usize,
) -> Vec<(Inst, i64)> {
    let f = &ctx.func;
    let mut proof = Proof {
        ctx,
        loads,
        cursor,
        body,
        length_word,
        fuel: 4096,
        active: BTreeSet::new(),
    };
    let mut changes = Vec::new();
    for block in f.layout.blocks() {
        if !ctx.domtree.block_dominates(body, block) {
            continue;
        }
        for inst in f.layout.block_insts(block) {
            let Data::IntCompare {
                cond, args: [a, b], ..
            } = f.dfg.insts[inst]
            else {
                continue;
            };
            for (end, other, condition) in [(a, b, cond), (b, a, cond.swap_args())] {
                let value = if proof.length(other) {
                    match condition {
                        IntCC::UnsignedLessThanOrEqual => Some(1),
                        IntCC::UnsignedGreaterThan => Some(0),
                        _ => None,
                    }
                } else if proof.same_cursor(other) {
                    match condition {
                        IntCC::UnsignedGreaterThanOrEqual => Some(1),
                        IntCC::UnsignedLessThan => Some(0),
                        _ => None,
                    }
                } else {
                    None
                };
                if let Some(value) = value.filter(|_| proof.end(end, block)) {
                    changes.push((inst, value));
                    break;
                }
            }
        }
    }
    changes
}
