//! Schedule an immutable packed word beside an earlier overlapping read.
//! A selected address admits the entire physical load independently of its
//! original control path. Original source guards, failures and effects stay put.
use super::{composed_reads::View, word_ranges::constant_offset};
use crate::native_artifact::{NativeCompileError, compiler::backend};
use cranelift_codegen::{
    Context,
    cursor::{Cursor, FuncCursor},
    dominator_tree::DominatorTree,
    ir::{
        Function, Inst, InstBuilder, InstructionData as Data, MemFlags, Opcode, Value, ValueDef,
        condcodes::IntCC, types,
    },
};
use cranelift_module::{DataDescription, Module};
use std::collections::{BTreeMap, BTreeSet};

struct Read {
    load: Inst,
    index: Value,
    pointer: Value,
    length: Value,
    flags: MemFlags,
}

fn available(f: &Function, dom: &DominatorTree, value: Value, point: Inst) -> bool {
    match f.dfg.value_def(f.dfg.resolve_aliases(value)) {
        ValueDef::Result(inst, _) => inst != point && dom.dominates(inst, point, &f.layout),
        ValueDef::Param(block, _) => dom.dominates(block, point, &f.layout),
        _ => false,
    }
}

fn read(
    ctx: &Context,
    inst: Inst,
    loads: &BTreeMap<Value, (Inst, usize)>,
    views: &[View],
) -> Option<Read> {
    let f = &ctx.func;
    let Data::Load {
        opcode: Opcode::Uload32,
        arg,
        offset,
        flags,
    } = f.dfg.insts[inst]
    else {
        return None;
    };
    if !flags.notrap()
        || i32::from(offset) != 0
        || f.dfg.value_type(f.dfg.first_result(inst)) != types::I64
    {
        return None;
    }
    let ValueDef::Result(address, 0) = f.dfg.value_def(f.dfg.resolve_aliases(arg)) else {
        return None;
    };
    let Data::Binary {
        opcode: Opcode::Iadd,
        args: [a, b],
    } = f.dfg.insts[address]
    else {
        return None;
    };
    for (pointer, index) in [(a, b), (b, a)] {
        let pointer = f.dfg.resolve_aliases(pointer);
        let Some(&(_, word)) = loads.get(&pointer) else {
            continue;
        };
        let Some(view) = views
            .iter()
            .find(|view| view.pointer_word == word && view.width == 1)
        else {
            continue;
        };
        let length = loads.iter().find_map(|(&value, &(_, word))| {
            (word == view.length_word && available(f, &ctx.domtree, value, inst)).then_some(value)
        })?;
        if f.dfg.value_type(index) == types::I64 {
            return Some(Read {
                load: inst,
                index: f.dfg.resolve_aliases(index),
                pointer,
                length,
                flags,
            });
        }
    }
    None
}

/// `views` must come exclusively from the private immutable fixed-view loan.
/// Mere readonly source syntax, backing capacity or public metadata is insufficient.
pub(in crate::native_artifact) fn schedule<M: Module>(
    module: &mut M,
    ctx: &mut Context,
    loads: &BTreeMap<Value, (Inst, usize)>,
    views: &[View],
) -> Result<usize, NativeCompileError> {
    schedule_with_delay(module, ctx, loads, views, false)
}

pub(in crate::native_artifact) fn schedule_delayed<M: Module>(
    module: &mut M,
    ctx: &mut Context,
    loads: &BTreeMap<Value, (Inst, usize)>,
    views: &[View],
) -> Result<usize, NativeCompileError> {
    schedule_with_delay(module, ctx, loads, views, true)
}

