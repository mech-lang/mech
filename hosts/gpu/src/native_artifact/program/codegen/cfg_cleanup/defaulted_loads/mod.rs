//! Select a safe address for a defaulted read before issuing its only load.
use super::{
    composed_reads::{View, arm, defaulted_read, read_comparison},
    word_ranges::literal,
};
use crate::native_artifact::{NativeCompileError, compiler::backend};
use cranelift_codegen::Context;
use cranelift_codegen::cursor::{Cursor, FuncCursor};
use cranelift_codegen::flowgraph::ControlFlowGraph;
use cranelift_codegen::ir::{
    Block, Function, Inst, InstBuilder, InstructionData as Data, Opcode as Op, Value,
    condcodes::IntCC, types,
};
use cranelift_module::{DataDescription, Module};
use std::collections::BTreeMap;

mod immutable;

struct Read {
    branch: Inst,
    condition: Value,
    load: Inst,
    address: Value,
    compute_address: Vec<Inst>,
    join: Block,
}

fn find(f: &Function, cfg: &ControlFlowGraph, branch: Inst) -> Option<Read> {
    let Data::Brif { arg, blocks, .. } = f.dfg.insts[branch] else {
        return None;
    };
    let cmp = read_comparison(f, arg)?;
    if !matches!(
        f.dfg.insts[cmp],
        Data::IntCompare {
            cond: IntCC::UnsignedLessThan,
            ..
        } | Data::IntCompareImm {
            cond: IntCC::UnsignedLessThan,
            ..
        }
    ) || blocks
        .iter()
        .any(|call| call.args(&f.dfg.value_lists).next().is_some())
    {
        return None;
    }
    let (join, value, selected, loads) = arm(f, cfg, blocks[0].block(&f.dfg.value_lists))?;
    let (other_join, default, _, default_loads) = arm(f, cfg, blocks[1].block(&f.dfg.value_lists))?;
    let &[load] = loads.as_slice() else {
        return None;
    };
    let Data::Load {
        opcode: Op::Uload8 | Op::Uload32,
        arg: address,
        offset,
        ..
    } = f.dfg.insts[load]
    else {
        return None;
    };
    if join != other_join
        || cfg.pred_iter(join).count() != 2
        || f.dfg.block_params(join).len() != 1
        || !default_loads.is_empty()
        || literal(f, default) != Some(0)
        || f.dfg.resolve_aliases(value) != f.dfg.first_result(load)
        || i32::from(offset) != 0
        || f.dfg.value_type(address) != types::I64
    {
        return None;
    }

    // Every arm block has one predecessor and ends with an unconditional
    // jump. Its address arithmetic is total and dominates the single load.
    let mut compute_address = Vec::new();
    let mut loaded = false;
    for block in selected {
        for inst in f.layout.block_insts(block) {
            if inst == load {
                loaded = true;
            } else if f.dfg.insts[inst].opcode() != Op::Jump {
                if loaded
                    || !matches!(
                        f.dfg.insts[inst].opcode(),
                        Op::Iconst | Op::Iadd | Op::IaddImm | Op::Isub | Op::ImulImm | Op::IshlImm
                    )
                {
                    return None;
                }
                compute_address.push(inst);
            }
        }
    }

    Some(Read {
        branch,
        condition: f.dfg.first_result(cmp),
        load,
        address,
        compute_address,
        join,
    })
}

pub(in crate::native_artifact) fn fold<M: Module>(
    module: &mut M,
    context: &mut Context,
) -> Result<usize, NativeCompileError> {
    fold_with_views(module, context, None, false).map(|(folded, _)| folded)
}

/// Only the private caller's fixed, immutable view loan may supply these
/// packet words. A matching source bound and address make the selected load
/// safe independently of control flow; the zero arm uses immutable data.
pub(in crate::native_artifact) fn fold_immutable<M: Module>(
    module: &mut M,
    context: &mut Context,
    packet_loads: &BTreeMap<Value, (Inst, usize)>,
    views: &[View],
    rematerialize: bool,
) -> Result<(usize, usize), NativeCompileError> {
    fold_with_views(module, context, Some((packet_loads, views)), rematerialize)
}

