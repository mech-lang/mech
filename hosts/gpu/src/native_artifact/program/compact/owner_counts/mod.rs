//! Fold reference-count loads only while the fresh backing's count is known.
//!
//! `Heap::allocate` initializes the first owner word to zero. Its checked,
//! nonempty result is the only seed; writable backing results may instead reuse
//! an owner and are not seeds. Unknown calls and potentially aliasing stores end the
//! proof. No ownership action, allocation, or failure edge is removed here.
use super::inlining::module_callee;
use cranelift_codegen::{
    Context,
    ir::{
        Block, Function, Inst, InstructionData as Data, MemFlags, Opcode, Value, ValueDef, types,
    },
};
use cranelift_module::{Linkage, Module};
use std::collections::{BTreeMap, BTreeSet};

fn allocation<M: Module>(module: &M, f: &Function, inst: Inst) -> bool {
    let Data::Call {
        opcode: Opcode::Call,
        func_ref,
        ..
    } = f.dfg.insts[inst]
    else {
        return false;
    };
    let Some(id) = module_callee(f, func_ref) else {
        return false;
    };
    let callee = module.declarations().get_function_decl(id);
    let signature = &f.dfg.signatures[f.dfg.ext_funcs[func_ref].signature];
    callee.linkage == Linkage::Import
        && callee.name.as_deref() == Some("mech_compact_allocate_backing")
        && &callee.signature == signature
        && signature.params.len() == 5
        && signature.returns.len() == 2
        && signature
            .params
            .iter()
            .chain(&signature.returns)
            .all(|p| p.value_type == types::I64)
}

fn positive_literal(f: &Function, value: Value) -> bool {
    let value = f.dfg.resolve_aliases(value);
    let ValueDef::Result(inst, 0) = f.dfg.value_def(value) else {
        return false;
    };
    f.dfg.value_type(value) == types::I64
        && matches!(f.dfg.insts[inst], Data::UnaryImm { opcode: Opcode::Iconst, imm } if imm.bits() != 0)
}

#[derive(Clone, Copy)]
struct Seed {
    allocation: Inst,
    success: Block,
}

fn seeds<M: Module>(module: &M, context: &Context) -> BTreeMap<Value, Seed> {
    let f = &context.func;
    let mut seeds = BTreeMap::new();
    for block in f
        .layout
        .blocks()
        .filter(|&b| context.domtree.is_reachable(b))
    {
        let Some(branch) = f.layout.last_inst(block) else {
            continue;
        };
        let Data::Brif {
            arg,
            blocks: [yes, no],
            ..
        } = f.dfg.insts[branch]
        else {
            continue;
        };
        let success = yes.block(&f.dfg.value_lists);
        if success == no.block(&f.dfg.value_lists)
            || !context.cfg.pred_iter(success).map(|p| p.inst).eq([branch])
        {
            continue;
        }
        let ValueDef::Result(call, 0) = f.dfg.value_def(f.dfg.resolve_aliases(arg)) else {
            continue;
        };
        if !allocation(module, f, call) {
            continue;
        }
        let [_, capacity, stride, _, _] = f.dfg.inst_args(call) else {
            continue;
        };
        let [_, owner] = f.dfg.inst_results(call) else {
            continue;
        };
        if positive_literal(f, *capacity) && positive_literal(f, *stride) {
            seeds.insert(
                *owner,
                Seed {
                    allocation: call,
                    success,
                },
            );
        }
    }
    seeds
}

type Point = (Block, Option<Inst>);

struct Query<'a, M> {
    module: &'a M,
    context: &'a Context,
    owner: Value,
    seed: Seed,
    remaining: usize,
    active: BTreeSet<Point>,
    memo: BTreeMap<Point, Option<u64>>,
}