// Stay on a single unconditional path, stopping before its next load, effect,
// branch or possible trap. Do not extend a value's lifetime across the pure
// address/hash setup if its load can instead precede the next memory access.
fn delayed_point(ctx: &Context, anchor: Inst, target: Inst) -> Inst {
    let f = &ctx.func;
    let mut point = anchor;
    let mut next = f.layout.next_inst(anchor);
    let mut seen = BTreeSet::from([anchor]);
    for _ in 0..128 {
        let Some(inst) = next else { break };
        if !seen.insert(inst) || !ctx.domtree.dominates(inst, target, &f.layout) {
            break;
        }
        point = inst;
        match f.dfg.insts[inst] {
            Data::Jump { destination, .. } => {
                let block = destination.block(&f.dfg.value_lists);
                if ctx.cfg.pred_iter(block).count() != 1 {
                    break;
                }
                next = f.layout.first_inst(block);
            }
            _ if matches!(
                f.dfg.insts[inst].opcode(),
                Opcode::Iconst
                    | Opcode::Iadd
                    | Opcode::IaddImm
                    | Opcode::Isub
                    | Opcode::Imul
                    | Opcode::ImulImm
                    | Opcode::Band
                    | Opcode::BandImm
                    | Opcode::Bor
                    | Opcode::BorImm
                    | Opcode::Bxor
                    | Opcode::BxorImm
                    | Opcode::Ishl
                    | Opcode::IshlImm
                    | Opcode::Ushr
                    | Opcode::UshrImm
                    | Opcode::Icmp
                    | Opcode::IcmpImm
                    | Opcode::Uextend
                    | Opcode::Ireduce
                    | Opcode::Select
            ) =>
            {
                next = f.layout.next_inst(inst)
            }
            _ => break,
        }
    }
    point
}

fn schedule_with_delay<M: Module>(
    module: &mut M,
    ctx: &mut Context,
    loads: &BTreeMap<Value, (Inst, usize)>,
    views: &[View],
    delay: bool,
) -> Result<usize, NativeCompileError> {
    ctx.flowgraph();
    let f = &ctx.func;
    let reads = f
        .layout
        .blocks()
        .flat_map(|b| f.layout.block_insts(b))
        .filter_map(|inst| read(ctx, inst, loads, views))
        .take(129)
        .collect::<Vec<_>>();
    if reads.len() > 128 {
        return Ok(0);
    }
    let mut plans = Vec::new();
    for target in &reads {
        let (root, displacement) = constant_offset(f, target.index);
        let anchor = reads.iter().find(|prior| {
            let (other, bias) = constant_offset(f, prior.index);
            prior.load != target.load
                && prior.pointer == target.pointer
                && prior.length == target.length
                && root == other
                && bias.wrapping_sub(displacement) > 0
                && bias.wrapping_sub(displacement) < 4
                && ctx.domtree.dominates(prior.load, target.load, &f.layout)
                && [target.index, target.pointer, target.length]
                    .into_iter()
                    .all(|value| available(f, &ctx.domtree, value, prior.load))
        });
        if let Some(anchor) = anchor {
            let point = if delay {
                delayed_point(ctx, anchor.load, target.load)
            } else {
                anchor.load
            };
            plans.push((target, point));
        }
    }
    if plans.is_empty() {
        return Ok(0);
    }
    let object = module
        .declare_anonymous_data(false, false)
        .map_err(backend)?;
    let mut data = DataDescription::new();
    data.set_align(16);
    data.define(vec![0; 16].into_boxed_slice());
    module.define_data(object, &data).map_err(backend)?;
    let global = module.declare_data_in_func(object, &mut ctx.func);
    for (target, point) in &plans {
        let mut c = FuncCursor::new(&mut ctx.func);
        c.goto_inst(*point);
        c.use_srcloc(target.load);
        let zero = c.ins().global_value(types::I64, global);
        let enough = c
            .ins()
            .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, target.length, 4);
        let last = c.ins().iadd_imm(target.length, -4);
        let fits = c
            .ins()
            .icmp(IntCC::UnsignedLessThanOrEqual, target.index, last);
        let valid = c.ins().band(enough, fits);
        let address = c.ins().iadd(target.pointer, target.index);
        let selected = c.ins().select(valid, address, zero);
        // The private current view and this full-word bound make the selected
        // load immutable and nontrapping even when its old path is not taken.
        let flags = target.flags.with_notrap().with_readonly();
        let word = c.ins().uload32(flags, selected, 0);
        c.func.dfg.replace(target.load).iadd_imm(word, 0);
    }
    Ok(plans.len())
}

#[cfg(all(test, feature = "jit"))]
#[path = "immutable_word_reads_tests.rs"]
mod tests;
