use cranelift_codegen::cursor::{Cursor, FuncCursor};
use cranelift_codegen::dominator_tree::DominatorTree;
use cranelift_codegen::ir::{
    Function, InstBuilder, InstructionData, Opcode, Type, Value, ValueDef,
};
use std::collections::{BTreeMap, BTreeSet};

/// Give identical integer literals on ordinary incoming edges a common SSA
/// definition so the existing constant-phi pass can recognize them.
pub(in super::super) fn coalesce(function: &mut Function, dom: &DominatorTree) -> usize {
    let Some(entry) = function.layout.entry_block() else {
        return 0;
    };
    if function.dfg.values_labels.is_some() {
        return 0;
    }

    let mut branches = Vec::new();
    let mut literals = BTreeMap::<(u32, u64), (Type, BTreeSet<Value>)>::new();
    let dfg = &function.dfg;
    for &block in dom.cfg_rpo() {
        let Some(inst) = function.layout.last_inst(block) else {
            continue;
        };
        let data = &dfg.insts[inst];
        if !matches!(data.opcode(), Opcode::Jump | Opcode::Brif) {
            continue;
        }
        let mut has_literal = false;
        for call in data.branch_destination(&dfg.jump_tables, &dfg.exception_tables) {
            for arg in call.args(&dfg.value_lists) {
                let Some(value) = arg.as_value() else {
                    continue;
                };
                let ValueDef::Result(def, 0) = dfg.value_def(value) else {
                    continue;
                };
                let InstructionData::UnaryImm {
                    opcode: Opcode::Iconst,
                    imm,
                } = dfg.insts[def]
                else {
                    continue;
                };
                let ty = dfg.value_type(value);
                let bits = ty.bits();
                if !ty.is_int() || bits > 64 {
                    continue;
                }
                let word = imm.bits() as u64 & (u64::MAX >> (64 - bits));
                literals
                    .entry((bits, word))
                    .or_insert_with(|| (ty, BTreeSet::new()))
                    .1
                    .insert(value);
                has_literal = true;
            }
        }
        if has_literal {
            branches.push(inst);
        }
    }

    let mut replacements = BTreeMap::new();
    let mut cursor = FuncCursor::new(function).at_first_insertion_point(entry);
    for ((_, word), (ty, values)) in literals {
        if values.len() > 1 {
            let literal = cursor.ins().iconst(ty, word as i64);
            replacements.extend(values.into_iter().map(|value| (value, literal)));
        }
    }
    if replacements.is_empty() {
        return 0;
    }

    let dfg = &mut function.dfg;
    for inst in branches {
        let calls =
            dfg.insts[inst].branch_destination_mut(&mut dfg.jump_tables, &mut dfg.exception_tables);
        for call in calls {
            call.update_args(&mut dfg.value_lists, |arg| {
                arg.map_value(|value| replacements.get(&value).copied().unwrap_or(value))
            });
        }
    }
    replacements.len()
}
