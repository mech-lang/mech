use mech_core::{FunctionCatalog, FunctionCatalogBuilder, OperationId, RuntimeFunctionId};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, OnceLock};

/// Immutable ownership policy for the temporary catalog-to-legacy transition.
///
/// An owned operation or runtime ID must be resolved by the supplied catalog;
/// it may not silently fall through to the legacy registries. Keeping this
/// policy beside the catalog makes custom compositions independent from the
/// standard distribution's migration state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LegacyFunctionBoundary {
    catalog_operations: BTreeMap<OperationId, String>,
    catalog_runtime_functions: BTreeSet<RuntimeFunctionId>,
}

impl LegacyFunctionBoundary {
    pub fn from_catalog(catalog: &FunctionCatalog) -> Self {
        Self {
            catalog_operations: catalog
                .specializer_entries()
                .chain(catalog.intrinsic_specializer_entries())
                .map(|entry| (entry.operation, entry.canonical_name.clone()))
                .collect(),
            catalog_runtime_functions: catalog.runtime_entries().map(|entry| entry.id).collect(),
        }
    }

    pub fn owns_operation(&self, operation: OperationId) -> bool {
        self.catalog_operations.contains_key(&operation)
    }

    pub fn owns_named_operation(&self, operation: OperationId, requested_name: &str) -> bool {
        self.catalog_operations
            .get(&operation)
            .is_some_and(|canonical_name| canonical_name == requested_name)
    }

    pub fn owns_runtime_function(&self, id: RuntimeFunctionId) -> bool {
        self.catalog_runtime_functions.contains(&id)
    }

    pub fn operation_count(&self) -> usize {
        self.catalog_operations.len()
    }

    pub fn runtime_function_count(&self) -> usize {
        self.catalog_runtime_functions.len()
    }
}

/// Builder used when a migration boundary intentionally claims an entry that
/// is absent from the catalog, so missing implementations fail closed.
#[derive(Default)]
pub struct LegacyFunctionBoundaryBuilder {
    boundary: LegacyFunctionBoundary,
}

impl LegacyFunctionBoundaryBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn claim_operation(&mut self, canonical_name: impl Into<String>) -> &mut Self {
        let canonical_name = canonical_name.into();
        self.boundary
            .catalog_operations
            .insert(OperationId::from_name(&canonical_name), canonical_name);
        self
    }

    pub fn claim_runtime_function(&mut self, id: RuntimeFunctionId) -> &mut Self {
        self.boundary.catalog_runtime_functions.insert(id);
        self
    }

    pub fn claim_catalog(&mut self, catalog: &FunctionCatalog) -> &mut Self {
        self.boundary.catalog_operations.extend(
            catalog
                .specializer_entries()
                .chain(catalog.intrinsic_specializer_entries())
                .map(|entry| (entry.operation, entry.canonical_name.clone())),
        );
        self.boundary
            .catalog_runtime_functions
            .extend(catalog.runtime_entries().map(|entry| entry.id));
        self
    }

    pub fn build(self) -> LegacyFunctionBoundary {
        self.boundary
    }
}

/// The immutable function composition supplied to a program.
#[derive(Clone)]
pub struct FunctionSystem {
    catalog: Arc<FunctionCatalog>,
    legacy_boundary: Arc<LegacyFunctionBoundary>,
}

impl FunctionSystem {
    pub fn new(
        catalog: Arc<FunctionCatalog>,
        legacy_boundary: Arc<LegacyFunctionBoundary>,
    ) -> Self {
        Self {
            catalog,
            legacy_boundary,
        }
    }

    /// Treats every entry in this exact catalog as catalog-owned.
    pub fn from_catalog(catalog: Arc<FunctionCatalog>) -> Self {
        let legacy_boundary = Arc::new(LegacyFunctionBoundary::from_catalog(&catalog));
        Self::new(catalog, legacy_boundary)
    }

    pub fn catalog(&self) -> &Arc<FunctionCatalog> {
        &self.catalog
    }

    pub fn legacy_boundary(&self) -> &Arc<LegacyFunctionBoundary> {
        &self.legacy_boundary
    }
}

