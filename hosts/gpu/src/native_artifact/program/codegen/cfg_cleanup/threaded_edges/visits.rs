//! Scratch membership for repeated walks of one immutable native CFG. Starting
//! a walk discards every prior visit without clearing or reallocating a tree.
use cranelift_codegen::ir::Block;

pub(super) struct Visits {
    stamps: Vec<u32>,
    generation: u32,
}

impl Visits {
    pub(super) fn new(blocks: usize) -> Self {
        Self {
            stamps: vec![0; blocks],
            generation: 0,
        }
    }

    pub(super) fn begin(&mut self) {
        if self.generation == u32::MAX {
            self.stamps.fill(0);
            self.generation = 1;
        } else {
            self.generation += 1;
        }
    }

    pub(super) fn insert(&mut self, block: Block) -> bool {
        let stamp = &mut self.stamps[block.as_u32() as usize];
        if *stamp == self.generation {
            false
        } else {
            *stamp = self.generation;
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    // Compare the exact DFS visit order, including cycles, duplicate pending
    // entries, owner-stop boundaries and reuse across changing query roots.
    #[test]
    fn all_four_block_graph_walks_match_ordered_sets() {
        let mut visits = Visits::new(4);
        let mut comparisons = 0;
        for edges in 0u32..1 << 16 {
            for start in 0..4 {
                for owner in 0..4 {
                    let walk = |insert: &mut dyn FnMut(Block) -> bool| {
                        let mut pending = vec![start];
                        let mut order = Vec::new();
                        while let Some(block) = pending.pop() {
                            if block == owner || !insert(Block::from_u32(block)) {
                                continue;
                            }
                            order.push(block);
                            for next in 0..4 {
                                if edges & (1 << (block * 4 + next)) != 0 {
                                    pending.push(next);
                                }
                            }
                        }
                        order
                    };
                    visits.begin();
                    let mut ordered = BTreeSet::new();
                    assert_eq!(
                        walk(&mut |b| visits.insert(b)),
                        walk(&mut |b| ordered.insert(b))
                    );
                    comparisons += 1;
                }
            }
        }
        assert_eq!(comparisons, 1_048_576);
    }

    #[test]
    fn generation_wrap_retains_no_visits() {
        let mut visits = Visits::new(100_001);
        visits.begin();
        let low = Block::from_u32(1);
        let high = Block::from_u32(100_000);
        assert!(visits.insert(low));
        visits.generation = u32::MAX;
        assert!(visits.insert(high));
        visits.begin();
        assert!(visits.insert(low));
        assert!(visits.insert(high));
        assert!(!visits.insert(low));
        assert!(!visits.insert(high));
    }
}
