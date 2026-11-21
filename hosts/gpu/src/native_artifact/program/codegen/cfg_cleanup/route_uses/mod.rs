//! Immutable use inventory and stopped reachability for one route discovery.
//! All selected rewrites are applied only after this inventory is retired.
mod reach;

use cranelift_codegen::dominator_tree::DominatorTree;
use cranelift_codegen::ir::{Block, Function, Value};
use std::{collections::BTreeMap, sync::Arc};

pub(super) struct Uses {
    values: Vec<Vec<Block>>,
    reach: reach::Reachability,
}

impl Uses {
    pub fn new(f: &Function) -> Self {
        let mut values = vec![Vec::new(); f.dfg.num_values()];
        let blocks = f
            .layout
            .blocks()
            .map(|b| b.as_u32() as usize + 1)
            .max()
            .unwrap_or(0);
        let mut successors = vec![Vec::new(); blocks];
        for block in f.layout.blocks() {
            if let Some(last) = f.layout.last_inst(block) {
                successors[block.as_u32() as usize].extend(
                    f.dfg.insts[last]
                        .branch_destination(&f.dfg.jump_tables, &f.dfg.exception_tables)
                        .iter()
                        .map(|call| call.block(&f.dfg.value_lists).as_u32() as usize),
                );
            }
            for inst in f.layout.block_insts(block) {
                let mut add = |value: Value| {
                    let value = f.dfg.resolve_aliases(value);
                    let list = &mut values[value.as_u32() as usize];
                    if list.last() != Some(&block) {
                        list.push(block)
                    }
                };
                for &value in f.dfg.inst_args(inst) {
                    add(value)
                }
                for call in f.dfg.insts[inst]
                    .branch_destination(&f.dfg.jump_tables, &f.dfg.exception_tables)
                {
                    for arg in call.args(&f.dfg.value_lists) {
                        if let Some(value) = arg.as_value() {
                            add(value)
                        }
                    }
                }
            }
        }
        Self {
            values,
            reach: reach::Reachability::new(successors),
        }
    }

    pub fn live_outs(&self, target: Block) -> LiveOuts<'_> {
        LiveOuts {
            uses: self,
            target,
            reach: BTreeMap::new(),
        }
    }
}

// One target owns the per-definition-block rows. A fresh query cannot reuse
// another route's target, and the shared graph belongs to the same snapshot.
pub(super) struct LiveOuts<'a> {
    uses: &'a Uses,
    target: Block,
    reach: BTreeMap<Block, Arc<Vec<bool>>>,
}

impl LiveOuts<'_> {
    pub fn required(&mut self, dom: &DominatorTree, value: Value, owner: Block) -> Option<bool> {
        if owner == self.target {
            return Some(false);
        }
        let owner_dominates_target = dom.block_dominates(owner, self.target);
        let mut used = false;
        for &block in &self.uses.values[value.as_u32() as usize] {
            if block == owner || !dom.is_reachable(block) {
                continue;
            }
            if owner_dominates_target && dom.block_dominates(self.target, block) {
                // A simple entry-to-use path visits owner before target and
                // cannot visit owner again. Its suffix proves this use live.
                used = true;
                continue;
            }
            if !owner_dominates_target && dom.block_dominates(owner, block) {
                // A target-to-use path avoiding owner would contradict owner
                // dominance by joining an entry-to-target path avoiding it.
                continue;
            }
            let reachable = self.reach.entry(owner).or_insert_with(|| {
                self.uses
                    .reach
                    .reachable(self.target.as_u32() as usize, owner.as_u32() as usize)
            });
            if reachable[block.as_u32() as usize] {
                return None;
            }
        }
        Some(used)
    }
}
