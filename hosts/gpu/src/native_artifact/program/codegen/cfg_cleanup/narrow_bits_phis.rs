//! Reuse a separately carried low word only after checking every incoming edge.
use cranelift_codegen::flowgraph::ControlFlowGraph;
use cranelift_codegen::ir::{Function, InstructionData as Data, Opcode as Op, Value, ValueDef};
use std::collections::BTreeMap;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Origin {
    Value(Value),
    Constant(u64),
}

fn origin(f: &Function, value: Value, bits: u32, depth: usize) -> Origin {
    let value = f.dfg.resolve_aliases(value);
    let opaque = Origin::Value(value);
    if depth == 0 {
        return opaque;
    }
    let ty = f.dfg.value_type(value);
    if !ty.is_int() || ty.is_vector() || ty.bits() > 64 || ty.bits() < bits {
        return opaque;
    }
    let mask = u64::MAX >> (64 - bits);
    let mut steps = 32;
    if let Some(constant) = super::super::constant_integer(f, value, 8, &mut steps) {
        return Origin::Constant(constant & mask);
    }
    let ValueDef::Result(inst, 0) = f.dfg.value_def(value) else {
        return opaque;
    };
    let next = |value| origin(f, value, bits, depth - 1);
    match f.dfg.insts[inst] {
        Data::Unary {
            opcode: Op::Ireduce | Op::Uextend | Op::Sextend,
            arg,
        } if f.dfg.value_type(arg).bits() >= bits => next(arg),
        Data::BinaryImm64 {
            opcode: Op::BandImm,
            arg,
            imm,
        } if imm.bits() as u64 & mask == mask => next(arg),
        Data::Binary { opcode, args } => {
            for (field, keep) in [(args[0], args[1]), (args[1], args[0])] {
                if opcode == Op::Band {
                    let mut steps = 32;
                    if super::super::constant_integer(f, field, 8, &mut steps)
                        .is_some_and(|constant| constant & mask == mask)
                    {
                        return next(keep);
                    }
                } else if matches!(opcode, Op::Bor | Op::Bxor) {
                    let mut steps = 64;
                    if super::zero_low_bits(f, field, bits, 8, &mut steps) {
                        return next(keep);
                    }
                }
            }
            opaque
        }
        _ => opaque,
    }
}

pub(super) fn fold(f: &mut Function) -> usize {
    // Debug identities require their own alias update contract. This optional
    // optimization leaves observed functions on the existing path.
    if f.dfg.values_labels.is_some() || !f.debug_tags.is_empty() {
        return 0;
    }
    let cfg = ControlFlowGraph::with_function(f);
    let mut parallel = BTreeMap::new();
    for block in f.layout.blocks() {
        if Some(block) == f.layout.entry_block() {
            continue;
        }
        let params = f.dfg.block_params(block);
        for (wide_slot, &wide) in params.iter().enumerate() {
            let wide_ty = f.dfg.value_type(wide);
            if !wide_ty.is_int() || wide_ty.is_vector() || wide_ty.bits() > 64 {
                continue;
            }
            for (low_slot, &low) in params.iter().enumerate() {
                let low_ty = f.dfg.value_type(low);
                if !low_ty.is_int() || low_ty.is_vector() || low_ty.bits() >= wide_ty.bits() {
                    continue;
                }
                let bits = low_ty.bits();
                let left = super::super::incoming_block_arguments(f, &cfg, block, wide_slot)
                    .collect::<Vec<_>>();
                let right = super::super::incoming_block_arguments(f, &cfg, block, low_slot)
                    .collect::<Vec<_>>();
                if left.is_empty() || left.len() != right.len() {
                    continue;
                }
                // No phi relation is assumed while proving the inputs. A
                // forwarded self-edge or unknown argument fails conservatively;
                // local pure expressions must prove equality on every edge.
                let agrees = left.iter().zip(&right).all(|((a, x), (b, y))| {
                    a == b
                        && x.zip(*y)
                            .is_some_and(|(x, y)| origin(f, x, bits, 8) == origin(f, y, bits, 8))
                });
                if agrees {
                    parallel.entry((wide, bits)).or_insert(low);
                }
            }
        }
    }
    let mut changes = Vec::new();
    for block in f.layout.blocks() {
        for inst in f.layout.block_insts(block) {
            let Data::Unary {
                opcode: Op::Ireduce,
                arg,
            } = f.dfg.insts[inst]
            else {
                continue;
            };
            let result = f.dfg.first_result(inst);
            let ty = f.dfg.value_type(result);
            if !ty.is_int() || ty.is_vector() || ty.bits() > 64 {
                continue;
            }
            let Origin::Value(wide) = origin(f, arg, ty.bits(), 8) else {
                continue;
            };
            if let Some(&low) = parallel.get(&(wide, ty.bits())) {
                // Both block parameters have the same dominance region. Any
                // valid use of the wide parameter can use its proved low peer.
                changes.push((inst, result, low));
            }
        }
    }
    for &(inst, result, low) in &changes {
        f.dfg.detach_inst_results(inst);
        f.dfg.change_to_alias(result, low);
        f.layout.remove_inst(inst);
    }
    if !changes.is_empty() {
        super::super::eliminate_dead_values(f);
    }
    changes.len()
}
