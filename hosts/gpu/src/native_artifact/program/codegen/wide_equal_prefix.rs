//! Forward a proved endpoint across the checked continuation of a short scan.
use super::{Certificate, Change, Resolve, candidate, reuse};
use cranelift_codegen::Context;
use cranelift_codegen::cursor::{Cursor, FuncCursor};
use cranelift_codegen::ir::{
    Block, BlockArg, BlockCall, Function, Inst, InstBuilder, InstructionData, Opcode, Value,
    ValueDef, types,
};
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct Handoff {
    pub(super) header: usize,
    entry: Block,
    selected: Inst,
    predecessors: BTreeSet<Inst>,
}

impl Handoff {
    pub(super) fn emit(self, f: &mut Function, end: Value) -> Value {
        let parameter = f.dfg.append_block_param(self.entry, types::I64);
        for inst in self.predecessors {
            let value = if inst == self.selected {
                end
            } else {
                FuncCursor::new(f).at_inst(inst).ins().iconst(types::I64, 0)
            };
            let dfg = &mut f.dfg;
            for call in dfg.insts[inst]
                .branch_destination_mut(&mut dfg.jump_tables, &mut dfg.exception_tables)
            {
                if call.block(&dfg.value_lists) == self.entry {
                    let mut args = call.args(&dfg.value_lists).collect::<Vec<_>>();
                    args.push(BlockArg::Value(value));
                    *call = BlockCall::new(self.entry, args, &mut dfg.value_lists);
                }
            }
        }
        parameter
    }
}

/// The bound exit leads through a unique, effect-free chain to one edge of
/// the continuation admission. The admission itself still executes.
fn selected_edge(context: &Context, prefix: &Change, entry: Block) -> Option<(Inst, BlockCall)> {
    let f = &context.func;
    let mut predecessor = prefix.limit_check;
    let mut block = prefix.limit_exit;
    for _ in 0..32 {
        let mut incoming = context.cfg.pred_iter(block);
        if block == entry || incoming.next()?.inst != predecessor || incoming.next().is_some() {
            return None;
        }
        if !f.layout.block_insts(block).all(|inst| {
            matches!(
                f.dfg.insts[inst].opcode(),
                Opcode::Iconst
                    | Opcode::Iadd
                    | Opcode::IaddImm
                    | Opcode::Isub
                    | Opcode::Icmp
                    | Opcode::IcmpImm
                    | Opcode::Uextend
                    | Opcode::Band
                    | Opcode::Jump
                    | Opcode::Brif
            )
        }) {
            return None;
        }
        let end = f.layout.last_inst(block)?;
        let edges =
            f.dfg.insts[end].branch_destination(&f.dfg.jump_tables, &f.dfg.exception_tables);
        let mut selected = edges
            .iter()
            .filter(|call| call.block(&f.dfg.value_lists) == entry);
        if let Some(call) = selected.next() {
            return selected.next().is_none().then_some((end, *call));
        }
        if f.dfg.insts[end].opcode() != Opcode::Jump {
            return None;
        }
        block = edges[0].block(&f.dfg.value_lists);
        predecessor = end;
    }
    None
}

fn addition(resolve: &Resolve<'_>, value: Value) -> Option<[Value; 2]> {
    let f = &resolve.context.func;
    let InstructionData::Binary {
        opcode: Opcode::Iadd,
        args,
    } = f.dfg.insts[resolve.inst(value)?]
    else {
        return None;
    };
    let mut operands = [resolve.value(args[0])?, resolve.value(args[1])?];
    operands.sort();
    Some(operands)
}

fn same_address(resolve: &Resolve<'_>, left: Value, right: Value) -> bool {
    let (Some(left), Some(right)) = (resolve.value(left), resolve.value(right)) else {
        return false;
    };
    left == right
        || addition(resolve, left).is_some_and(|args| Some(args) == addition(resolve, right))
}

fn prove(
    context: &Context,
    prefix_cert: &Certificate,
    prefix: &Change,
    suffix_cert: &Certificate,
    suffix: &Change,
) -> Option<Handoff> {
    let f = &context.func;
    let entry = f.layout.inst_block(suffix.entry)?;
    if f.layout.first_inst(entry)? != suffix.entry
        || f.dfg.insts[suffix.entry].opcode() != Opcode::Jump
        || !context
            .domtree
            .dominates(prefix_cert.header, prefix.limit_check, &f.layout)
    {
        return None;
    }
    let (selected, edge) = selected_edge(context, prefix, entry)?;
    let resolve = Resolve {
        context,
        header: suffix_cert.header,
    };
    let on_edge = |value| {
        let value = resolve.value(value)?;
        match f.dfg.value_def(value) {
            ValueDef::Param(block, index) if block == entry => {
                edge.args(&f.dfg.value_lists).nth(index)?.as_value()
            }
            _ => Some(value),
        }
    };
    let cursor = on_edge(suffix.initial(f, suffix_cert)?)?;
    let cursor = resolve.value(cursor)?;
    if cursor != f.dfg.block_params(prefix_cert.header)[0]
        || !prefix.bases.iter().zip(suffix.bases).all(|(&left, right)| {
            on_edge(right).is_some_and(|right| same_address(&resolve, left, right))
        })
    {
        return None;
    }
    Some(Handoff {
        header: suffix_cert.source_header,
        entry,
        selected,
        predecessors: context.cfg.pred_iter(entry).map(|pred| pred.inst).collect(),
    })
}

pub(super) fn continuation(
    context: &Context,
    prefix_cert: &Certificate,
    prefix: &Change,
    scope: reuse::Scope,
    later: &[Certificate],
    already_paired: &BTreeMap<usize, (reuse::State, Value)>,
) -> Option<Handoff> {
    for cert in later {
        if cert.reuse_scope != Some(scope) || already_paired.contains_key(&cert.source_header) {
            continue;
        }
        let Some(change) = candidate(context, cert) else {
            continue;
        };
        if !matches!(
            reuse::extent(context, cert, &change),
            Some(reuse::Extent::Variable)
        ) || !context
            .domtree
            .dominates(scope.block, change.entry, &context.func.layout)
        {
            continue;
        }
        if let Some(handoff) = prove(context, prefix_cert, prefix, cert, &change)
            && handoff.entry != scope.block
        {
            return Some(handoff);
        }
    }
    None
}

#[cfg(test)]
#[path = "wide_equal_prefix/tests.rs"]
mod tests;
