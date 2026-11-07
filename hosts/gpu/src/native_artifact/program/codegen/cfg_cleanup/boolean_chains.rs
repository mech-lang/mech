//! Branch on existing canonical predicates without materializing their chain.
//!
//! Every operand has already executed before the original terminator. Only
//! that terminator and its pure Boolean combination are changed: no source
//! operation, work debit, load, call or publication moves across a branch.
//! Raw words and noncanonical loop entries/backedges never grant permission.
use cranelift_codegen::cursor::{Cursor, FuncCursor};
use cranelift_codegen::ir::{
    Block, BlockArg, Function, Inst, InstBuilder, InstructionData as D, Opcode as O, Value,
    ValueDef,
};

enum Tree {
    Leaf(Value),
    And(Box<Tree>, Box<Tree>),
    Or(Box<Tree>, Box<Tree>),
}
impl Tree {
    fn leaves(&self) -> usize {
        match self {
            Self::Leaf(_) => 1,
            Self::And(a, b) | Self::Or(a, b) => a.leaves() + b.leaves(),
        }
    }
}

fn tree(f: &Function, value: Value, known: &[bool], depth: usize) -> Tree {
    let value = f.dfg.resolve_aliases(value);
    let ValueDef::Result(inst, 0) = f.dfg.value_def(value) else {
        return Tree::Leaf(value);
    };
    if depth == 0 || !known[value.as_u32() as usize] {
        return Tree::Leaf(value);
    }
    match f.dfg.insts[inst] {
        D::Unary {
            opcode: O::Uextend | O::Sextend | O::Ireduce,
            arg,
        } if known[f.dfg.resolve_aliases(arg).as_u32() as usize] => tree(f, arg, known, depth - 1),
        D::BinaryImm64 {
            opcode: O::BandImm,
            arg,
            imm,
        } if imm.bits() == 1 && known[f.dfg.resolve_aliases(arg).as_u32() as usize] => {
            tree(f, arg, known, depth - 1)
        }
        D::Binary {
            opcode,
            args: [a, b],
        } if matches!(opcode, O::Band | O::Bor)
            && [a, b]
                .iter()
                .all(|&v| known[f.dfg.resolve_aliases(v).as_u32() as usize]) =>
        {
            let a = Box::new(tree(f, a, known, depth - 1));
            let b = Box::new(tree(f, b, known, depth - 1));
            if opcode == O::Band {
                Tree::And(a, b)
            } else {
                Tree::Or(a, b)
            }
        }
        _ => Tree::Leaf(value),
    }
}

#[derive(Clone)]
struct Target {
    block: Block,
    args: Vec<BlockArg>,
}

fn emit(f: &mut Function, inst: Inst, t: Tree, yes: Target, no: Target) {
    let and = matches!(&t, Tree::And(..));
    match t {
        Tree::Leaf(value) => {
            f.dfg
                .replace(inst)
                .brif(value, yes.block, &yes.args, no.block, &no.args);
        }
        Tree::And(a, b) | Tree::Or(a, b) => {
            let block = f.dfg.make_block();
            let after = f.layout.inst_block(inst).unwrap();
            f.layout.insert_block_after(block, after);
            if f.layout.is_cold(after) {
                f.layout.set_cold(block);
            }
            let next = FuncCursor::new(f)
                .at_bottom(block)
                .ins()
                .jump(no.block, &no.args);
            emit(f, next, *b, yes.clone(), no.clone());
            let target = Target {
                block,
                args: Vec::new(),
            };
            if and {
                emit(f, inst, *a, target, no);
            } else {
                emit(f, inst, *a, yes, target);
            }
        }
    }
}

pub(super) fn expand(f: &mut Function) -> usize {
    let Some(known) = super::boolean_conditions::canonical_values(f) else {
        return 0;
    };
    let mut extra_blocks = 0;
    let candidates = f
        .layout
        .blocks()
        .filter_map(|block| {
            let inst = f.layout.last_inst(block)?;
            let D::Brif { arg, blocks, .. } = f.dfg.insts[inst] else {
                return None;
            };
            // Let the existing constant-fold/reachability sweep retire these
            // directly; distribution would only retain extra jump blocks.
            if super::constant_condition(f, arg).is_some() {
                return None;
            }
            let t = tree(f, arg, &known, 8);
            if !(2..=6).contains(&t.leaves()) {
                return None;
            }
            let targets = blocks.map(|call| Target {
                block: call.block(&f.dfg.value_lists),
                args: call.args(&f.dfg.value_lists).collect(),
            });
            if targets
                .iter()
                .flat_map(|t| &t.args)
                .any(|arg| arg.as_value().is_none())
            {
                return None;
            }
            extra_blocks += t.leaves() - 1;
            if extra_blocks > 4096 {
                return None;
            }
            Some((inst, t, targets))
        })
        .take(1024)
        .collect::<Vec<_>>();
    let count = candidates.len();
    for (inst, t, [yes, no]) in candidates {
        emit(f, inst, t, yes, no);
    }
    count
}

#[cfg(test)]
mod tests;