fn build_default_function_system() -> FunctionSystem {
    let mut builder = FunctionCatalogBuilder::new();

    crate::stdlib::catalog::install_catalog(&mut builder)
        .expect("engine function catalog fragment must be valid");
    #[cfg(feature = "mech-math")]
    mech_math::install_catalog(&mut builder).expect("math function catalog fragment must be valid");
    #[cfg(feature = "mech-compare")]
    mech_compare::install_catalog(&mut builder)
        .expect("compare function catalog fragment must be valid");
    #[cfg(feature = "mech-logic")]
    mech_logic::install_catalog(&mut builder)
        .expect("logic function catalog fragment must be valid");
    #[cfg(feature = "mech-range")]
    mech_range::install_catalog(&mut builder)
        .expect("range function catalog fragment must be valid");
    #[cfg(feature = "mech-matrix")]
    mech_matrix::install_catalog(&mut builder)
        .expect("matrix function catalog fragment must be valid");
    #[cfg(feature = "mech-set")]
    mech_set::install_catalog(&mut builder).expect("set function catalog fragment must be valid");
    #[cfg(feature = "mech-string")]
    mech_string::install_catalog(&mut builder)
        .expect("string function catalog fragment must be valid");
    #[cfg(feature = "stats")]
    mech_stats::install_catalog(&mut builder)
        .expect("stats function catalog fragment must be valid");
    #[cfg(feature = "combinatorics")]
    mech_combinatorics::install_catalog(&mut builder)
        .expect("combinatorics function catalog fragment must be valid");

    let catalog = Arc::new(
        builder
            .build()
            .expect("static function catalog must be valid"),
    );
    FunctionSystem::from_catalog(catalog)
}

/// Returns the explicit function composition for the standard distribution.
///
/// Standard composition moves to `mech-stdlib` in PR3. PR2 keeps the explicit
/// composition here while eliminating all discovery through registries.
pub fn default_function_system() -> FunctionSystem {
    static SYSTEM: OnceLock<FunctionSystem> = OnceLock::new();
    SYSTEM.get_or_init(build_default_function_system).clone()
}

