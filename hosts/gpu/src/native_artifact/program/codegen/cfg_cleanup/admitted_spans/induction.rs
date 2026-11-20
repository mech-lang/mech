//! Prove the invariant cursor <= end. Each incoming cursor is the one fixed
//! initial word, a copy of the current cursor, or current + 1 computed on the
//! != end edge. The admission supplies initial <= end, so the increment cannot
//! wrap. Nested copy phis are allowed; unsupported cycles decline.
use super::super::bounded_byte_histogram::fixed_word;
use super::super::word_ranges::literal;
use cranelift_codegen::Context;
use cranelift_codegen::ir::{Block, Inst, InstructionData as Data, Opcode, Value, ValueDef, types};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Default)]
struct Origins {
    begin: Option<usize>,
    current: bool,
    next: bool,
    increments: BTreeSet<Inst>,
}
impl Origins {
    fn merge(&mut self, other: Self) -> Option<()> {
        if let Some(word) = other.begin {
            if self.begin.is_some_and(|old| old != word) {
                return None;
            }
            self.begin = Some(word);
        }
        self.current |= other.current;
        self.next |= other.next;
        self.increments.extend(other.increments);
        Some(())
    }
}

struct Proof<'a> {
    ctx: &'a Context,
    loads: &'a BTreeMap<Value, (Inst, usize)>,
    cursor: Value,
    active: BTreeSet<Value>,
    memo: BTreeMap<Value, Origins>,
    fuel: usize,
}
impl Proof<'_> {
    fn incoming(&self, block: Block, slot: usize) -> Option<Vec<Value>> {
        let f = &self.ctx.func;
        let mut values = Vec::new();
        for pred in self.ctx.cfg.pred_iter(block) {
            for call in f.dfg.insts[pred.inst]
                .branch_destination(&f.dfg.jump_tables, &f.dfg.exception_tables)
            {
                if call.block(&f.dfg.value_lists) == block {
                    values.push(call.args(&f.dfg.value_lists).nth(slot)?.as_value()?);
                }
            }
        }
        (!values.is_empty()).then_some(values)
    }
    fn value(&mut self, value: Value) -> Option<Origins> {
        self.fuel = self.fuel.checked_sub(1)?;
        let f = &self.ctx.func;
        let value = f.dfg.resolve_aliases(value);
        if value == self.cursor {
            return Some(Origins {
                current: true,
                ..Origins::default()
            });
        }
        if let Some(old) = self.memo.get(&value) {
            return Some(old.clone());
        }
        if f.dfg.value_type(value) != types::I64
            || self.active.len() >= 64
            || !self.active.insert(value)
        {
            return None;
        }
        let result = if let Some(word) = fixed_word(f, &self.ctx.cfg, self.loads, value) {
            Origins {
                begin: Some(word),
                ..Origins::default()
            }
        } else {
            match f.dfg.value_def(value) {
                ValueDef::Param(block, slot) => {
                    let mut result = Origins::default();
                    for value in self.incoming(block, slot)? {
                        result.merge(self.value(value)?)?;
                    }
                    result
                }
                ValueDef::Result(inst, 0) => {
                    let (base, amount) = match f.dfg.insts[inst] {
                        Data::BinaryImm64 {
                            opcode: Opcode::IaddImm,
                            arg,
                            imm,
                        } => (arg, imm.bits() as u64),
                        Data::Binary {
                            opcode: Opcode::Iadd,
                            args: [a, b],
                        } => {
                            if let Some(n) = literal(f, b) {
                                (a, n)
                            } else if let Some(n) = literal(f, a) {
                                (b, n)
                            } else {
                                return None;
                            }
                        }
                        _ => return None,
                    };
                    let mut result = self.value(base)?;
                    match amount {
                        0 => (),
                        1 if result.begin.is_none() && result.current && !result.next => {
                            result.current = false;
                            result.next = true;
                            result.increments.insert(inst);
                        }
                        _ => return None,
                    }
                    result
                }
                _ => return None,
            }
        };
        self.active.remove(&value);
        self.memo.insert(value, result.clone());
        Some(result)
    }
}

pub(super) fn prove(
    ctx: &Context,
    loads: &BTreeMap<Value, (Inst, usize)>,
    cursor: Value,
    body: Block,
) -> Option<usize> {
    let ValueDef::Param(header, slot) = ctx.func.dfg.value_def(cursor) else {
        return None;
    };
    let mut proof = Proof {
        ctx,
        loads,
        cursor,
        active: BTreeSet::new(),
        memo: BTreeMap::new(),
        fuel: 2048,
    };
    let mut origins = Origins::default();
    for value in proof.incoming(header, slot)? {
        origins.merge(proof.value(value)?)?;
    }
    if !origins.next
        || origins.increments.iter().any(|&inst| {
            ctx.func
                .layout
                .inst_block(inst)
                .is_none_or(|block| !ctx.domtree.block_dominates(body, block))
        })
    {
        return None;
    }
    origins.begin
}
