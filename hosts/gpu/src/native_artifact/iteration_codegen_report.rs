//! Bounded evidence of substitutions actually selected while emitting native
//! instructions. This is build metadata, never executable proof or authority.
use mech_engine::native_artifact::{NativeMatchValue, NativeScalarOp};

const MAX_REGIONS: usize = 256;
const MAX_SUBSTITUTIONS: usize = 4096;

#[derive(Default)]
pub(in crate::native_artifact) struct CodegenReport {
    pub source_edge_facts: EdgeFacts,
    pub tag_threading: TagThreading,
    pub frame_effects: FrameEffects,
    pub guarded_loops: GuardedLoops,
    substitutions: usize,
    tag_records: usize,
    guarded_records: usize,
}
#[derive(serde::Serialize)]
pub(in crate::native_artifact) struct EdgeFacts {
    complete: bool,
    regions: Vec<Region>,
}
impl Default for EdgeFacts {
    fn default() -> Self { Self { complete: true, regions: Vec::new() } }
}
#[derive(serde::Serialize)]
struct Region {
    header: usize,
    guards: Vec<Guard>,
    arithmetic: Vec<Arithmetic>,
}
#[derive(serde::Serialize)]
struct Guard { site: usize, source: NativeMatchValue, offset: usize, ordinal: u32 }
#[derive(serde::Serialize)]
struct Arithmetic { site: usize, op: NativeScalarOp, operands: [NativeMatchValue; 2] }
#[derive(Clone, Copy)]
pub(in crate::native_artifact) struct RegionToken(usize);

#[derive(serde::Serialize)]
pub(in crate::native_artifact) struct TagThreading {
    complete: bool,
    regions: Vec<TagRegion>,
}
impl Default for TagThreading {
    fn default() -> Self { Self { complete: true, regions: Vec::new() } }
}
#[derive(serde::Serialize)]
struct TagRegion {
    header: usize,
    carrier_word: usize,
    variants: Vec<u32>,
    source_sites: usize,
    product_nodes: usize,
    header_versions: usize,
    folded_branches: Vec<usize>,
    known_latches: Vec<usize>,
}
#[derive(serde::Serialize)]
pub(in crate::native_artifact) struct GuardedLoops { complete: bool, regions: Vec<GuardedRegion> }
impl Default for GuardedLoops { fn default() -> Self { Self { complete: true, regions: Vec::new() } } }
#[derive(serde::Serialize)]
struct GuardedRegion {
    header: usize, index_word: usize, width_word: usize, limit_word: usize, active_tag: u32,
    stop_guards: [usize; 3], range_site: usize, recurrence_latches: Vec<usize>,
    emitted_nodes: usize, certified_range_versions: Vec<usize>,
    fixed_template_instructions: usize,
    checked_fallback: bool,
}

#[derive(serde::Serialize)]
pub(in crate::native_artifact) struct FrameEffects { complete: bool, regions: Vec<FrameRegion> }
impl Default for FrameEffects {
    fn default() -> Self { Self { complete: true, regions: Vec::new() } }
}
#[derive(serde::Serialize)]
struct FrameRegion { header: usize, operations: Vec<FrameOperation> }
#[derive(serde::Serialize)]
struct FrameOperation {
    operation: u64,
    // Number of emitted calls, including separate versions of a source site.
    // These are compile receipts, not runtime execution counts.
    emitted_calls: u64,
    stored_words: Vec<usize>,
    reloaded_words: Vec<usize>,
}

