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
    DemandLiveness,
    EarlyIdentities,
    BooleanConditions,
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
    /// Public compilation enables both current-funded numeric trees and proved phases. Tests may
    /// ablate only their emission to exercise the independently proved smaller
    /// parent pieces and leaf loans retained on the ordinary fallback route.
    NumericTrees,
    /// Keep per-node counters in tests; settle their exact weighted projection
    /// only at cold boundaries instead of maintaining it in the native loop.
    ProjectedNumericDebits,
    /// Derive projected child totals from the unchanged native fuel ledger.
    FuelDerivedNumericDebits,
    /// Emission-only ablation retains the ordinary collection error path.
    UniformWords,
    NumericCollections,
    /// Omit capacity guards at a proved frontier with no append operation.
    ZeroAppendFrontiers,
    /// Refine a declined static capacity frontier using only a bounded pure
    /// source prefix. No source operation, debit or mutation is performed.
    RefinedAppendCapacity,
    SourceLiveness,
    PrivateLiveness,
    /// Omit repeated zero stores until a resumable helper can reify homes.
    ScalarHomeRetirement,
    NumericOutlining,
    BooleanBranches,
    BitCounts,
    ScalarSelects,
    UnsignedGuardFacts,
    /// Preserve a convex unsigned range through one common modular addition
    /// wrap, including canonicalized subtraction by a negative immediate.
    ModularUnsignedRanges,
    GuardedNumericLoads,
    /// Reuse a surviving successful U64 cell read for an immediately proved
    /// checked increment of the same current cell; retain ownership boundaries.
    CheckedIncrementWrites,
    /// Preserve a checked cell across a replacement of a proved independent
    /// logical carrier value, including its original copy-on-write boundary.
    IndependentViewUpdates,
    /// Use an exact fixed matrix extent certified by this numeric loan's
    /// original selected factory. Growing views retain their current length.
    StaticNumericExtents,
    /// Preserve bytewise FNV while loading bounded words and admitting exact
    /// two-byte equality tails in native immutable UTF8 work.
    Utf8WordChunks,
    /// Reuse native equality classes of an unchanged String matrix when a
    /// later immutable dictionary lookup reads the same source.
    ReuseStringClasses,
    /// Borrow a selected String-matrix cell only for its adjacent retained
    /// insertion. Ablation keeps the original canonical String construction.
    BorrowedMatrixMapKeys,
    /// Keep a complete-use Map-key result private across length/frame/cell uses.
    BorrowedMapKeys,
    /// Retain a completed generic dictionary through one native UTF8 lookup.
    GenericDictionary,
    /// Omit a complete-use initial Map-state aggregate, retaining its real children.
    DictionarySeedFrames,
    /// Keep admitted immutable Map keys and independent current native U64 cells.
    BorrowedDictionaryValues,
    /// Emit a closed fixed-key map body; ablation keeps its ordered callbacks.
    FixedKeyMapLoops,
    /// Test private ownership before loading/comparing an old write target.
    PrivateWritesFirst,
    /// Stronger immutable-cell facts require the guarded-load optimization.
    MovableCaptureReads,
    DisjointState,
    NontrappingHomes,
    ChildFunding,
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
    pub scalar_selects: usize,
    pub unsigned_guards: usize,
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
    scalar_captures: std::cell::RefCell<std::collections::BTreeSet<usize>>,
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
            scalar_captures: Default::default(),
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
    pub fn reserve_scalar_capture(&self, header: usize) -> bool {
        if self.scalar_captures.borrow().contains(&header) {
            return true;
        }
        if !self.scalar_region_available() {
            return false;
        }
        self.emitted_scalar_region();
        self.scalar_captures.borrow_mut().insert(header);
        true
    }
    pub fn scalar_capture_selected(&self, header: usize) -> bool {
        self.scalar_captures.borrow().contains(&header)
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
