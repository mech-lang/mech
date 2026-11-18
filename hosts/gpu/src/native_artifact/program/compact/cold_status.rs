//! Share the materialization of a fixed status on cold checked returns.

use cranelift_codegen::cursor::{Cursor, FuncCursor};
use cranelift_codegen::flowgraph::ControlFlowGraph;
use cranelift_codegen::ir::{
    Block, Function, Inst, InstBuilder, InstructionData as Data, Opcode, Value, ValueDef, types,
};
use std::collections::BTreeMap;

fn constant(f: &Function, value: Value) -> Option<i64> {
    let value = f.dfg.resolve_aliases(value);
    let ValueDef::Result(inst, 0) = f.dfg.value_def(value) else {
        return None;
    };
    let Data::UnaryImm {
        opcode: Opcode::Iconst,
        imm,
        ..
    } = f.dfg.insts[inst]
    else {
        return None;
    };
    (f.dfg.value_type(value) == types::I64).then_some(imm.bits())
}

fn status(f: &Function, cfg: &ControlFlowGraph, block: Block, value: Value) -> Option<i64> {
    if let Some(status) = constant(f, value) {
        return Some(status);
    }
    let ValueDef::Param(owner, ordinal) = f.dfg.value_def(f.dfg.resolve_aliases(value)) else {
        return None;
    };
    if owner != block {
        return None;
    }
    let mut common = None;
    for predecessor in cfg.pred_iter(block) {
        for edge in f.dfg.insts[predecessor.inst]
            .branch_destination(&f.dfg.jump_tables, &f.dfg.exception_tables)
        {
            if edge.block(&f.dfg.value_lists) != block {
                continue;
            }
            let value = edge.args(&f.dfg.value_lists).nth(ordinal)?.as_value()?;
            let status = constant(f, value)?;
            if common.is_some_and(|expected| expected != status) {
                return None;
            }
            common = Some(status);
        }
    }
    common
}

pub(super) fn share(f: &mut Function, cfg: &ControlFlowGraph) -> usize {
    let mut groups = BTreeMap::<i64, Vec<(Inst, Value)>>::new();
    for block in f.layout.blocks().filter(|&block| f.layout.is_cold(block)) {
        let Some(ret) = f.layout.last_inst(block) else {
            continue;
        };
        if f.dfg.insts[ret].opcode() != Opcode::Return {
            continue;
        }
        let &[code, origin] = f.dfg.inst_args(ret) else {
            continue;
        };
        if constant(f, origin).is_none() {
            continue;
        }
        if let Some(code) = status(f, cfg, block, code).filter(|&code| code != 0) {
            groups.entry(code).or_default().push((ret, origin));
        }
    }
    let mut shared = 0;
    for (code, returns) in groups.into_iter().filter(|(_, sites)| sites.len() > 2) {
        let exit = f.dfg.make_block();
        f.layout.append_block(exit);
        f.layout.set_cold(exit);
        let origin = f.dfg.append_block_param(exit, types::I64);
        let mut cursor = FuncCursor::new(f).at_bottom(exit);
        let code = cursor.ins().iconst(types::I64, code);
        cursor.ins().return_(&[code, origin]);
        for (ret, origin) in &returns {
            f.dfg.replace(*ret).jump(exit, &[(*origin).into()]);
        }
        shared += returns.len();
    }
    shared
}
