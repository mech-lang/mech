/*
 * Released under the terms of the Apache 2.0 license with LLVM
 * exception. See `LICENSE` for details.
 */

//! Lightweight CFG analyses.

use crate::alloc::vec::Vec;

use crate::{domtree, postorder, Block, Function, Inst, ProgPoint, RegAllocError, VecExt};
use smallvec::{smallvec, SmallVec};

#[derive(Debug, Default)]
pub struct CFGInfoCtx {
    visited: Vec<bool>,
    block_to_rpo: Vec<Option<u32>>,
    backedge: Vec<u32>,
}

#[derive(Debug, Default)]
pub struct CFGInfo {
    /// Postorder traversal of blocks.
    pub postorder: Vec<Block>,
    /// Domtree parents, indexed by block.
    pub domtree: Vec<Block>,
    /// For each instruction, the block it belongs to.
    pub insn_block: Vec<Block>,
    /// For each block, the first instruction.
    pub block_entry: Vec<ProgPoint>,
    /// For each block, the last instruction.
    pub block_exit: Vec<ProgPoint>,
    /// For each block, what is the approximate loop depth?
    ///
    /// This measure is fully precise iff the input CFG is reducible
    /// and blocks are in RPO, so that loop backedges are precisely
    /// those whose block target indices are less than their source
    /// indices. Otherwise, it will be approximate, but should still
    /// be usable for heuristic purposes.
    pub approx_loop_depth: Vec<u32>,
}

impl CFGInfo {
    pub fn new<F: Function>(f: &F) -> Result<Self, RegAllocError> {
        let mut ctx = CFGInfoCtx::default();
        let mut this = Self::default();
        this.init(f, &mut ctx)?;
        Ok(this)
    }

    pub fn init<F: Function>(&mut self, f: &F, ctx: &mut CFGInfoCtx) -> Result<(), RegAllocError> {
        let nb = f.num_blocks();

        postorder::calculate(
            nb,
            f.entry_block(),
            &mut ctx.visited,
            &mut self.postorder,
            |block| f.block_succs(block),
        )?;

        domtree::calculate(
            nb,
            |block| f.block_preds(block),
            &self.postorder,
            &mut ctx.block_to_rpo,
            &mut self.domtree,
            f.entry_block(),
        );

        let insn_block = self.insn_block.repopulate(f.num_insts(), Block::invalid());
        let block_entry = self
            .block_entry
            .repopulate(nb, ProgPoint::before(Inst::invalid()));
        let block_exit = self
            .block_exit
            .repopulate(nb, ProgPoint::before(Inst::invalid()));
        let (backedge_in, backedge_out) = ctx.backedge.repopulate(nb * 2, 0).split_at_mut(nb);

        for block in 0..f.num_blocks() {
            let block = Block::new(block);
            for inst in f.block_insns(block).iter() {
                insn_block[inst.index()] = block;
            }
            block_entry[block.index()] = ProgPoint::before(f.block_insns(block).first());
            block_exit[block.index()] = ProgPoint::after(f.block_insns(block).last());

            // Check critical edge condition: if there is more than
            // one predecessor, each must have only one successor
            // (this block).
            let preds = f.block_preds(block).len() + if block == f.entry_block() { 1 } else { 0 };
            if preds > 1 {
                for &pred in f.block_preds(block) {
                    let succs = f.block_succs(pred).len();
                    if succs > 1 {
                        return Err(RegAllocError::CritEdge(pred, block));
                    }
                }
            }

            // Check branch-arg condition: if any successors have more
            // than one predecessor (given above, there will only be
            // one such successor), then the last instruction of this
            // block (the branch) cannot have any args other than the
            // blockparams.
            let mut require_no_branch_args = false;
            for &succ in f.block_succs(block) {
                let preds = f.block_preds(succ).len() + if succ == f.entry_block() { 1 } else { 0 };
                if preds > 1 {
                    require_no_branch_args = true;
                    break;
                }
            }
            if require_no_branch_args {
                let last = f.block_insns(block).last();
                if !f.inst_operands(last).is_empty() {
                    return Err(RegAllocError::DisallowedBranchArg(last));
                }
            }

            for &succ in f.block_succs(block) {
                if succ.index() <= block.index() {
                    backedge_in[succ.index()] += 1;
                    backedge_out[block.index()] += 1;
                }
            }
        }

        if self.use_natural_loop_depths(f, &ctx.block_to_rpo) {
            return Ok(());
        }

        let approx_loop_depth = self.approx_loop_depth.cleared();
        let mut backedge_stack: SmallVec<[u32; 4]> = smallvec![];
        let mut cur_depth = 0;
        for block in 0..nb {
            if backedge_in[block] > 0 {
                cur_depth += 1;
                backedge_stack.push(backedge_in[block]);
            }

            approx_loop_depth.push(cur_depth);

            while backedge_stack.len() > 0 && backedge_out[block] > 0 {
                backedge_out[block] -= 1;
                *backedge_stack.last_mut().unwrap() -= 1;
                if *backedge_stack.last().unwrap() == 0 {
                    cur_depth -= 1;
                    backedge_stack.pop();
                }
            }
        }

        Ok(())
    }

