//! Source-derived fresh/reusable control contexts within one numeric loan.
//! Only a finite producer plus its successful private/no-op write can enter
//! the reusable header. No runtime scalar word represents that permission.
use super::*;
use cranelift_codegen::ir::Block;
use mech_engine::native_artifact::NativeTailLoop;

#[derive(Clone, Copy, Eq, PartialEq)]
pub(in crate::native_artifact) enum Mode {
    Fresh,
    Ready,
    Write,
}
pub(in crate::native_artifact) struct Context {
    pub first: usize,
    pub write: usize,
    pub after_write: u32,
    pub latch: u32,
    pub continuation: usize,
    pub sites: Vec<usize>,
    pub blocks: BTreeMap<u32, Block>,
    pub staged: Block,
    pub slow: Block,
}
impl Context {
    pub fn derive(
        b: &mut FunctionBuilder<'_>,
        facts: &NativeProgramFacts<'_>,
        proof: &Proof,
        tail: &NativeTailLoop,
        prefix: &Prefix,
        recipe: &rolling::Recipe,
    ) -> Option<Self> {
        if recipe.old.is_none()
            || prefix.first == tail.header() as usize
            || prefix.first == tail.latch() as usize
            || super::super::numeric_read_packs::source_boundaries(facts, proof)
                .contains(&prefix.first)
        {
            return None;
        }
        let mut pre = BTreeSet::new();
        let mut pending = vec![tail.header()];
        while let Some(at) = pending.pop() {
            if at == tail.exit() || at == tail.latch() {
                continue;
            }
            let site = at as usize;
            if !proof.sites.contains(&site)
                || site == recipe.write
                || prefix.sites.contains(&site) && site != prefix.first
            {
                return None;
            }
            if !pre.insert(site) {
                continue;
            }
            if pre.len() > 128 {
                return None;
            }
            if site != prefix.first {
                pending.extend(facts.successors(at).ok()?.into_iter().flatten());
            }
        }
        if !pre.contains(&prefix.first) {
            return None;
        }
        let after_write = super::successor(facts, recipe.write)?;
        let mut at = after_write;
        let mut post = BTreeSet::new();
        let mut last = None;
        while at != tail.latch() {
            let site = at as usize;
            if !proof.sites.contains(&site)
                || pre.contains(&site)
                || !post.insert(site)
                || post.len() > 32
            {
                return None;
            }
            last = Some(site);
            at = super::successor(facts, site)?;
        }
        if last != Some(recipe.continuation) {
            return None;
        }
        pre.extend(post);
        pre.insert(tail.latch() as usize);
        if pre.len() > 160 {
            return None;
        }
        let blocks = pre
            .iter()
            .map(|&site| (site as u32, b.create_block()))
            .collect();
        let staged = b.create_block();
        let slow = b.create_block();
        b.set_cold_block(slow);
        Some(Self {
            first: prefix.first,
            write: recipe.write,
            after_write,
            latch: tail.latch(),
            continuation: recipe.continuation,
            sites: pre.into_iter().collect(),
            blocks,
            staged,
            slow,
        })
    }
    pub fn block(&self, mode: Mode, site: usize) -> Option<Block> {
        match mode {
            Mode::Fresh => None,
            Mode::Write => Some(self.staged),
            Mode::Ready => self.blocks.get(&(site as u32)).copied(),
        }
    }
    pub fn target(&self, mode: Mode, site: usize, next: u32) -> Option<Block> {
        match mode {
            Mode::Fresh => None,
            Mode::Write if next == self.write as u32 => Some(self.staged),
            Mode::Ready if next == self.latch && site != self.continuation => None,
            _ => self.blocks.get(&next).copied(),
        }
    }
    pub fn report(&self) -> (usize, usize, usize, usize) {
        (self.first, self.write, self.sites.len(), 0)
    }
}
