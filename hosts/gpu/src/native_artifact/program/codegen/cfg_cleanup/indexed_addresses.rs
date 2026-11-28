//! Put an affine address bias on the base, outside the scaled index.
use super::word_ranges::{Facts, constant_offset, literal};
use cranelift_codegen::cursor::{Cursor, FuncCursor};
use cranelift_codegen::flowgraph::ControlFlowGraph;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{
    Function, Inst, InstBuilder, InstructionData as Data, Opcode as Op, Value, ValueDef, types,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
struct Sum {
    terms: BTreeMap<Value, u64>,
    bias: u64,
}

impl Sum {
    fn collect(
        &mut self,
        f: &Function,
        value: Value,
        scale: u64,
        budget: &mut usize,
    ) -> Option<()> {
        if *budget == 0 || f.dfg.value_type(value) != types::I64 {
            return None;
        }
        *budget -= 1;
        let value = f.dfg.resolve_aliases(value);
        if let Some(n) = literal(f, value) {
            self.bias = self.bias.wrapping_add(n.wrapping_mul(scale));
            return Some(());
        }
        if let ValueDef::Result(inst, 0) = f.dfg.value_def(value) {
            match f.dfg.insts[inst] {
                Data::Binary {
                    opcode: Op::Iadd,
                    args: [a, b],
                } => {
                    self.collect(f, a, scale, budget)?;
                    return self.collect(f, b, scale, budget);
                }
                Data::Binary {
                    opcode: Op::Isub,
                    args: [a, b],
                } => {
                    self.collect(f, a, scale, budget)?;
                    return self.collect(f, b, scale.wrapping_neg(), budget);
                }
                Data::Binary {
                    opcode: Op::Imul,
                    args: [a, b],
                } => {
                    if let Some(n) = literal(f, b) {
                        return self.collect(f, a, scale.wrapping_mul(n), budget);
                    }
                    if let Some(n) = literal(f, a) {
                        return self.collect(f, b, scale.wrapping_mul(n), budget);
                    }
                }
                Data::BinaryImm64 { opcode, arg, imm } => {
                    let n = imm.bits() as u64;
                    match opcode {
                        Op::IaddImm => {
                            self.bias = self.bias.wrapping_add(n.wrapping_mul(scale));
                            return self.collect(f, arg, scale, budget);
                        }
                        Op::ImulImm => return self.collect(f, arg, scale.wrapping_mul(n), budget),
                        Op::IshlImm => {
                            return self.collect(
                                f,
                                arg,
                                scale.wrapping_shl((n & 63) as u32),
                                budget,
                            );
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        // Casts, masks, variable shifts, calls and block parameters are atoms.
        // In particular, never distribute arithmetic across a narrowing cast.
        let term = self.terms.entry(value).or_default();
        *term = term.wrapping_add(scale);
        Some(())
    }
}

struct Address {
    inst: Inst,
    base: Value,
    index: Value,
    scale: u64,
    bias: u64,
}

fn definition(f: &Function, value: Value) -> Option<Data> {
    let ValueDef::Result(inst, 0) = f.dfg.value_def(f.dfg.resolve_aliases(value)) else {
        return None;
    };
    Some(f.dfg.insts[inst])
}

impl Address {
    fn is_canonical(&self, f: &Function, pointer: Value) -> bool {
        let Some(Data::Binary {
            opcode: Op::Iadd,
            args,
        }) = definition(f, pointer)
        else {
            return false;
        };
        [(args[0], args[1]), (args[1], args[0])].into_iter().any(|(base, index)| {
            let biased = (self.bias == 0 && base == self.base)
                || matches!(definition(f, base), Some(Data::BinaryImm64 { opcode: Op::IaddImm, arg, imm })
                    if arg == self.base && imm.bits() as u64 == self.bias);
            let scaled = (self.scale == 1 && index == self.index)
                || matches!(definition(f, index), Some(Data::BinaryImm64 { opcode: Op::ImulImm, arg, imm })
                    if arg == self.index && imm.bits() as u64 == self.scale)
                || matches!(definition(f, index), Some(Data::BinaryImm64 { opcode: Op::IshlImm, arg, imm })
                    if arg == self.index && 1u64 << (imm.bits() as u64 & 63) == self.scale);
            biased && scaled
        })
    }
}

fn address(f: &Function, pointer: Value) -> Option<Address> {
    let pointer = f.dfg.resolve_aliases(pointer);
    let inst = f.dfg.value_def(pointer).inst()?;
    let mut sum = Sum::default();
    sum.collect(f, pointer, 1, &mut 48)?;
    sum.terms.retain(|_, scale| *scale != 0);
    if sum.terms.len() != 2 {
        return None;
    }
    let (&base, _) = sum.terms.iter().find(|(_, scale)| **scale == 1)?;
    let (&index, &scale) = sum.terms.iter().find(|(value, _)| **value != base)?;
    if !scale.is_power_of_two() || scale > 16 {
        return None;
    }
    let result = Address {
        inst,
        base,
        index,
        scale,
        bias: sum.bias,
    };
    (!result.is_canonical(f, pointer)).then_some(result)
}

pub(in crate::native_artifact) fn normalize(f: &mut Function, hoist: bool) -> usize {
    let pointers = f
        .layout
        .blocks()
        .flat_map(|block| f.layout.block_insts(block))
        .filter_map(|inst| match f.dfg.insts[inst] {
            Data::Load { arg, .. } => Some(arg),
            Data::Store {
                args: [_, pointer], ..
            } => Some(pointer),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let plans = pointers
        .into_iter()
        .filter_map(|pointer| address(f, pointer))
        .collect::<Vec<_>>();
    let mut bases = BTreeMap::new();
    for plan in &plans {
        let base = if plan.bias == 0 {
            plan.base
        } else if let Some(&base) = bases.get(&(plan.base, plan.bias)) {
            base
        } else {
            let anchor = hoist
                .then(|| match f.dfg.value_def(plan.base) {
                    ValueDef::Param(block, _) => f.layout.first_inst(block),
                    ValueDef::Result(inst, _) => f.layout.next_inst(inst),
                    ValueDef::Union(..) => None,
                })
                .flatten();
            let mut cursor = FuncCursor::new(f);
            cursor.goto_inst(anchor.unwrap_or(plan.inst));
            let base = cursor.ins().iadd_imm(plan.base, plan.bias as i64);
            if anchor.is_some() {
                bases.insert((plan.base, plan.bias), base);
            }
            base
        };
        let mut cursor = FuncCursor::new(f);
        cursor.goto_inst(plan.inst);
        let index = if plan.scale == 1 {
            plan.index
        } else {
            cursor
                .ins()
                .ishl_imm(plan.index, i64::from(plan.scale.trailing_zeros()))
        };
        // Only wrapping I64 arithmetic was distributed. The pointer is equal
        // modulo 2^64, including overflow and negative biases. Memory offsets,
        // flags, source guards and effect ordering are unchanged.
        cursor.func.dfg.replace(plan.inst).iadd(base, index);
    }
    plans.len()
}

pub(in crate::native_artifact) fn rebase_bounded_comparisons(f: &mut Function) -> usize {
    let cfg = ControlFlowGraph::with_function(f);
    let mut facts = Facts::across_edges(f, &cfg, None);
    let replacements = f
        .layout
        .blocks()
        .flat_map(|block| f.layout.block_insts(block))
        .filter_map(|inst| {
            let Data::IntCompareImm {
                opcode: Op::IcmpImm,
                cond: cond @ (IntCC::UnsignedLessThan | IntCC::UnsignedLessThanOrEqual),
                arg,
                imm,
            } = f.dfg.insts[inst]
            else {
                return None;
            };
            if f.dfg.value_type(arg) != types::I64 {
                return None;
            }
            let (base, bias) = constant_offset(f, arg);
            let limit = (imm.bits() as u64).checked_sub(bias)?;
            if bias == 0 || f.dfg.value_type(base) != types::I64 {
                return None;
            }
            // The rebased unsigned comparison is equivalent when the index
            // never wraps across zero after adding this bias.
            facts.value_range(base)?.1.checked_add(bias)?;
            Some((inst, cond, base, limit as i64))
        })
        .collect::<Vec<_>>();
    for &(inst, cond, base, limit) in &replacements {
        f.dfg.replace(inst).icmp_imm(cond, base, limit);
    }
    replacements.len()
}

#[cfg(all(test, feature = "jit"))]
#[path = "indexed_addresses_tests.rs"]
mod tests;