impl<M: Module> Query<'_, M> {
    fn integer(&mut self, value: Value, depth: usize) -> Option<u64> {
        self.remaining = self.remaining.checked_sub(1)?;
        if depth == 16 {
            return None;
        }
        let f = &self.context.func;
        let value = f.dfg.resolve_aliases(value);
        if f.dfg.value_type(value) != types::I64 {
            return None;
        }
        let ValueDef::Result(inst, 0) = f.dfg.value_def(value) else {
            return None;
        };
        match f.dfg.insts[inst] {
            Data::UnaryImm {
                opcode: Opcode::Iconst,
                imm,
            } => Some(imm.bits() as u64),
            Data::BinaryImm64 {
                opcode: Opcode::IaddImm,
                arg,
                imm,
            } => Some(
                self.integer(arg, depth + 1)?
                    .wrapping_add(imm.bits() as u64),
            ),
            _ => None,
        }
    }

    fn before(&mut self, point: Point, depth: usize) -> Option<u64> {
        if depth == 128 {
            return None;
        }
        if let Some(result) = self.memo.get(&point) {
            return *result;
        }
        if !self.active.insert(point) {
            return None;
        }
        let result = self.scan(point, depth);
        self.active.remove(&point);
        self.memo.insert(point, result);
        result
    }

    fn scan(&mut self, (block, mut cursor): Point, depth: usize) -> Option<u64> {
        while let Some(inst) = cursor {
            self.remaining = self.remaining.checked_sub(1)?;
            if inst == self.seed.allocation {
                return Some(0);
            }
            let f = &self.context.func;
            let data = f.dfg.insts[inst];
            let opcode = data.opcode();
            if let Data::Store {
                opcode: Opcode::Store,
                flags,
                args: [value, address],
                offset,
            } = data
                && flags == MemFlags::new()
                && offset == 0.into()
                && f.dfg.resolve_aliases(address) == self.owner
            {
                return self.integer(value, 0);
            }
            if opcode.can_store() {
                // Even a syntactically different pointer can alias this word.
                // Stack stores also remain barriers in this first proof.
                return None;
            }
            if opcode.is_call() {
                // Another allocation can update the heap registry, but does
                // not mutate any live owner's reference count or free it.
                if !allocation(self.module, f, inst) {
                    return None;
                }
            } else if opcode.other_side_effects() && !opcode.is_branch() {
                return None;
            }
            cursor = f.layout.prev_inst(inst);
        }
        let predecessors = self
            .context
            .cfg
            .pred_iter(block)
            .filter(|p| self.context.domtree.is_reachable(p.block))
            .map(|p| (p.block, Some(p.inst)))
            .collect::<Vec<_>>();
        let mut known = None;
        for predecessor in predecessors {
            let value = self.before(predecessor, depth + 1)?;
            if known.is_some_and(|earlier| earlier != value) {
                return None;
            }
            known = Some(value);
        }
        known
    }
}

pub(super) fn fold<M: Module>(module: &M, context: &mut Context) -> (usize, usize) {
    context.flowgraph();
    let seeds = seeds(module, context);
    let f = &context.func;
    let loads = f
        .layout
        .blocks()
        .filter(|&b| context.domtree.is_reachable(b))
        .flat_map(|b| f.layout.block_insts(b).map(move |inst| (b, inst)))
        .filter_map(|(block, inst)| {
            let Data::Load {
                opcode: Opcode::Load,
                flags,
                arg,
                offset,
            } = f.dfg.insts[inst]
            else {
                return None;
            };
            let owner = f.dfg.resolve_aliases(arg);
            let seed = *seeds.get(&owner)?;
            (flags == MemFlags::new()
                && offset == 0.into()
                && f.dfg.value_type(f.dfg.first_result(inst)) == types::I64
                && context.domtree.block_dominates(seed.success, block))
            .then_some((block, inst, owner, seed))
        })
        .collect::<Vec<_>>();
    let mut folded = 0;
    for (block, inst, owner, seed) in loads {
        let mut query = Query {
            module,
            context,
            owner,
            seed,
            remaining: 4096,
            active: BTreeSet::new(),
            memo: BTreeMap::new(),
        };
        let point = (block, query.context.func.layout.prev_inst(inst));
        if let Some(count) = query.before(point, 0) {
            context.func.dfg.insts[inst] = Data::UnaryImm {
                opcode: Opcode::Iconst,
                imm: (count as i64).into(),
            };
            folded += 1;
        }
    }
    (seeds.len(), folded)
}

#[cfg(all(
    test,
    target_family = "unix",
    any(target_arch = "aarch64", target_arch = "x86_64")
))]
#[path = "tests.rs"]
mod tests;