type ImmutableViews<'a> = (&'a BTreeMap<Value, (Inst, usize)>, &'a [View]);

fn fold_with_views<M: Module>(
    module: &mut M,
    context: &mut Context,
    immutable: Option<ImmutableViews<'_>>,
    rematerialize: bool,
) -> Result<(usize, usize), NativeCompileError> {
    context.flowgraph();
    let endianness = module.isa().endianness();
    let f = &mut context.func;
    let reads = f
        .layout
        .blocks()
        .filter_map(|block| {
            let branch = f.layout.last_inst(block)?;
            let read = find(f, &context.cfg, branch)?;
            let pure = immutable.and_then(|(loads, views)| {
                let matched = defaulted_read(f, &context.cfg, loads, views, branch, endianness)?;
                let Data::Load { flags, .. } = f.dfg.insts[read.load] else {
                    return None;
                };
                Some(immutable::Spec {
                    index: matched.index,
                    pointer_word: matched.view.pointer_word,
                    length_word: matched.view.length_word,
                    width: matched.view.width,
                    flags,
                })
            });
            // This private-loan pass changes only certified immutable reads.
            // Keep every other checked branch in place; the independent
            // ordinary safe-address fold can still be enabled separately.
            if immutable.is_some() && pure.is_none() {
                return None;
            }
            Some((read, pure))
        })
        .collect::<Vec<_>>();
    if reads.is_empty() {
        return Ok((0, 0));
    }
    let (groups, owners) = immutable.map_or_else(
        || (Vec::new(), vec![None; reads.len()]),
        |views| immutable::plan(f, &context.cfg, &context.domtree, &reads, views),
    );

    let object = module
        .declare_anonymous_data(false, false)
        .map_err(backend)?;
    let mut data = DataDescription::new();
    data.define(vec![0_u8; 8].into_boxed_slice());
    data.set_align(8);
    module.define_data(object, &data).map_err(backend)?;
    let global = module.declare_data_in_func(object, f);
    let entry = f
        .layout
        .entry_block()
        .expect("a checked read requires an entry");
    let mut cursor = FuncCursor::new(f);
    cursor.goto_first_inst(entry);
    let zero = cursor.ins().global_value(types::I64, global);
    let values = groups
        .iter()
        .map(|group| group.emit(&mut cursor, zero, rematerialize))
        .collect::<Vec<_>>();

    let count = reads.len();
    let immutable_count = owners.iter().flatten().count();
    for (
        (
            Read {
                branch,
                condition,
                load,
                address,
                compute_address,
                join,
            },
            _,
        ),
        owner,
    ) in reads.into_iter().zip(owners)
    {
        if let Some(owner) = owner {
            let loaded = values[owner];
            cursor.func.dfg.replace(branch).jump(join, &[loaded.into()]);
            continue;
        }
        for inst in compute_address {
            cursor.func.layout.remove_inst(inst);
            cursor.func.layout.insert_inst(inst, branch);
        }
        cursor.goto_inst(branch);
        cursor.use_srcloc(load);
        // Integer address calculation can wrap on the default path. The
        // wrapping address is never used there. The load keeps the original
        // flags and the admitted address whenever the check succeeds.
        let selected = cursor.ins().select(condition, address, zero);
        let Data::Load { arg, .. } = &mut cursor.func.dfg.insts[load] else {
            unreachable!("validated load shape");
        };
        *arg = selected;
        cursor.func.layout.remove_inst(load);
        cursor.func.layout.insert_inst(load, branch);
        let loaded = cursor.func.dfg.first_result(load);
        cursor.func.dfg.replace(branch).jump(join, &[loaded.into()]);
    }
    let isa = module.isa();
    context.flowgraph();
    context.eliminate_unreachable_code(isa).map_err(backend)?;
    context.verify(isa).map_err(backend)?;
    Ok((count, immutable_count))
}

#[cfg(all(test, feature = "jit", target_pointer_width = "64"))]
mod tests;
