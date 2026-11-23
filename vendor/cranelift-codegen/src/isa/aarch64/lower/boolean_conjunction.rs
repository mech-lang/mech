//! Bounded, read-only preflight for a canonical integer comparison conjunction.
//!
//! Only ICMP results establish zero/one semantics. The surrounding conversions
//! and ANDs must all be single-use as reported by lowering's existing use-state
//! analysis; unknown Boolean inputs, phis, floats and shared prefixes decline.
//! No instruction emission or sinking occurs before this whole preflight passes.
use crate::ir::{
    DataFlowGraph, InstructionData as D, Opcode as O, Type, Value, ValueDef, condcodes::IntCC,
    types,
};

pub(super) const MAX_COMPARISONS: usize = 8;
const MAX_DEPTH: usize = 16;
const MAX_NODES: usize = 64;

#[derive(Clone, Copy, Debug)]
pub(super) struct Comparison {
    pub cc: IntCC,
    pub left: Value,
    pub right: Value,
    pub ty: Type,
}

#[derive(Clone, Copy, Debug)]
pub(in crate::isa::aarch64) struct Chain {
    comparisons: [Option<Comparison>; MAX_COMPARISONS],
    len: usize,
}

impl Chain {
    pub(super) fn iter(&self) -> impl Iterator<Item = Comparison> + '_ {
        self.comparisons[..self.len].iter().map(|c| c.unwrap())
    }
}

fn is_one(dfg: &DataFlowGraph, value: Value) -> bool {
    let value = dfg.resolve_aliases(value);
    let ValueDef::Result(inst, 0) = dfg.value_def(value) else {
        return false;
    };
    matches!(dfg.insts[inst], D::UnaryImm { opcode: O::Iconst, imm } if imm.bits() == 1)
}

/// `unique` is the lowering engine's unique-use test, including its transitive
/// multiple-use propagation. Comparison operands are ordinary typed SSA values;
/// their producers are never consumed or cloned by this proof.
pub(super) fn derive(
    dfg: &DataFlowGraph,
    root: Value,
    mut unique: impl FnMut(Value) -> bool,
) -> Option<Chain> {
    fn visit(
        dfg: &DataFlowGraph,
        value: Value,
        unique: &mut impl FnMut(Value) -> bool,
        chain: &mut Chain,
        depth: usize,
        remaining: &mut usize,
    ) -> Option<()> {
        if depth == 0 || *remaining == 0 {
            return None;
        }
        *remaining -= 1;
        let value = dfg.resolve_aliases(value);
        let ty = dfg.value_type(value);
        if !ty.is_int() || ty.bits() > 64 || !unique(value) {
            return None;
        }
        let ValueDef::Result(inst, 0) = dfg.value_def(value) else {
            return None;
        };
        match dfg.insts[inst] {
            D::Unary {
                opcode: O::Uextend | O::Ireduce,
                arg,
            } => {
                // The recursive proof establishes 0/1, which every supported
                // integer width preserves. This is not a raw bit-test proof.
                visit(dfg, arg, unique, chain, depth - 1, remaining)
            }
            D::BinaryImm64 {
                opcode: O::BandImm,
                arg,
                imm,
            } if imm.bits() == 1 => visit(dfg, arg, unique, chain, depth - 1, remaining),
            D::Binary {
                opcode: O::Band,
                args: [left, right],
            } => {
                if is_one(dfg, left) {
                    visit(dfg, right, unique, chain, depth - 1, remaining)
                } else if is_one(dfg, right) {
                    visit(dfg, left, unique, chain, depth - 1, remaining)
                } else {
                    visit(dfg, left, unique, chain, depth - 1, remaining)?;
                    visit(dfg, right, unique, chain, depth - 1, remaining)
                }
            }
            D::IntCompare {
                cond,
                args: [left, right],
                ..
            } => {
                let input_ty = dfg.value_type(left);
                if ty != types::I8
                    || input_ty != dfg.value_type(right)
                    || !matches!(input_ty, types::I8 | types::I16 | types::I32 | types::I64)
                    || chain.len == MAX_COMPARISONS
                {
                    return None;
                }
                chain.comparisons[chain.len] = Some(Comparison {
                    cc: cond,
                    left,
                    right,
                    ty: input_ty,
                });
                chain.len += 1;
                Some(())
            }
            _ => None,
        }
    }
    let mut chain = Chain {
        comparisons: [None; MAX_COMPARISONS],
        len: 0,
    };
    let mut remaining = MAX_NODES;
    visit(
        dfg,
        root,
        &mut unique,
        &mut chain,
        MAX_DEPTH,
        &mut remaining,
    )?;
    (chain.len >= 2).then_some(chain)
}

/// Flags that make this exact integer condition false when an earlier AND
/// operand was false. These are condition-specific, not a shared NZCV guess.
pub(super) fn false_flags(cc: IntCC) -> crate::isa::aarch64::inst::NZCV {
    use IntCC::*;
    let bits = match cc {
        Equal
        | UnsignedGreaterThanOrEqual
        | UnsignedGreaterThan
        | SignedLessThan
        | SignedLessThanOrEqual => 0,
        NotEqual | SignedGreaterThan => 4,
        UnsignedLessThan | UnsignedLessThanOrEqual => 2,
        SignedGreaterThanOrEqual => 1,
    };
    crate::isa::aarch64::inst::NZCV::new(bits & 8 != 0, bits & 4 != 0, bits & 2 != 0, bits & 1 != 0)
}

#[cfg(test)]
mod tests;