impl CodegenReport {
    /// Constant-size per-region counts accumulated only at actual narrow
    /// synchronization emissions. Each private op contributes at most once.
    pub fn frame_effects(&mut self, header: usize, operations: Vec<(u64, u64, u64, u64)>) {
        if self.frame_effects.regions.len() == MAX_REGIONS || operations.len() > 14 {
            self.frame_effects.complete = false;
            return;
        }
        let words = |mask: u64| (0..64).filter(|&word| mask & (1u64 << word) != 0).collect();
        self.frame_effects.regions.push(FrameRegion { header, operations: operations.into_iter()
            .map(|(operation, emitted_calls, before, after)| FrameOperation {
                operation, emitted_calls, stored_words: words(before), reloaded_words: words(after),
            }).collect() });
    }
    pub fn guarded_loop(&mut self, header: usize, index_word: usize, width_word: usize, limit_word: usize,
        active_tag: u32, stop_guards: [usize; 3], range_site: usize, recurrence_latches: Vec<usize>,
        emitted_nodes: usize, certified_range_versions: Vec<usize>, fixed_template_instructions: usize)
    {
        let count = 3usize.saturating_add(recurrence_latches.len()).saturating_add(certified_range_versions.len());
        if self.guarded_loops.regions.len() == MAX_REGIONS || self.guarded_records.saturating_add(count) > MAX_SUBSTITUTIONS {
            self.guarded_loops.complete = false; return;
        }
        self.guarded_records += count;
        self.guarded_loops.regions.push(GuardedRegion { header, index_word, width_word, limit_word, active_tag,
            stop_guards, range_site, recurrence_latches, emitted_nodes, certified_range_versions,
            fixed_template_instructions, checked_fallback: true });
    }
    /// Called after emitting the complete selected product graph. Source
    /// sites may repeat because distinct tag versions retain the same origin.
    pub fn tag_threading(&mut self, header: usize, carrier_word: usize, variants: Vec<u32>,
        source_sites: usize, product_nodes: usize, header_versions: usize,
        folded_branches: Vec<usize>, known_latches: Vec<usize>)
    {
        let count = variants.len().saturating_add(folded_branches.len()).saturating_add(known_latches.len());
        if self.tag_threading.regions.len() == MAX_REGIONS
            || self.tag_records.saturating_add(count) > MAX_SUBSTITUTIONS
        {
            self.tag_threading.complete = false;
            return;
        }
        self.tag_records += count;
        self.tag_threading.regions.push(TagRegion { header, carrier_word, variants,
            source_sites, product_nodes, header_versions, folded_branches, known_latches });
    }
    pub fn packed_ssa_region(&mut self, header: usize) -> Option<RegionToken> {
        if self.source_edge_facts.regions.len() == MAX_REGIONS {
            self.source_edge_facts.complete = false;
            return None;
        }
        let token = RegionToken(self.source_edge_facts.regions.len());
        self.source_edge_facts.regions.push(Region { header, guards: Vec::new(), arithmetic: Vec::new() });
        Some(token)
    }
    fn admit(&mut self, token: Option<RegionToken>) -> Option<&mut Region> {
        let token = token?;
        if self.substitutions == MAX_SUBSTITUTIONS {
            self.source_edge_facts.complete = false;
            return None;
        }
        self.substitutions += 1;
        self.source_edge_facts.regions.get_mut(token.0)
    }
    pub fn guard(&mut self, token: Option<RegionToken>, site: usize,
        source: NativeMatchValue, offset: usize, ordinal: u32)
    {
        if let Some(region) = self.admit(token) { region.guards.push(Guard { site, source, offset, ordinal }); }
    }
    pub fn arithmetic(&mut self, token: Option<RegionToken>, site: usize,
        op: NativeScalarOp, operands: [NativeMatchValue; 2])
    {
        if let Some(region) = self.admit(token) { region.arithmetic.push(Arithmetic { site, op, operands }); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn emission_receipts_are_bounded_and_report_incomplete_coverage() {
        let mut report = CodegenReport::default();
        for header in 0..MAX_REGIONS { assert!(report.packed_ssa_region(header).is_some()); }
        assert!(report.source_edge_facts.complete);
        assert!(report.packed_ssa_region(MAX_REGIONS).is_none());
        assert!(!report.source_edge_facts.complete);
        assert_eq!(report.source_edge_facts.regions.len(), MAX_REGIONS);
        // The admission lane is shared by both substitution kinds; overflow
        // cannot quietly become a receipt claiming complete site coverage.
        let mut report = CodegenReport::default();
        let region = report.packed_ssa_region(19);
        for _ in 0..MAX_SUBSTITUTIONS { assert!(report.admit(region).is_some()); }
        assert!(report.source_edge_facts.complete);
        assert!(report.admit(region).is_none());
        assert!(!report.source_edge_facts.complete);
        assert_eq!(report.substitutions, MAX_SUBSTITUTIONS);
        let mut report = CodegenReport::default();
        report.tag_threading(1, 2, vec![0, 1], 32, 64, 3, vec![9; MAX_SUBSTITUTIONS - 4], vec![10; 2]);
        assert!(report.tag_threading.complete);
        assert_eq!(report.tag_threading.regions.len(), 1);
        report.tag_threading(1, 2, vec![0, 1], 32, 64, 3, vec![9], vec![10]);
        assert!(!report.tag_threading.complete);
        assert_eq!(report.tag_threading.regions.len(), 1, "bounded receipt must flag incomplete coverage");
        let mut report = CodegenReport::default();
        for header in 0..MAX_REGIONS { report.frame_effects(header, vec![(0, 3, 8, 16)]); }
        assert!(report.frame_effects.complete);
        report.frame_effects(MAX_REGIONS, vec![]);
        assert!(!report.frame_effects.complete);
        assert_eq!(report.frame_effects.regions.len(), MAX_REGIONS);
        let mut report = CodegenReport::default();
        report.frame_effects(0, vec![(0, 1, 0, 0); 15]);
        assert!(!report.frame_effects.complete);
        assert!(report.frame_effects.regions.is_empty());
        let mut report = CodegenReport::default();
        for header in 0..MAX_REGIONS {
            report.guarded_loop(header, 1, 2, 3, 0, [4,5,6], 7, vec![8], 64, vec![39], 1024);
        }
        assert!(report.guarded_loops.complete);
        report.guarded_loop(0, 1, 2, 3, 0, [4,5,6], 7, vec![8], 64, vec![39], 1024);
        assert!(!report.guarded_loops.complete);
        assert_eq!(report.guarded_loops.regions.len(), MAX_REGIONS);
    }
}
