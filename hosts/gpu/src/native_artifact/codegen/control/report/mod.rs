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
    pub join_map_copies: Vec<JoinMapCopy>,
    pub scalar_loops: Vec<ScalarLoop>,
    substitutions: usize,
    tag_records: usize,
    guarded_records: usize,
    lookup_records: usize,
}
#[derive(serde::Serialize)]
pub(in crate::native_artifact) struct ScalarLoop {
    pub header: usize,
    pub carrier_word: usize,
    pub register_words: usize,
    pub source_sites: usize,
    pub product_nodes: usize,
    pub variants: Vec<u32>,
    pub folded_branches: Vec<usize>,
    pub known_latches: Vec<usize>,
    pub maximum_source_instructions: usize,
    pub fixed_template_instructions: usize,
}
#[derive(serde::Serialize)]
pub(in crate::native_artifact) struct JoinMapCopy {
    pub collection: u32,
    pub symbol: String,
    pub stack_bytes: u64,
    pub code_bytes: u64,
    pub geometry_words: usize,
    pub initialized_extent_word: usize,
}
#[derive(serde::Serialize)]
pub(in crate::native_artifact) struct SteadyLoop {
    pub header: usize,
    pub active_node: usize,
    pub active_tag: u32,
    pub symbol: String,
    pub lookup_mode: &'static str,
    pub lookup_cache_function: Option<Box<SteadyLoop>>,
    pub lookup_cache_selection_word: Option<usize>,
    pub candidate_value_loads: usize,
    pub paired_candidate_loads: usize,
    pub source_nodes: Vec<(usize, usize)>,
    pub present_nodes: Vec<usize>,
    pub continuations: Vec<(usize, Option<usize>, Vec<usize>)>,
    pub entry_words: Vec<usize>,
    pub specialized_widths: Vec<u64>,
    pub entry_instructions: usize,
    pub fuel_check_edges: Vec<Option<usize>>,
    pub maximum_fuel_check_instructions: usize,
    pub header_snapshot_edges: Vec<Option<usize>>,
    pub prepaid_stop_edges: Vec<usize>,
    pub fuel_reconstructed_at_exits: bool,
    pub prepaid_extent_arguments: usize,
    pub cursor_range_site: usize,
    pub cursor_index_word: usize,
    pub cursor_recurrence_edges: Vec<usize>,
    pub cold_snapshot_recipes: Vec<ColdSnapshot>,
    pub peeled_probe_nodes: Vec<usize>,
    pub entry_stop_index_word: Option<usize>,
    pub entry_stop_endpoint_words: Option<[usize; 2]>,
    pub maximum_source_instructions: usize,
    pub maximum_snapshot_instructions: usize,
    pub caller_instructions: usize,
    pub native_calls: usize,
    pub stack_bytes: u64,
    pub code_bytes: u64,
    pub cleanup: CfgCleanup,
}
#[derive(serde::Serialize)]
pub(in crate::native_artifact) struct ColdSnapshot {
    pub node: usize,
    pub header: bool,
    pub incoming: Option<usize>,
    pub words: Vec<(usize, u64)>,
    pub updates: Option<u64>,
    pub previous: Option<u64>,
    pub reload_words: [usize; 2],
}
#[derive(serde::Serialize)]
pub(in crate::native_artifact) struct CountValueLoop {
    pub header: usize,
    pub index_word: usize,
    pub active_tag: u32,
    pub version_stride: usize,
    pub emitted_nodes: usize,
    pub empty_seed_checked: bool,
    pub source_read_nodes: Vec<(usize, usize)>,
    pub source_add_nodes: Vec<(usize, usize)>,
    pub overflow_checks_removed: Vec<(usize, usize)>,
}
#[derive(serde::Serialize)]
pub(in crate::native_artifact) struct MutationLoop {
    pub header: usize,
    pub index_word: usize,
    pub active_tag: u32,
    pub version_stride: usize,
    pub emitted_nodes: usize,
    pub alignment_guard: bool,
    pub source_insert_nodes: Vec<(usize, usize)>,
    pub counter_headers: Vec<(usize, usize)>,
    pub counter_checks_removed: Vec<(usize, usize)>,
    pub nonempty_probes: Vec<(usize, usize)>,
}
#[derive(serde::Serialize)]
pub(in crate::native_artifact) struct BoundCallShim {
    pub symbol: String,
    pub stack_bytes: u64,
    pub code_bytes: u64,
    pub template_bound: usize,
}
#[derive(serde::Serialize)]
pub(in crate::native_artifact) struct PreservingCallRegion {
    pub header: usize,
    pub emitted_calls: usize,
    pub maximum_source_template_work: usize,
    pub extra_entry_work: usize,
}
#[derive(serde::Serialize)]
pub(in crate::native_artifact) struct CfgCleanup {
    pub enabled: bool,
    pub rounds: usize,
    pub constant_branches_removed: usize,
    pub before_blocks: usize,
    pub before_instructions: usize,
    pub after_blocks: usize,
    pub after_instructions: usize,
    pub fixed_point: bool,
    pub pre_codegen_data_passes: usize,
}
#[derive(serde::Serialize)]
pub(in crate::native_artifact) struct OutlinedLoop {
    pub header: usize,
    pub symbol: String,
    pub stack_bytes: u64,
    pub code_bytes: u64,
    pub caller_template_bound: usize,
    pub cleanup: CfgCleanup,
    pub constant_packet_bytes: u32,
    pub constant_endpoint_words: Option<[usize; 2]>,
    pub emitted_hash_constant_loads: usize,
    pub emitted_endpoint_loads: usize,
}
#[derive(serde::Serialize)]
pub(in crate::native_artifact) struct InvariantLoop {
    pub header: usize,
    pub version_stride: usize,
    pub emitted_nodes: usize,
    pub call_substitutions: Vec<(usize, usize, u64)>,
    pub rolling_substitutions: Vec<(usize, usize)>,
    pub steady_latches: Vec<(usize, usize)>,
    pub refreshes: Vec<(usize, usize)>,
    pub fixed_template_work: usize,
}
#[derive(serde::Serialize)]
pub(in crate::native_artifact) struct LoopConstants {
    complete: bool,
    callers: Vec<ConstantCaller>,
}
impl Default for LoopConstants {
    fn default() -> Self {
        Self {
            complete: true,
            callers: Vec::new(),
        }
    }
}
#[derive(serde::Serialize)]
struct ConstantCaller {
    header: usize,
    stack_bytes: u32,
    endpoint_words: Option<[usize; 2]>,
    hash_values: [u64; 2],
    template_instructions: usize,
}
impl CodegenReport {
    pub fn loop_constant_caller(
        &mut self,
        header: usize,
        stack_bytes: u32,
        endpoint_words: Option<[usize; 2]>,
        template_instructions: usize,
    ) {
        if self.loop_constants.callers.len() == MAX_REGIONS {
            self.loop_constants.complete = false;
            return;
        }
        self.loop_constants.callers.push(ConstantCaller {
            header,
            stack_bytes,
            endpoint_words,
            hash_values: [
                crate::native_artifact::program::runtime::maps::packed::HASH_FIRST,
                crate::native_artifact::program::runtime::maps::packed::HASH_SECOND,
            ],
            template_instructions,
        });
    }
}
#[derive(serde::Serialize)]
pub(in crate::native_artifact) struct EdgeFacts {
    complete: bool,
    regions: Vec<Region>,
}
impl Default for EdgeFacts {
    fn default() -> Self {
        Self {
            complete: true,
            regions: Vec::new(),
        }
    }
}
#[derive(serde::Serialize)]
struct Region {
    header: usize,
    guards: Vec<Guard>,
    arithmetic: Vec<Arithmetic>,
}
#[derive(serde::Serialize)]
struct Guard {
    site: usize,
    source: NativeMatchValue,
    offset: usize,
    ordinal: u32,
}
#[derive(serde::Serialize)]
struct Arithmetic {
    site: usize,
    op: NativeScalarOp,
    operands: [NativeMatchValue; 2],
}
#[derive(Clone, Copy)]
pub(in crate::native_artifact) struct RegionToken(usize);

