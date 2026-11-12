//! Retain canonical zero/one facts across SSA joins and loop-carried words.
//! This runs after unreachable-code removal. It never trusts a source kind,
//! raw input, memory load or call result. Every incoming phi edge participates.
use cranelift_codegen::cursor::{Cursor, FuncCursor};
use cranelift_codegen::ir::{
    Function, InstBuilder, InstructionData as D, Opcode as O, Value, ValueDef, condcodes::IntCC,
    types,
};
use std::collections::VecDeque;

const MAX_VALUES: usize = 131_072;
const MAX_EDGES: usize = 524_288;

/// Resolve the operand of `& 1` to an integer comparison. Unsigned
/// extensions preserve the logical zero/one result. The caller supplies
/// any incoming-edge resolution, so a proof cannot borrow another path's
/// block arguments.
pub(in super::super) fn masked_integer_comparison(
    f: &Function,
    instruction: &D,
    mut resolve: impl FnMut(Value) -> Option<Value>,
) -> Option<Value> {
    let D::BinaryImm64 {
        opcode: O::BandImm,
        arg,
        imm,
    } = *instruction
    else {
        return None;
    };
    if imm.bits() != 1 {
        return None;
    }
    let mut value = arg;
    for _ in 0..8 {
        value = resolve(value)?;
        let ValueDef::Result(definition, 0) = f.dfg.value_def(value) else {
            return None;
        };
        match f.dfg.insts[definition] {
            D::Unary {
                opcode: O::Uextend,
                arg,
            } => value = arg,
            D::IntCompare { .. } | D::IntCompareImm { .. }
                if f.dfg.value_type(value) == types::I8 =>
            {
                return Some(value);
            }
            _ => return None,
        }
    }
    None
}

fn literal(f: &Function, value: Value) -> Option<u64> {
    let value = f.dfg.resolve_aliases(value);
    let ValueDef::Result(inst, 0) = f.dfg.value_def(value) else {
        return None;
    };
    let D::UnaryImm {
        opcode: O::Iconst,
        imm,
    } = f.dfg.insts[inst]
    else {
        return None;
    };
    let bits = f.dfg.value_type(value).bits();
    (bits > 0 && bits <= 64).then(|| imm.bits() as u64 & (u64::MAX >> (64 - bits)))
}

fn canonical_values(f: &Function) -> Option<Vec<bool>> {
    let count = f.dfg.num_values();
    if count > MAX_VALUES {
        return None;
    }
    let mut dependencies = vec![None; count];
    let entry = f.layout.entry_block()?;
    for block in f.layout.blocks() {
        if block != entry {
            for &parameter in f.dfg.block_params(block) {
                dependencies[parameter.as_u32() as usize] = Some(Vec::new());
            }
        }
        for inst in f.layout.block_insts(block) {
            let &[value] = f.dfg.inst_results(inst) else {
                continue;
            };
            let ty = f.dfg.value_type(value);
            if !ty.is_int() || ty.bits() > 64 {
                continue;
            }
            let inputs = match f.dfg.insts[inst] {
                D::UnaryImm {
                    opcode: O::Iconst, ..
                } if matches!(literal(f, value), Some(0 | 1)) => Some(Vec::new()),
                D::IntCompare { .. } | D::IntCompareImm { .. } | D::FloatCompare { .. } => {
                    Some(Vec::new())
                }
                D::Unary {
                    opcode: O::Uextend | O::Sextend | O::Ireduce,
                    arg,
                } => Some(vec![arg]),
                D::Binary {
                    opcode: O::Band,
                    args,
                } if args.iter().any(|&v| matches!(literal(f, v), Some(0 | 1))) => Some(Vec::new()),
                D::Binary {
                    opcode: O::Band | O::Bor | O::Bxor,
                    args,
                } => Some(args.to_vec()),
                D::BinaryImm64 {
                    opcode: O::BandImm,
                    imm,
                    ..
                } if matches!(imm.bits(), 0 | 1) => Some(Vec::new()),
                D::BinaryImm64 {
                    opcode: O::BorImm | O::BxorImm,
                    arg,
                    imm,
                } if matches!(imm.bits(), 0 | 1) => Some(vec![arg]),
                D::Ternary {
                    opcode: O::Select,
                    args,
                } => Some(args[1..].to_vec()),
                _ => None,
            };
            dependencies[value.as_u32() as usize] = inputs;
        }
    }
    let mut edges = 0usize;
    for block in f.layout.blocks() {
        let Some(inst) = f.layout.last_inst(block) else {
            continue;
        };
        for call in
            f.dfg.insts[inst].branch_destination(&f.dfg.jump_tables, &f.dfg.exception_tables)
        {
            let target = call.block(&f.dfg.value_lists);
            let parameters = f.dfg.block_params(target);
            if call.len(&f.dfg.value_lists) != parameters.len() {
                return None;
            }
            for (&parameter, arg) in parameters.iter().zip(call.args(&f.dfg.value_lists)) {
                edges = edges.checked_add(1)?;
                if edges > MAX_EDGES {
                    return None;
                }
                let deps = &mut dependencies[parameter.as_u32() as usize];
                if let Some(input) = arg.as_value() {
                    if let Some(inputs) = deps {
                        inputs.push(input);
                    }
                } else {
                    *deps = None;
                }
            }
        }
    }
    // A greatest fixed point handles induction through copies, Boolean
    // operations and phis. Unknown seeds invalidate all transitive users.
    // In reachable, well-formed SSA every executed cycle has an entry edge;
    // that edge must itself supply a canonical seed. A non-Boolean backedge
    // invalidates the entire cycle, rather than being assumed canonical.
    let mut known = dependencies.iter().map(Option::is_some).collect::<Vec<_>>();
    let mut users = vec![Vec::new(); count];
    for (at, deps) in dependencies.iter().enumerate() {
        if let Some(deps) = deps {
            for &input in deps {
                edges = edges.checked_add(1)?;
                if edges > MAX_EDGES {
                    return None;
                }
                let input = f.dfg.resolve_aliases(input).as_u32() as usize;
                users[input].push(at);
            }
        }
    }
    let mut pending = known
        .iter()
        .enumerate()
        .filter_map(|(i, &yes)| (!yes).then_some(i))
        .collect::<VecDeque<_>>();
    while let Some(unknown) = pending.pop_front() {
        for &user in &users[unknown] {
            if known[user] {
                known[user] = false;
                pending.push_back(user);
            }
        }
    }
    Some(known)
}