/// Compatibility accessor for callers that only need the immutable catalog.
pub fn default_function_catalog() -> Arc<FunctionCatalog> {
    Arc::clone(default_function_system().catalog())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn static_exports(catalog: &FunctionCatalog) -> Vec<&mech_core::FunctionExport> {
        catalog
            .specializer_entries()
            .flat_map(|entry| catalog.exports_for_operation(entry.operation))
            .collect()
    }

    #[test]
    fn default_system_owns_every_explicit_catalog_entry() {
        let system = default_function_system();
        let catalog = system.catalog();

        assert_eq!(
            system.legacy_boundary().operation_count(),
            catalog.specializer_count() + catalog.intrinsic_specializer_count(),
        );
        assert_eq!(
            system.legacy_boundary().runtime_function_count(),
            catalog.runtime_factory_count(),
        );
        assert!(catalog.specializer_entries().all(|entry| {
            system
                .legacy_boundary()
                .owns_named_operation(entry.operation, &entry.canonical_name)
        }));
        assert!(catalog.intrinsic_specializer_entries().all(|entry| {
            system
                .legacy_boundary()
                .owns_named_operation(entry.operation, &entry.canonical_name)
        }));
        assert!(
            catalog
                .runtime_entries()
                .all(|entry| system.legacy_boundary().owns_runtime_function(entry.id))
        );
    }

    #[cfg(feature = "linked_stdlib")]
    #[test]
    fn linked_standard_catalog_matches_the_frozen_source_surface_counts() {
        let catalog = default_function_catalog();
        let exports = static_exports(&catalog);

        assert_eq!(catalog.specializer_count(), 119);
        assert_eq!(catalog.intrinsic_specializer_count(), 10);
        assert_eq!(
            exports
                .iter()
                .filter(|export| export.exposure == mech_core::FunctionExposure::Prelude)
                .count(),
            52,
        );
        assert_eq!(
            exports
                .iter()
                .filter(|export| export.module.is_some())
                .count(),
            50,
        );
        assert_eq!(exports.len(), 120);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn explicit_runtime_catalog_matches_the_linked_legacy_inventory() {
        use mech_core::FunctionDescriptor;

        let catalog = default_function_catalog();
        let mut legacy = BTreeMap::<RuntimeFunctionId, (&'static str, usize)>::new();
        for descriptor in inventory::iter::<FunctionDescriptor> {
            let id = RuntimeFunctionId::from_name(descriptor.name);
            let incoming = (descriptor.name, descriptor.ptr as usize);
            if let Some(existing) = legacy.insert(id, incoming) {
                assert_eq!(
                    existing, incoming,
                    "conflicting duplicate legacy runtime factory {}",
                    descriptor.name,
                );
            }
        }

        assert_eq!(catalog.runtime_factory_count(), legacy.len());
        for entry in catalog.runtime_entries() {
            let (legacy_name, legacy_factory) = legacy
                .remove(&entry.id)
                .unwrap_or_else(|| panic!("explicit-only runtime factory {}", entry.name));
            assert_eq!(entry.name, legacy_name);
            assert_eq!(
                entry.factory as usize, legacy_factory,
                "runtime factory pointer mismatch for {}",
                entry.name,
            );
        }
        assert!(
            legacy.is_empty(),
            "unmigrated legacy runtime factories: {legacy:?}",
        );
    }

    #[test]
    fn explicit_boundary_can_claim_catalog_owned_missing_entries() {
        let operation = OperationId::from_name("test/claimed");
        let runtime_id = RuntimeFunctionId::from_name("ClaimedSS<f64>");
        let mut boundary = LegacyFunctionBoundaryBuilder::new();
        boundary
            .claim_operation("test/claimed")
            .claim_runtime_function(runtime_id);
        let boundary = Arc::new(boundary.build());
        let catalog = Arc::new(FunctionCatalogBuilder::new().build().unwrap());
        let system = FunctionSystem::new(Arc::clone(&catalog), Arc::clone(&boundary));

        assert!(Arc::ptr_eq(system.catalog(), &catalog));
        assert!(Arc::ptr_eq(system.legacy_boundary(), &boundary));
        assert!(system.legacy_boundary().owns_operation(operation));
        assert!(
            system
                .legacy_boundary()
                .owns_named_operation(operation, "test/claimed")
        );
        assert!(system.legacy_boundary().owns_runtime_function(runtime_id));
    }

    #[test]
    fn named_ownership_rejects_a_colliding_requested_name() {
        let operation = OperationId::from_name("test/claimed");
        let mut boundary = LegacyFunctionBoundaryBuilder::new();
        boundary.claim_operation("test/claimed");
        let boundary = boundary.build();

        assert!(boundary.owns_named_operation(operation, "test/claimed"));
        assert!(!boundary.owns_named_operation(operation, "different/colliding-name"));
    }

    #[test]
    fn custom_catalog_ownership_never_inherits_the_default_system() {
        let catalog = Arc::new(FunctionCatalogBuilder::new().build().unwrap());
        let system = FunctionSystem::from_catalog(Arc::clone(&catalog));

        assert!(Arc::ptr_eq(system.catalog(), &catalog));
        assert_eq!(system.legacy_boundary().operation_count(), 0);
        assert_eq!(system.legacy_boundary().runtime_function_count(), 0);
        assert!(
            !system
                .legacy_boundary()
                .owns_operation(OperationId::from_name("math/add"))
        );
        assert!(
            !system
                .legacy_boundary()
                .owns_runtime_function(RuntimeFunctionId::from_name("AddSS<f64>"))
        );
    }

    #[test]
    fn default_accessors_share_the_same_cached_catalog() {
        let first = default_function_system();
        let second = default_function_system();
        let catalog = default_function_catalog();

        assert!(Arc::ptr_eq(first.catalog(), second.catalog()));
        assert!(Arc::ptr_eq(
            first.legacy_boundary(),
            second.legacy_boundary()
        ));
        assert!(Arc::ptr_eq(first.catalog(), &catalog));
    }
}
