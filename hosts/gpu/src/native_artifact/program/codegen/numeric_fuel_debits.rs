//! Eliminate redundant hot child-total updates without changing any fuel check.
//! For each lane, A = fuel + children + root_count * root_price is invariant
//! between cold helpers. Child totals are A - fuel - root_count * root_price.
//! These are internal wrapping words: the identity holds modulo 2^64 even if
//! an intermediate anchor sum wraps. It reproduces the original word exactly;
//! the existing runtime still independently validates exact funded totals.
use crate::native_artifact::codegen::common::ssa_words::Words;
use crate::native_artifact::program::prepare::numeric_tree::Proof;
use cranelift_codegen::ir::{InstBuilder, types};
use cranelift_frontend::{FunctionBuilder, Variable};

struct Lane {
    anchor: Variable,
    fuel: usize,
    children: usize,
    root_price: u64,
}

pub(super) struct Anchors {
    root_count: usize,
    lanes: Vec<Lane>,
}

impl Anchors {
    pub(super) fn new(
        b: &mut FunctionBuilder<'_>,
        words: &Words,
        state: usize,
        shadow: usize,
        tree: &Proof,
    ) -> Self {
        let mut lanes = Vec::new();
        for (fuel, children, root_price, needed) in [
            (
                state + 1,
                tree.child_compute(),
                tree.nodes[0].compute,
                tree.nodes[1..].iter().any(|node| node.compute != 0),
            ),
            (
                state + 2,
                tree.child_comparison(),
                tree.nodes[0].comparison,
                tree.nodes[1..].iter().any(|node| node.comparison != 0),
            ),
        ] {
            if needed {
                lanes.push(Lane {
                    anchor: b.declare_var(types::I64),
                    fuel,
                    children: shadow + children,
                    root_price,
                });
            }
        }
        let anchors = Self {
            root_count: shadow + tree.traversals(0),
            lanes,
        };
        anchors.refresh(b, words);
        anchors
    }

    /// Call only after importing a helper's reconciled fuel and counters.
    /// Helper work and renewed loans are not charged as child traversals.
    pub(super) fn refresh(&self, b: &mut FunctionBuilder<'_>, words: &Words) {
        let roots = words.read(b, self.root_count);
        for lane in &self.lanes {
            let fuel = words.read(b, lane.fuel);
            let children = words.read(b, lane.children);
            let roots = b.ins().imul_imm(roots, lane.root_price as i64);
            let sum = b.ins().iadd(fuel, children);
            let anchor = b.ins().iadd(sum, roots);
            b.def_var(lane.anchor, anchor);
        }
    }

    /// Before every spill/publication/helper, reconstruct the exact original
    /// child words. This is idempotent; an error path may publish them again.
    pub(super) fn publish(&self, b: &mut FunctionBuilder<'_>, words: &Words) {
        let roots = words.read(b, self.root_count);
        for lane in &self.lanes {
            let anchor = b.use_var(lane.anchor);
            let fuel = words.read(b, lane.fuel);
            let roots = b.ins().imul_imm(roots, lane.root_price as i64);
            let spent = b.ins().isub(anchor, fuel);
            let children = b.ins().isub(spent, roots);
            words.write(b, lane.children, children);
        }
    }
}
