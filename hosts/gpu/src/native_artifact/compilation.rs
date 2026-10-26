//! Explicit compile-time choices. Public entry points always use the fixed
//! default; no option grants source proof or changes runtime admission.
#[derive(Clone, Copy, Default)]
pub(in crate::native_artifact) struct OptimizationOptions {
    #[cfg(feature = "canonical-native-program")]
    disabled: Option<OptimizationPass>,
}
#[cfg(feature = "canonical-native-program")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::native_artifact) enum OptimizationPass {
    EdgeFacts,
    TagThreading,
    FrameEffects,
    LoopBounds,
    LookupFacts,
    CfgCleanup,
    LoopOutlining,
    LoopInvariants,
    PreservingCalls,
    MutationFacts,
    CountBounds,
    LoopConstants,
    SteadyLoops,
    ColdRecipes,
    LookupSplit,
    CandidateValues,
    PairedCandidates,
    ScalarLoops,
    DisjointState,
    NontrappingHomes,
}
#[cfg(feature = "canonical-native-program")]
impl OptimizationOptions {
    pub fn enabled(self, pass: OptimizationPass) -> bool {
        self.disabled != Some(pass)
    }
    #[cfg(test)]
    pub fn without(pass: OptimizationPass) -> Self {
        Self {
            disabled: Some(pass),
        }
    }
}

#[cfg(all(test, feature = "canonical-native-program"))]
#[derive(Clone, Copy, Default, Debug, Eq, PartialEq)]
pub(in crate::native_artifact) struct EmissionCounters {
    pub edge_facts: (usize, usize),
    pub tag_threading: (usize, usize, usize),
    pub frame_effects: (usize, usize, usize),
    pub loop_bounds: (usize, usize),
    pub lookup_facts: (usize, usize),
    pub cfg_cleanup: (usize, usize),
    pub loop_outlining: usize,
    pub loop_invariants: (usize, usize, usize),
    pub preserving_calls: usize,
    pub mutation_facts: (usize, usize, usize),
    pub count_bounds: usize,
    pub loop_constants: usize,
    pub steady_loops: usize,
    pub cold_recipes: usize,
    pub candidate_values: usize,
    pub paired_candidates: usize,
}
/// Synchronous scratch for exactly one shared JIT/Object definition. Options
/// are immutable; test observations are local and never authorize a transform.
#[cfg(feature = "canonical-native-program")]
pub(in crate::native_artifact) struct CompilationControls {
    options: OptimizationOptions,
    packed_regions: std::cell::Cell<usize>,
    scalar_regions: std::cell::Cell<usize>,
    invariant: EmissionBudget,
    mutation: EmissionBudget,
    count_value: EmissionBudget,
    steady: EmissionBudget,
}
#[cfg(feature = "canonical-native-program")]
impl CompilationControls {
    pub fn new(options: OptimizationOptions) -> Self {
        Self {
            options,
            packed_regions: Default::default(),
            scalar_regions: Default::default(),
            invariant: Default::default(),
            mutation: Default::default(),
            count_value: Default::default(),
            steady: Default::default(),
        }
    }
    pub fn enabled(&self, pass: OptimizationPass) -> bool {
        self.options.enabled(pass)
    }
    pub fn options(&self) -> OptimizationOptions {
        self.options
    }
}

// Selection budgets belong to compilation scheduling, not to the report.
// They retain the existing bounded metadata envelope and fallback order.
#[cfg(feature = "canonical-native-program")]
#[derive(Default)]
struct EmissionBudget {
    regions: std::cell::Cell<usize>,
    records: std::cell::Cell<usize>,
}
#[cfg(feature = "canonical-native-program")]
impl EmissionBudget {
    fn reserve(&self, records: usize) -> bool {
        if self.regions.get() >= 256 || self.records.get().saturating_add(records) > 4096 {
            return false;
        }
        self.records.set(self.records.get() + records);
        self.regions.set(self.regions.get() + 1);
        true
    }
}
#[cfg(feature = "canonical-native-program")]
impl CompilationControls {
    pub fn reserve_packed_region(&self) -> bool {
        let count = self.packed_regions.get();
        self.packed_regions.set(count.saturating_add(1));
        count < 256
    }
    pub fn scalar_region_available(&self) -> bool {
        self.scalar_regions.get() < 256
    }
    pub fn emitted_scalar_region(&self) {
        self.scalar_regions.set(self.scalar_regions.get() + 1);
    }
    pub fn reserve_invariant_region(&self, records: usize) -> bool {
        self.invariant.reserve(records)
    }
    pub fn reserve_mutation_region(&self, records: usize) -> bool {
        self.mutation.reserve(records)
    }
    pub fn reserve_count_value_region(&self, records: usize) -> bool {
        self.count_value.reserve(records)
    }
    pub fn reserve_steady_region(&self, records: usize) -> bool {
        self.steady.reserve(records)
    }
    pub fn nest_steady_region(&self) {
        // The direct-lookup callee now belongs to its hashed caller's report.
        // Its record demand remains charged; only the top-level slot is freed.
        self.steady.regions.set(self.steady.regions.get() - 1);
    }
}
