/// Selection policy for the experimental AArch64 masked-field lowering.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum Aarch64MaskedFields {
    /// Keep ordinary lowering.
    #[default]
    Off,
    /// Restrict selection to the original narrow cases.
    Narrow,
    /// Select wide cases and tie the destination to the original base.
    Wide,
    /// Select wide cases with a separate destination.
    WideCopy,
}

/// Mech's experimental machine-level choices for one configured ISA.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct Aarch64CompilationOptions {
    /// Reuse a dominating prefetched or indexed base.
    pub reuse_prefetch_bases: bool,
    /// Inspect or rewrite eligible loop live ranges.
    pub loop_live_ranges: bool,
    /// Reuse a successor subtract on a branch edge.
    pub subtract_branch: bool,
    /// Reuse a successor immediate decrement on a branch edge.
    pub subtract_immediate_branch: bool,
    /// Directory for floating-point spill diagnostics.
    #[cfg(feature = "std")]
    pub inspect_spill_cache: Option<std::path::PathBuf>,
    /// Reuse whole-register byte loads.
    pub reuse_i8_loads: bool,
    /// Reuse whole-register byte compare results.
    pub reuse_i8_compares: bool,
    /// Reuse proved whole-register byte boolean results.
    pub reuse_i8_booleans: bool,
    /// Reuse word-sized add results without extending them again.
    pub reuse_i32_add: bool,
    /// Reuse word-sized subtract results without extending them again.
    pub reuse_i32_sub: bool,
    /// Reuse word-sized multiply results without extending them again.
    pub reuse_i32_mul: bool,
    /// Reuse an eligible unsigned shift result.
    pub reuse_i32_shifts: bool,
    /// Pack eligible narrow halves.
    pub pack_narrow_halves: bool,
    /// Fold eligible shifted extensions.
    pub fold_shifted_extensions: bool,
    /// Combine disjoined comparisons that define a shared boolean.
    pub ccmp_shared_boolean: bool,
    /// Combine comparisons controlling a select and immediate branch.
    pub ccmp_branch_select: bool,
    /// Select masked-field insertions.
    pub masked_fields: Aarch64MaskedFields,
}