    // Count actual natural-loop membership for reducible graphs. The layout
    // approximation is still useful when a cyclic region has multiple entries.
    fn use_natural_loop_depths<F: Function>(&mut self, f: &F, rpo: &[Option<u32>]) -> bool {
        let mut latches = Vec::new();
        for &block in &self.postorder {
            for &header in f.block_succs(block) {
                if rpo[header.index()] > rpo[block.index()] {
                    continue;
                }
                if !self.dominates(header, block) {
                    return false;
                }
                latches.push((header, block));
            }
        }

        let depths = self.approx_loop_depth.repopulate(f.num_blocks(), 0);
        if latches.is_empty() {
            return true;
        }

        // Process inner headers first. Collapse each completed inner loop to
        // its header when walking the predecessors of an enclosing loop.
        latches.sort_unstable_by_key(|&(header, _)| rpo[header.index()].unwrap());
        let mut innermost = alloc::vec![Block::invalid(); f.num_blocks()];
        let mut parent = alloc::vec![Block::invalid(); f.num_blocks()];
        let mut representative = alloc::vec![Block::invalid(); f.num_blocks()];
        let mut pending = Vec::new();
        for group in latches.chunk_by(|a, b| a.0 == b.0).rev() {
            let header = group[0].0;
            innermost[header.index()] = header;
            representative[header.index()] = header;
            pending.extend(group.iter().map(|&(_, latch)| latch));

            while let Some(block) = pending.pop() {
                if rpo[block.index()].is_none() {
                    continue;
                }
                let inner = innermost[block.index()];
                let walk_from = if inner.is_invalid() {
                    innermost[block.index()] = header;
                    block
                } else {
                    let mut root = inner;
                    while representative[root.index()] != root {
                        let next = representative[root.index()];
                        representative[root.index()] = representative[next.index()];
                        root = representative[root.index()];
                    }
                    if root == header {
                        continue;
                    }
                    parent[root.index()] = header;
                    representative[root.index()] = header;
                    root
                };
                pending.extend(f.block_preds(walk_from));
            }
        }

        // A loop's parent dominates its header, so RPO visits the parent first.
        for group in latches.chunk_by(|a, b| a.0 == b.0) {
            let header = group[0].0;
            let outer = parent[header.index()];
            depths[header.index()] = if outer.is_valid() {
                depths[outer.index()] + 1
            } else {
                1
            };
        }
        for (index, inner) in innermost.into_iter().enumerate() {
            if inner.is_valid() {
                depths[index] = depths[inner.index()];
            }
        }
        true
    }

    pub fn dominates(&self, a: Block, b: Block) -> bool {
        domtree::dominates(&self.domtree[..], a, b)
    }
}
