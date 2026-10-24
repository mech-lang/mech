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
    pub lookup_facts: LookupFacts,
    pub cfg_cleanup: Option<CfgCleanup>,
    pub outlined_loops: Vec<OutlinedLoop>,
    pub loop_invariants: Vec<InvariantLoop>,
    pub bound_call_shim: Option<BoundCallShim>,
    pub preserving_call_regions: Vec<PreservingCallRegion>,
    pub mutation_loops: Vec<MutationLoop>,
    pub count_value_loops: Vec<CountValueLoop>,
    pub loop_constants: LoopConstants,
    pub steady_loops: Vec<SteadyLoop>,
    substitutions: usize,
    tag_records: usize,
    guarded_records: usize,
    lookup_records: usize,
    invariant_records: usize,
    mutation_records: usize,
    count_value_records: usize,
    steady_records: usize,
}
#[derive(serde::Serialize)]
pub(in crate::native_artifact) struct SteadyLoop {
    pub header: usize, pub active_node: usize, pub active_tag: u32, pub symbol: String,
    pub source_nodes: Vec<(usize, usize)>, pub present_nodes: Vec<usize>,
    pub continuations: Vec<(usize, Option<usize>, Vec<usize>)>,
    pub entry_words: Vec<usize>, pub specialized_widths: Vec<u64>, pub entry_instructions: usize,
    pub fuel_check_edges: Vec<Option<usize>>, pub maximum_fuel_check_instructions: usize,
    pub maximum_source_instructions: usize, pub maximum_snapshot_instructions: usize,
    pub caller_instructions: usize, pub native_calls: usize,
    pub stack_bytes: u64, pub code_bytes: u64, pub cleanup: CfgCleanup,
}
#[derive(serde::Serialize)]
pub(in crate::native_artifact) struct CountValueLoop {
    pub header: usize, pub index_word: usize, pub active_tag: u32,
    pub version_stride: usize, pub emitted_nodes: usize, pub empty_seed_checked: bool,
    pub source_read_nodes: Vec<(usize, usize)>, pub source_add_nodes: Vec<(usize, usize)>,
    pub overflow_checks_removed: Vec<(usize, usize)>,
}
#[derive(serde::Serialize)]
pub(in crate::native_artifact) struct MutationLoop {
    pub header: usize, pub index_word: usize, pub active_tag: u32,
    pub version_stride: usize, pub emitted_nodes: usize, pub alignment_guard: bool,
    pub source_insert_nodes: Vec<(usize, usize)>,
    pub counter_headers: Vec<(usize, usize)>, pub counter_checks_removed: Vec<(usize, usize)>,
    pub nonempty_probes: Vec<(usize, usize)>,
}
#[derive(serde::Serialize)]
pub(in crate::native_artifact) struct BoundCallShim {
    pub symbol: String, pub stack_bytes: u64, pub code_bytes: u64, pub template_bound: usize,
}
#[derive(serde::Serialize)]
pub(in crate::native_artifact) struct PreservingCallRegion {
    pub header: usize, pub emitted_calls: usize, pub maximum_source_template_work: usize,
    pub extra_entry_work: usize,
}
#[derive(serde::Serialize)]
pub(in crate::native_artifact) struct CfgCleanup {
    pub enabled: bool, pub rounds: usize, pub constant_branches_removed: usize,
    pub before_blocks: usize, pub before_instructions: usize,
    pub after_blocks: usize, pub after_instructions: usize, pub fixed_point: bool,
    pub pre_codegen_data_passes: usize,
}
#[derive(serde::Serialize)]
pub(in crate::native_artifact) struct OutlinedLoop {
    pub header: usize, pub symbol: String, pub stack_bytes: u64, pub code_bytes: u64,
    pub caller_template_bound: usize, pub cleanup: CfgCleanup,
    pub constant_packet_bytes: u32, pub constant_endpoint_words: Option<[usize; 2]>,
    pub emitted_hash_constant_loads: usize, pub emitted_endpoint_loads: usize,
}
#[derive(serde::Serialize)]
pub(in crate::native_artifact) struct InvariantLoop {
    pub header: usize, pub version_stride: usize, pub emitted_nodes: usize,
    pub call_substitutions: Vec<(usize, usize, u64)>,
    pub rolling_substitutions: Vec<(usize, usize)>,
    pub steady_latches: Vec<(usize, usize)>, pub refreshes: Vec<(usize, usize)>,
    pub fixed_template_work: usize,
}
#[derive(serde::Serialize)]
pub(in crate::native_artifact) struct LoopConstants { complete: bool, callers: Vec<ConstantCaller> }
impl Default for LoopConstants { fn default() -> Self { Self { complete: true, callers: Vec::new() } } }
#[derive(serde::Serialize)]
struct ConstantCaller {
    header: usize, stack_bytes: u32, endpoint_words: Option<[usize; 2]>,
    hash_values: [u64; 2], template_instructions: usize,
}
impl CodegenReport {
    pub fn reserve_steady_region(&mut self, records: usize) -> bool {
        if self.steady_loops.len() >= MAX_REGIONS
            || self.steady_records.saturating_add(records) > MAX_SUBSTITUTIONS { return false; }
        self.steady_records += records;
        true
    }
    pub fn loop_constant_caller(&mut self, header: usize, stack_bytes: u32, endpoint_words: Option<[usize; 2]>, template_instructions: usize) {
        if self.loop_constants.callers.len() == MAX_REGIONS { self.loop_constants.complete = false; return; }
        self.loop_constants.callers.push(ConstantCaller { header, stack_bytes, endpoint_words,
            hash_values: [super::super::program_packed_map::HASH_FIRST, super::super::program_packed_map::HASH_SECOND], template_instructions });
    }
    pub fn reserve_count_value_region(&mut self, records: usize) -> bool {
        if self.count_value_loops.len() >= MAX_REGIONS
            || self.count_value_records.saturating_add(records) > MAX_SUBSTITUTIONS { return false; }
        self.count_value_records += records;
        true
    }
    pub fn reserve_mutation_region(&mut self, records: usize) -> bool {
        if self.mutation_loops.len() >= MAX_REGIONS
            || self.mutation_records.saturating_add(records) > MAX_SUBSTITUTIONS { return false; }
        self.mutation_records += records;
        true
    }
    pub fn reserve_invariant_region(&mut self, records: usize) -> bool {
        if self.loop_invariants.len() >= MAX_REGIONS
            || self.invariant_records.saturating_add(records) > MAX_SUBSTITUTIONS { return false; }
        self.invariant_records += records;
        true
    }
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
pub(in crate::native_artifact) struct LookupFacts { complete: bool, regions: Vec<LookupRegion> }
impl Default for LookupFacts { fn default() -> Self { Self { complete: true, regions: Vec::new() } } }
#[derive(serde::Serialize)]
struct LookupRegion {
    header: usize, source_nodes: usize, emitted_nodes: usize,
    substitutions: Vec<LookupSubstitution>,
}
#[derive(serde::Serialize)]
struct LookupSubstitution { node: usize, site: usize, operation: u64, fact: &'static str }

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
    pub fn lookup_facts(&mut self, header: usize, source_nodes: usize, emitted_nodes: usize,
        substitutions: Vec<(usize, usize, u64, &'static str)>)
    {
        if self.lookup_facts.regions.len() == MAX_REGIONS
            || self.lookup_records.saturating_add(substitutions.len()) > MAX_SUBSTITUTIONS
        { self.lookup_facts.complete = false; return; }
        self.lookup_records += substitutions.len();
        self.lookup_facts.regions.push(LookupRegion { header, source_nodes, emitted_nodes,
            substitutions: substitutions.into_iter().map(|(node, site, operation, fact)| LookupSubstitution {
                node, site, operation, fact,
            }).collect() });
    }
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
        let mut report = CodegenReport::default();
        for header in 0..MAX_REGIONS { report.lookup_facts(header, 32, 64, vec![(1, 2, 3, "Fresh")]); }
        assert!(report.lookup_facts.complete);
        report.lookup_facts(0, 32, 64, vec![]);
        assert!(!report.lookup_facts.complete);
        assert_eq!(report.lookup_facts.regions.len(), MAX_REGIONS);
        let mut report = CodegenReport::default();
        report.lookup_facts(0, 32, 64, vec![(1, 2, 3, "Cached"); MAX_SUBSTITUTIONS + 1]);
        assert!(!report.lookup_facts.complete);
        assert!(report.lookup_facts.regions.is_empty());
    }
}