#[derive(serde::Serialize)]
pub(in crate::native_artifact) struct TagThreading {
    complete: bool,
    regions: Vec<TagRegion>,
}
impl Default for TagThreading {
    fn default() -> Self {
        Self {
            complete: true,
            regions: Vec::new(),
        }
    }
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
pub(in crate::native_artifact) struct GuardedLoops {
    complete: bool,
    regions: Vec<GuardedRegion>,
}
impl Default for GuardedLoops {
    fn default() -> Self {
        Self {
            complete: true,
            regions: Vec::new(),
        }
    }
}
#[derive(serde::Serialize)]
struct GuardedRegion {
    header: usize,
    index_word: usize,
    width_word: usize,
    limit_word: usize,
    active_tag: u32,
    stop_guards: [usize; 3],
    range_site: usize,
    recurrence_latches: Vec<usize>,
    emitted_nodes: usize,
    certified_range_versions: Vec<usize>,
    fixed_template_instructions: usize,
    checked_fallback: bool,
}

#[derive(serde::Serialize)]
pub(in crate::native_artifact) struct LookupFacts {
    complete: bool,
    regions: Vec<LookupRegion>,
}
impl Default for LookupFacts {
    fn default() -> Self {
        Self {
            complete: true,
            regions: Vec::new(),
        }
    }
}
#[derive(serde::Serialize)]
struct LookupRegion {
    header: usize,
    source_nodes: usize,
    emitted_nodes: usize,
    substitutions: Vec<LookupSubstitution>,
}
#[derive(serde::Serialize)]
struct LookupSubstitution {
    node: usize,
    site: usize,
    operation: u64,
    fact: &'static str,
}

#[derive(serde::Serialize)]
pub(in crate::native_artifact) struct FrameEffects {
    complete: bool,
    regions: Vec<FrameRegion>,
}
impl Default for FrameEffects {
    fn default() -> Self {
        Self {
            complete: true,
            regions: Vec::new(),
        }
    }
}
#[derive(serde::Serialize)]
struct FrameRegion {
    header: usize,
    operations: Vec<FrameOperation>,
}
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
    pub fn lookup_facts(
        &mut self,
        header: usize,
        source_nodes: usize,
        emitted_nodes: usize,
        substitutions: Vec<(usize, usize, u64, &'static str)>,
    ) {
        if self.lookup_facts.regions.len() == MAX_REGIONS
            || self.lookup_records.saturating_add(substitutions.len()) > MAX_SUBSTITUTIONS
        {
            self.lookup_facts.complete = false;
            return;
        }
        self.lookup_records += substitutions.len();
        self.lookup_facts.regions.push(LookupRegion {
            header,
            source_nodes,
            emitted_nodes,
            substitutions: substitutions
                .into_iter()
                .map(|(node, site, operation, fact)| LookupSubstitution {
                    node,
                    site,
                    operation,
                    fact,
                })
                .collect(),
        });
    }
    /// Constant-size per-region counts accumulated only at actual narrow
    /// synchronization emissions. Each private op contributes at most once.
    pub fn frame_effects(&mut self, header: usize, operations: Vec<(u64, u64, u64, u64)>) {
        if self.frame_effects.regions.len() == MAX_REGIONS || operations.len() > 14 {
            self.frame_effects.complete = false;
            return;
        }
        let words = |mask: u64| (0..64).filter(|&word| mask & (1u64 << word) != 0).collect();
        self.frame_effects.regions.push(FrameRegion {
            header,
            operations: operations
                .into_iter()
                .map(|(operation, emitted_calls, before, after)| FrameOperation {
                    operation,
                    emitted_calls,
                    stored_words: words(before),
                    reloaded_words: words(after),
                })
                .collect(),
        });
    }
    pub fn guarded_loop(
        &mut self,
        header: usize,
        index_word: usize,
        width_word: usize,
        limit_word: usize,
        active_tag: u32,
        stop_guards: [usize; 3],
        range_site: usize,
        recurrence_latches: Vec<usize>,
        emitted_nodes: usize,
        certified_range_versions: Vec<usize>,
        fixed_template_instructions: usize,
    ) {
        let count = 3usize
            .saturating_add(recurrence_latches.len())
            .saturating_add(certified_range_versions.len());
        if self.guarded_loops.regions.len() == MAX_REGIONS
            || self.guarded_records.saturating_add(count) > MAX_SUBSTITUTIONS
        {
            self.guarded_loops.complete = false;
            return;
        }
        self.guarded_records += count;
        self.guarded_loops.regions.push(GuardedRegion {
            header,
            index_word,
            width_word,
            limit_word,
            active_tag,
            stop_guards,
            range_site,
            recurrence_latches,
            emitted_nodes,
            certified_range_versions,
            fixed_template_instructions,
            checked_fallback: true,
        });
    }
    /// Called after emitting the complete selected product graph. Source
    /// sites may repeat because distinct tag versions retain the same origin.
    pub fn tag_threading(
        &mut self,
        header: usize,
        carrier_word: usize,
        variants: Vec<u32>,
        source_sites: usize,
        product_nodes: usize,
        header_versions: usize,
        folded_branches: Vec<usize>,
        known_latches: Vec<usize>,
    ) {
        let count = variants
            .len()
            .saturating_add(folded_branches.len())
            .saturating_add(known_latches.len());
        if self.tag_threading.regions.len() == MAX_REGIONS
            || self.tag_records.saturating_add(count) > MAX_SUBSTITUTIONS
        {
            self.tag_threading.complete = false;
            return;
        }
        self.tag_records += count;
        self.tag_threading.regions.push(TagRegion {
            header,
            carrier_word,
            variants,
            source_sites,
            product_nodes,
            header_versions,
            folded_branches,
            known_latches,
        });
    }
    pub fn packed_ssa_region(&mut self, header: usize) -> Option<RegionToken> {
        if self.source_edge_facts.regions.len() == MAX_REGIONS {
            self.source_edge_facts.complete = false;
            return None;
        }
        let token = RegionToken(self.source_edge_facts.regions.len());
        self.source_edge_facts.regions.push(Region {
            header,
            guards: Vec::new(),
            arithmetic: Vec::new(),
        });
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
    pub fn guard(
        &mut self,
        token: Option<RegionToken>,
        site: usize,
        source: NativeMatchValue,
        offset: usize,
        ordinal: u32,
    ) {
        if let Some(region) = self.admit(token) {
            region.guards.push(Guard {
                site,
                source,
                offset,
                ordinal,
            });
        }
    }
    pub fn arithmetic(
        &mut self,
        token: Option<RegionToken>,
        site: usize,
        op: NativeScalarOp,
        operands: [NativeMatchValue; 2],
    ) {
        if let Some(region) = self.admit(token) {
            region.arithmetic.push(Arithmetic { site, op, operands });
        }
    }
}

#[cfg(test)]
#[path = "tests.rs"]
pub(in crate::native_artifact::codegen::control::codegen_report) mod tests;

#[cfg(all(test, feature = "canonical-native-program"))]
impl CodegenReport {
    /// Immutable actual-emission observations from this compilation's bounded
    /// report. There is no ambient counter to drain or leak to another build.
    pub fn emission_counts(&self) -> crate::native_artifact::codegen::options::EmissionCounters {
        use crate::native_artifact::codegen::options::EmissionCounters;
        assert!(
            self.source_edge_facts.complete
                && self.tag_threading.complete
                && self.frame_effects.complete
                && self.guarded_loops.complete
                && self.lookup_facts.complete
                && self.loop_constants.complete,
            "truncated emission report cannot qualify an ablation"
        );
        let mut counts = EmissionCounters::default();
        counts.edge_facts = (
            self.source_edge_facts
                .regions
                .iter()
                .map(|r| r.guards.len())
                .sum(),
            self.source_edge_facts
                .regions
                .iter()
                .map(|r| r.arithmetic.len())
                .sum(),
        );
        counts.tag_threading = (
            self.tag_threading
                .regions
                .iter()
                .map(|r| r.product_nodes)
                .sum(),
            self.tag_threading
                .regions
                .iter()
                .map(|r| r.folded_branches.len())
                .sum(),
            self.tag_threading
                .regions
                .iter()
                .map(|r| r.known_latches.len())
                .sum(),
        );
        for operation in self
            .frame_effects
            .regions
            .iter()
            .flat_map(|r| &r.operations)
        {
            let calls = usize::try_from(operation.emitted_calls).unwrap();
            counts.frame_effects.0 += calls;
            counts.frame_effects.1 += calls * operation.stored_words.len();
            counts.frame_effects.2 += calls * operation.reloaded_words.len();
        }
        counts.loop_bounds = (
            self.guarded_loops.regions.len(),
            self.guarded_loops
                .regions
                .iter()
                .map(|r| r.certified_range_versions.len())
                .sum(),
        );
        counts.lookup_facts = (
            self.lookup_facts
                .regions
                .iter()
                .filter(|r| !r.substitutions.is_empty())
                .count(),
            self.lookup_facts
                .regions
                .iter()
                .map(|r| r.substitutions.len())
                .sum(),
        );
        if let Some(cfg) = &self.cfg_cleanup {
            counts.cfg_cleanup = (usize::from(cfg.enabled), cfg.constant_branches_removed);
        }
        counts.loop_outlining = self.outlined_loops.len();
        counts.loop_invariants = (
            self.loop_invariants.len(),
            self.loop_invariants
                .iter()
                .map(|r| r.call_substitutions.len())
                .sum(),
            self.loop_invariants
                .iter()
                .map(|r| r.rolling_substitutions.len())
                .sum(),
        );
        counts.preserving_calls = self
            .preserving_call_regions
            .iter()
            .map(|r| r.emitted_calls)
            .sum();
        counts.mutation_facts = (
            self.mutation_loops
                .iter()
                .map(|r| r.counter_headers.len())
                .sum(),
            self.mutation_loops
                .iter()
                .map(|r| r.counter_checks_removed.len())
                .sum(),
            self.mutation_loops
                .iter()
                .map(|r| r.nonempty_probes.len())
                .sum(),
        );
        counts.count_bounds = self
            .count_value_loops
            .iter()
            .map(|r| r.overflow_checks_removed.len())
            .sum();
        counts.loop_constants = self.loop_constants.callers.len();
        fn steady(value: &SteadyLoop, counts: &mut EmissionCounters) {
            counts.steady_loops += usize::from(value.lookup_mode != "direct");
            counts.cold_recipes += value.cold_snapshot_recipes.len();
            counts.candidate_values += value.candidate_value_loads;
            counts.paired_candidates += value.paired_candidate_loads;
            if let Some(child) = &value.lookup_cache_function {
                steady(child, counts);
            }
        }
        for value in &self.steady_loops {
            steady(value, &mut counts);
        }
        counts
    }
}
