//! Express a branch-only stop predicate as the negated conjunction.

use cranelift_codegen::cursor::{Cursor, FuncCursor};
use cranelift_codegen::dominator_tree::DominatorTree;
use cranelift_codegen::ir::condcodes::{CondCode, IntCC};
use cranelift_codegen::ir::{
    Function, Inst, InstBuilder, InstructionData as Data, Opcode, Value, ValueDef, types,
};
use std::collections::BTreeMap;

struct Site {
    mask: Inst,
    branch: Inst,
    words: [Value; 6],
    predicates: [(IntCC, [Value; 2]); 2],
}

fn comparison(f: &Function, word: Value) -> Option<(Value, IntCC, [Value; 2])> {
    let ValueDef::Result(extension, 0) = f.dfg.value_def(word) else {
        return None;
    };
    let Data::Unary {
        opcode: Opcode::Uextend,
        arg,
    } = f.dfg.insts[extension]
    else {
        return None;
    };
    let arg = f.dfg.resolve_aliases(arg);
    let ValueDef::Result(test, 0) = f.dfg.value_def(arg) else {
        return None;
    };
    let Data::IntCompare { cond, args, .. } = f.dfg.insts[test] else {
        return None;
    };
    (f.dfg.value_type(arg) == types::I8
        && args
            .iter()
            .all(|&value| f.dfg.value_type(value) == types::I64))
    .then(|| {
        (
            arg,
            cond.complement(),
            args.map(|v| f.dfg.resolve_aliases(v)),
        )
    })
}

fn jump_destination(
    f: &Function,
    mut call: cranelift_codegen::ir::BlockCall,
) -> Option<(cranelift_codegen::ir::Block, Vec<Value>)> {
    for _ in 0..96 {
        let args = call
            .args(&f.dfg.value_lists)
            .map(|arg| Some(f.dfg.resolve_aliases(arg.as_value()?)))
            .collect::<Option<Vec<_>>>()?;
        let block = call.block(&f.dfg.value_lists);
        if !args.is_empty() {
            return Some((block, args));
        }
        if !f.dfg.block_params(block).is_empty() {
            return None;
        }
        let end = f.layout.last_inst(block)?;
        let Data::Jump { destination, .. } = f.dfg.insts[end] else {
            return None;
        };
        call = destination;
    }
    None
}

fn candidate(f: &Function, dom: &DominatorTree, branch: Inst) -> Option<Site> {
    let Data::Brif {
        arg,
        blocks: [stop, walk],
        ..
    } = f.dfg.insts[branch]
    else {
        return None;
    };
    let ValueDef::Result(mask, 0) = f.dfg.value_def(f.dfg.resolve_aliases(arg)) else {
        return None;
    };
    let Data::BinaryImm64 {
        opcode: Opcode::BandImm,
        arg,
        imm,
    } = f.dfg.insts[mask]
    else {
        return None;
    };
    if imm.bits() != 1
        || f.layout.next_inst(mask) != Some(branch)
        || !dom.is_reachable(f.layout.inst_block(branch)?)
    {
        return None;
    }
    let word = f.dfg.resolve_aliases(arg);
    let ValueDef::Result(combine, 0) = f.dfg.value_def(word) else {
        return None;
    };
    let Data::Binary {
        opcode: Opcode::Bor,
        args,
    } = f.dfg.insts[combine]
    else {
        return None;
    };
    let [left, right] = args.map(|v| f.dfg.resolve_aliases(v));
    // Keep this scheduling change on loop decisions that forward a value
    // into both the continuing computation and its typed completion.
    let (back, carried) = jump_destination(f, walk)?;
    let (done, returned) = jump_destination(f, stop)?;
    let current = f.layout.inst_block(branch)?;
    if !dom.block_dominates(back, current)
        || dom.block_dominates(done, current)
        || !carried.iter().any(|value| returned.contains(value))
    {
        return None;
    }
    let (a, left_cc, left_args) = comparison(f, left)?;
    let (b, right_cc, right_args) = comparison(f, right)?;
    Some(Site {
        mask,
        branch,
        words: [word, f.dfg.first_result(mask), left, right, a, b],
        predicates: [(left_cc, left_args), (right_cc, right_args)],
    })
}

pub(in crate::native_artifact) fn optimize(f: &mut Function, dom: &DominatorTree) -> usize {
    let plans = f
        .layout
        .blocks()
        .filter_map(|block| candidate(f, dom, f.layout.last_inst(block)?))
        .collect::<Vec<_>>();
    if plans.is_empty() {
        return 0;
    }
    let mut uses = BTreeMap::new();
    for plan in &plans {
        for word in plan.words {
            uses.insert(word, 0);
        }
    }
    for inst in f
        .layout
        .blocks()
        .flat_map(|block| f.layout.block_insts(block))
    {
        for value in f.dfg.inst_values(inst) {
            if let Some(count) = uses.get_mut(&f.dfg.resolve_aliases(value)) {
                *count += 1;
            }
        }
    }
    let mut changed = 0;
    for plan in plans {
        if !plan.words.iter().all(|word| uses[word] == 1) {
            continue;
        }
        let Data::Brif { blocks, .. } = f.dfg.insts[plan.branch] else {
            unreachable!()
        };
        let [yes, no] = blocks;
        let [left, right] = {
            let mut c = FuncCursor::new(f).at_inst(plan.mask);
            plan.predicates
                .map(|(cond, [a, b])| c.ins().icmp(cond, a, b))
        };
        let flag = FuncCursor::new(f)
            .at_inst(plan.branch)
            .ins()
            .band(left, right);
        f.dfg.insts[plan.branch] = Data::Brif {
            opcode: Opcode::Brif,
            arg: flag,
            blocks: [no, yes],
        };
        f.dfg.detach_inst_results(plan.mask);
        f.layout.remove_inst(plan.mask);
        changed += 1;
    }
    changed
}

#[cfg(all(test, feature = "jit"))]
#[path = "branch_form_tests.rs"]
mod tests;