fn condition(f: &Function, value: Value, known: &[bool], depth: usize) -> Option<(Value, bool)> {
    if depth == 0 {
        return None;
    }
    let value = f.dfg.resolve_aliases(value);
    let ValueDef::Result(inst, 0) = f.dfg.value_def(value) else {
        return None;
    };
    let (cc, left, right) = match f.dfg.insts[inst] {
        D::Unary {
            opcode: O::Uextend,
            arg,
        } => return condition(f, arg, known, depth - 1),
        D::IntCompare {
            cond, args: [a, b], ..
        } => (cond, a, literal(f, b)),
        D::IntCompareImm { cond, arg, imm, .. } => (cond, arg, Some(imm.bits() as u64)),
        _ => return None,
    };
    if !matches!(cc, IntCC::Equal | IntCC::NotEqual) {
        return None;
    }
    let (tested, word) = if let Some(word @ (0 | 1)) = right {
        (left, word)
    } else if let D::IntCompare { args: [a, b], .. } = f.dfg.insts[inst] {
        (b, literal(f, a).filter(|v| matches!(v, 0 | 1))?)
    } else {
        return None;
    };
    let tested = f.dfg.resolve_aliases(tested);
    known[tested.as_u32() as usize].then_some((tested, (cc == IntCC::Equal) == (word == 1)))
}

pub(super) fn optimize(f: &mut Function) -> usize {
    let Some(known) = canonical_values(f) else {
        return 0;
    };
    let changes = f
        .layout
        .blocks()
        .filter_map(|block| {
            let inst = f.layout.last_inst(block)?;
            let D::Brif { arg, .. } = f.dfg.insts[inst] else {
                return None;
            };
            let (value, positive) = condition(f, arg, &known, 8)?;
            Some((inst, value, positive))
        })
        .collect::<Vec<_>>();
    for &(inst, value, positive) in &changes {
        // Only bit zero matters for a proved zero/one value. Expose that
        // predicate to the backend's native bit-test branch rule, avoiding
        // narrow-word normalization on its ordinary full-word branch route.
        // Unsigned extensions preserve that bit, including an I8 producer's
        // otherwise unspecified high physical register bits.
        let mut value = value;
        for _ in 0..8 {
            let ValueDef::Result(definition, 0) = f.dfg.value_def(value) else {
                break;
            };
            let D::Unary {
                opcode: O::Uextend,
                arg,
            } = f.dfg.insts[definition]
            else {
                break;
            };
            value = f.dfg.resolve_aliases(arg);
        }
        let bit = FuncCursor::new(f).at_inst(inst).ins().band_imm(value, 1);
        let D::Brif { arg, blocks, .. } = &mut f.dfg.insts[inst] else {
            unreachable!()
        };
        *arg = bit;
        if !positive {
            blocks.swap(0, 1);
        }
    }
    changes.len()
}

#[cfg(test)]
mod tests;
