//! Split the packet values used exclusively by the ordinary recurrence from
//! the short append run. The caller owns a disjoint, non-escaping input packet.
use super::{Step, Trace};
use cranelift_codegen::Context;
use cranelift_codegen::cursor::FuncCursor;
use cranelift_codegen::ir::{Block, InstBuilder, InstructionData, MemFlags, Opcode, Value, types};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy)]
struct Word {
    value: Value,
    flags: MemFlags,
    offset: i32,
    used_by_run: bool,
}

pub(super) struct Reloads {
    input: Value,
    words: Vec<Word>,
}

impl Reloads {
    pub(super) fn count(&self) -> usize {
        self.words.len()
    }

    pub(super) fn for_run(&self, cursor: &mut FuncCursor<'_>) -> Vec<(Value, Value)> {
        self.words
            .iter()
            .filter(|word| word.used_by_run)
            .map(|word| {
                let value = cursor
                    .ins()
                    .load(types::I64, word.flags, self.input, word.offset);
                (word.value, value)
            })
            .collect()
    }

    pub(super) fn emit(&self, cursor: &mut FuncCursor<'_>) -> Vec<Value> {
        self.words
            .iter()
            .map(|word| {
                cursor
                    .ins()
                    .load(types::I64, word.flags, self.input, word.offset)
            })
            .collect()
    }
}

pub(super) fn split(
    context: &mut Context,
    header: Block,
    trace: &Trace,
    packet_words: usize,
    split_shared: bool,
) -> Option<Reloads> {
    let f = &context.func;
    let entry = f.layout.entry_block()?;
    let input = *f.dfg.block_params(entry).get(1)?;
    let header_start = f.layout.first_inst(header)?;
    let mut kept = BTreeSet::from([trace.selector, trace.stored]);
    kept.extend(&trace.next);
    for check in &trace.seed_checks {
        kept.extend(check.values());
    }
    let mut run_uses = BTreeSet::new();
    for step in &trace.steps {
        match step {
            Step::Copy { args, .. } => run_uses.extend(args.iter().copied()),
            Step::Guard(check) => run_uses.extend(check.values()),
        }
    }

    let mut inside = Vec::new();
    let mut outside_uses = BTreeSet::new();
    let mut inside_uses = BTreeSet::new();
    for block in f.layout.blocks() {
        if context.domtree.dominates(header, block, &f.layout) {
            for inst in f.layout.block_insts(block) {
                inside_uses.extend(f.dfg.inst_values(inst).map(|v| f.dfg.resolve_aliases(v)));
                inside.push(inst);
            }
        } else {
            for inst in f.layout.block_insts(block) {
                outside_uses.extend(f.dfg.inst_values(inst).map(|v| f.dfg.resolve_aliases(v)));
            }
        }
    }
    let words = f
        .layout
        .blocks()
        .flat_map(|block| f.layout.block_insts(block))
        .filter_map(|inst| {
            let InstructionData::Load {
                opcode: Opcode::Load,
                arg,
                flags,
                offset,
            } = f.dfg.insts[inst]
            else {
                return None;
            };
            let offset = i32::from(offset);
            let value = f.dfg.first_result(inst);
            if f.dfg.resolve_aliases(arg) != input
                || !context.domtree.dominates(inst, header_start, &f.layout)
                || f.dfg.value_type(value) != types::I64
                || offset < 0
                || offset % 8 != 0
                || offset as usize / 8 >= packet_words
                || kept.contains(&value)
                || (!split_shared && run_uses.contains(&value))
                || outside_uses.contains(&value)
                || !inside_uses.contains(&value)
            {
                return None;
            }
            Some(Word {
                value,
                flags,
                offset,
                used_by_run: run_uses.contains(&value),
            })
        })
        .collect::<Vec<_>>();
    if words.is_empty() {
        return None;
    }

    let predecessors = context
        .cfg
        .pred_iter(header)
        .map(|pred| {
            (
                pred.inst,
                context.domtree.dominates(header, pred.inst, &f.layout),
            )
        })
        .collect::<Vec<_>>();
    let f = &mut context.func;
    let replacement = words
        .iter()
        .map(|word| (word.value, f.dfg.append_block_param(header, types::I64)))
        .collect::<BTreeMap<_, _>>();
    for inst in inside {
        let operands = f
            .dfg
            .inst_values(inst)
            .map(|v| {
                replacement
                    .get(&f.dfg.resolve_aliases(v))
                    .copied()
                    .unwrap_or(v)
            })
            .collect::<Vec<_>>();
        f.dfg.overwrite_inst_values(inst, operands.into_iter());
    }
    for (inst, within) in predecessors {
        let dfg = &mut f.dfg;
        for edge in
            dfg.insts[inst].branch_destination_mut(&mut dfg.jump_tables, &mut dfg.exception_tables)
        {
            if edge.block(&dfg.value_lists) == header {
                for word in &words {
                    let value = if within {
                        replacement[&word.value]
                    } else {
                        word.value
                    };
                    edge.append_argument(value, &mut dfg.value_lists);
                }
            }
        }
    }
    Some(Reloads { input, words })
}
