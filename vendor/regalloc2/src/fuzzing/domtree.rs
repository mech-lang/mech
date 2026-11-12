//! Fuzz the dominator-tree calculation.

use crate::{domtree, postorder, Block};
use arbitrary::{Arbitrary, Result, Unstructured};
use std::collections::HashSet;
use std::{vec, vec::Vec};

#[derive(Clone, Debug)]
struct CFG {
    num_blocks: usize,
    preds: Vec<Vec<Block>>,
    succs: Vec<Vec<Block>>,
}

impl Arbitrary<'_> for CFG {
    fn arbitrary(u: &mut Unstructured) -> Result<CFG> {
        let num_blocks = u.int_in_range(1..=1000)?;
        let mut succs = vec![];
        for _ in 0..num_blocks {
            let mut block_succs = vec![];
            for _ in 0..u.int_in_range(0..=5)? {
                block_succs.push(Block::new(u.int_in_range(0..=(num_blocks - 1))?));
            }
            succs.push(block_succs);
        }
        let mut preds = vec![];
        for _ in 0..num_blocks {
            preds.push(vec![]);
        }
        for from in 0..num_blocks {
            for succ in &succs[from] {
                preds[succ.index()].push(Block::new(from));
            }
        }
        Ok(CFG {
            num_blocks,
            preds,
            succs,
        })
    }
}

#[derive(Clone, Debug)]
struct Path {
    blocks: Vec<Block>,
}

impl Path {
    fn choose_from_cfg(cfg: &CFG, u: &mut Unstructured) -> Result<Path> {
        let succs = u.int_in_range(0..=(2 * cfg.num_blocks))?;
        let mut block = Block::new(0);
        let mut blocks = vec![];
        blocks.push(block);
        for _ in 0..succs {
            if cfg.succs[block.index()].is_empty() {
                break;
            }
            block = *u.choose(&cfg.succs[block.index()])?;
            blocks.push(block);
        }
        Ok(Path { blocks })
    }
}

fn check_idom_violations(idom: &[Block], path: &Path) {
    // "a dom b" means that any path from the entry block through the CFG that
    // contains a and b will contain a before b.
    //
    // To test this, for any given block b_i, we have the set S of b_0 ..
    // b_{i-1}, and we walk up the domtree from b_i to get all blocks that
    // dominate b_i; each such block must appear in S. (Otherwise, we have a
    // counterexample for which dominance says it should appear in the path
    // prefix, but it does not.)
    let mut visited = HashSet::new();
    visited.insert(Block::new(0));
    for block in &path.blocks {
        let mut parent = idom[block.index()];
        let mut domset = HashSet::new();
        domset.insert(*block);
        while parent.is_valid() {
            assert!(visited.contains(&parent));
            domset.insert(parent);
            let next = idom[parent.index()];
            parent = next;
        }

        // Check that `dominates()` returns true for every block in domset,
        // and false for every other block.
        for domblock in 0..idom.len() {
            let domblock = Block::new(domblock);
            assert_eq!(
                domset.contains(&domblock),
                domtree::dominates(idom, domblock, *block)
            );
        }
        visited.insert(*block);
    }
}

/// A control-flow graph ([`CFG`]) and a [`Path`] through it.
#[derive(Clone, Debug)]
pub struct TestCase {
    cfg: CFG,
    path: Path,
}

impl Arbitrary<'_> for TestCase {
    fn arbitrary(u: &mut Unstructured) -> Result<TestCase> {
        let cfg = CFG::arbitrary(u)?;
        let path = Path::choose_from_cfg(&cfg, u)?;
        Ok(TestCase { cfg, path })
    }
}

pub fn check(t: TestCase) {
    let mut postorder = vec![];
    postorder::calculate(
        t.cfg.num_blocks,
        Block::new(0),
        &mut vec![],
        &mut postorder,
        |block| &t.cfg.succs[block.index()],
    )
    .unwrap();

    let mut idom = vec![];
    domtree::calculate(
        t.cfg.num_blocks,
        |block| &t.cfg.preds[block.index()],
        &postorder[..],
        &mut vec![],
        &mut idom,
        Block::new(0),
    );
    check_idom_violations(&idom[..], &t.path);
}

#[test]
fn smoke() {
    arbtest::arbtest(|u| {
        let test_case = TestCase::arbitrary(u)?;
        check(test_case);
        Ok(())
    })
    .budget_ms(1_000);
}
