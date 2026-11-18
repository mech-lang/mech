//! Observe the physical word homes used by compact checked lowering.

use super::{Home, Words};
use cranelift_codegen::{
    dominator_tree::DominatorTree,
    flowgraph::ControlFlowGraph,
    ir::{Block, Function, Value},
};
use cranelift_frontend::{FunctionBuilder, Variable};
use std::collections::BTreeSet;

pub(super) struct Tracked {
    slot: usize,
    variable: Variable,
    definition: Option<(Block, Value)>,
    writes: usize,
    early_read: bool,
    reads: Vec<Block>,
}

impl Tracked {
    pub(super) fn read(&mut self, b: &mut FunctionBuilder<'_>, direct: bool) -> Value {
        self.reads
            .push(b.current_block().expect("read has a native block"));
        if let Some((_, value)) = self.definition {
            if direct {
                return value;
            }
        } else {
            self.early_read = true;
        }
        b.use_var(self.variable)
    }

    pub(super) fn write(&mut self, b: &mut FunctionBuilder<'_>, value: Value) {
        if self.definition.is_none() {
            let block = b.current_block().expect("definition has a native block");
            self.definition = Some((block, value));
        }
        self.writes += 1;
        b.def_var(self.variable, value);
    }
}

impl Words {
    pub(in crate::native_artifact) fn track_direct_homes(
        &mut self,
        slots: &BTreeSet<usize>,
        direct: bool,
    ) {
        self.promote_tracked = direct;
        let tracked = self.tracked.get_mut();
        for &slot in slots {
            let (home, _) = self.variables.get_mut(&slot).expect("known source home");
            let Home::Variable(variable) = *home else {
                unreachable!("direct definition needs an ordinary scalar home");
            };
            let index = u32::try_from(tracked.len()).expect("native home layout fits u32");
            tracked.push(Tracked {
                slot,
                variable,
                definition: None,
                writes: 0,
                early_read: false,
                reads: Vec::new(),
            });
            *home = Home::Tracked(index);
        }
    }

    pub(in crate::native_artifact) fn prove_tracked_homes(
        &self,
        function: &Function,
    ) -> Vec<HomeProof> {
        let tracked = self.tracked.borrow();
        if tracked.is_empty() {
            return Vec::new();
        }
        let cfg = ControlFlowGraph::with_function(function);
        let dom = DominatorTree::with_function(function, &cfg);
        tracked
            .iter()
            .map(|home| {
                let single_def = home.definition.filter(|_| home.writes == 1);
                let blocked_reads = single_def.map_or(home.reads.len(), |(from, _)| {
                    home.reads
                        .iter()
                        .filter(|&&to| {
                            !function.layout.is_block_inserted(to)
                                || !dom.is_reachable(to)
                                || !dom.dominates(from, to, &function.layout)
                        })
                        .count()
                });
                HomeProof {
                    slot: home.slot,
                    definition: single_def.map(|(block, _)| block.as_u32()),
                    reads: home.reads.len(),
                    writes: home.writes,
                    early_read: home.early_read,
                    blocked_reads,
                    admitted: home.writes == 1
                        && !home.early_read
                        && blocked_reads == 0
                        && !home.reads.is_empty(),
                }
            })
            .collect()
    }
}

#[derive(serde::Serialize)]
pub(in crate::native_artifact) struct HomeProof {
    pub(in crate::native_artifact) slot: usize,
    pub(in crate::native_artifact) admitted: bool,
    definition: Option<u32>,
    reads: usize,
    writes: usize,
    early_read: bool,
    blocked_reads: usize,
}
