//! Build-time selection of separately linkable, maintained resident factories.
//!
//! A path is not evidence: only an entry carrying its owner's private witness
//! can select one of these declarations. Generated launchers reference each
//! declaration directly, so the linker need not retain the complete installer.
use mech_core::{
    FunctionCatalogBuilder, ImplementationMemoryClass, MResult, ResidentKernelFactory,
    ResidentKernelFactoryEntry, ResidentOperationKey,
};

/// One maintained registration, shared by the complete catalog and generated
/// runtime-only applications. There is deliberately no public constructor.
pub struct NativeLinkedFactory {
    #[cfg(feature = "native-artifact")]
    rust_path: &'static str,
    #[cfg(feature = "native-artifact")]
    required_engine_features: &'static [&'static str],
    entry: fn() -> ResidentKernelFactoryEntry,
}

impl NativeLinkedFactory {
    pub(crate) const fn new(
        rust_path: &'static str,
        required_engine_features: &'static [&'static str],
        entry: fn() -> ResidentKernelFactoryEntry,
    ) -> Self {
        #[cfg(not(feature = "native-artifact"))]
        let _ = (rust_path, required_engine_features);
        Self {
            #[cfg(feature = "native-artifact")]
            rust_path,
            #[cfg(feature = "native-artifact")]
            required_engine_features,
            entry,
        }
    }

    #[cfg(feature = "native-artifact")]
    pub const fn rust_path(&self) -> &'static str {
        self.rust_path
    }

    /// Requirements owned by the registration, not inferred from its public
    /// operation name or assumed from the generator's own Cargo features.
    #[cfg(feature = "native-artifact")]
    pub const fn required_engine_features(&self) -> &'static [&'static str] {
        self.required_engine_features
    }

    pub fn install(&self, builder: &mut FunctionCatalogBuilder) -> MResult<()> {
        builder.insert_resident_factory_entry((self.entry)())
    }
}

#[cfg(feature = "native-artifact")]
struct RawFactoryWitness {
    linkage: &'static NativeLinkedFactory,
}

pub(super) fn raw_entry(
    module: &[&str],
    name: &str,
    memory: ImplementationMemoryClass,
    factory: ResidentKernelFactory,
    linkage: &'static NativeLinkedFactory,
) -> ResidentKernelFactoryEntry {
    let key = ResidentOperationKey::new(
        module.iter().map(|part| (*part).into()).collect(),
        name.into(),
    )
    .expect("maintained factory linkage has a valid key");
    let entry = ResidentKernelFactoryEntry::new(key, memory, factory);
    #[cfg(not(feature = "native-artifact"))]
    let _ = linkage;
    #[cfg(feature = "native-artifact")]
    let entry = entry.with_lowering_witness(std::sync::Arc::new(RawFactoryWitness { linkage }));
    entry
}

#[cfg(feature = "native-artifact")]
fn linkage(entry: &ResidentKernelFactoryEntry) -> Option<&'static NativeLinkedFactory> {
    if let Some(witness) = entry.lowering_witness::<RawFactoryWitness>() {
        return Some(witness.linkage);
    }
    #[cfg(any(feature = "pure_text", feature = "map_ops"))]
    return super::native_collection_semantics::linkage(entry);
    #[cfg(not(any(feature = "pure_text", feature = "map_ops")))]
    None
}

/// Packaging metadata derived from a complete artifact or native-image closure.
/// This does not authorize operations, remove resource requirements, activate
/// a program, execute a kernel, or replace ordinary lowering/identity checks.
#[cfg(feature = "native-artifact")]
pub struct NativeFactoryLinkageProfile {
    basis: &'static str,
    operations: Box<[crate::OperationReference]>,
    factories: Option<Box<[&'static NativeLinkedFactory]>>,
    missing: Box<[crate::OperationReference]>,
}

#[cfg(feature = "native-artifact")]
impl NativeFactoryLinkageProfile {
    pub fn derive(artifact: &crate::ProgramArtifact, catalog: &mech_core::FunctionCatalog) -> Self {
        Self::from_operations(
            artifact.kernel_operation_references(),
            catalog,
            "artifact-kernels",
        )
    }

    /// Select factories for restoring an already-compiled image, not for
    /// lowering source. The compiler still uses the full source catalog before
    /// producing the checked Object. No original resource requirement is removed.
    pub fn derive_native_image(
        plan: &crate::native_artifact::NativeProgramArtifactPlan,
        catalog: &mech_core::FunctionCatalog,
    ) -> Self {
        Self::from_operations(
            plan.library_operation_references(),
            catalog,
            "native-image-libraries",
        )
    }

    fn from_operations(
        operations: Vec<crate::OperationReference>,
        catalog: &mech_core::FunctionCatalog,
        basis: &'static str,
    ) -> Self {
        let mut factories = Vec::with_capacity(operations.len());
        let mut missing = Vec::new();
        for operation in &operations {
            match catalog
                .resident_factory(&operation.module_path, &operation.operation_name)
                .and_then(linkage)
            {
                Some(factory) => factories.push(factory),
                None => missing.push(operation.clone()),
            }
        }
        Self {
            basis,
            operations: operations.into_boxed_slice(),
            factories: missing.is_empty().then(|| factories.into_boxed_slice()),
            missing: missing.into_boxed_slice(),
        }
    }

    pub fn operations(&self) -> &[crate::OperationReference] {
        &self.operations
    }

    pub const fn selection_basis(&self) -> &'static str {
        self.basis
    }

    /// None conservatively retains complete catalog installation. In
    /// particular, an unrecognized/custom factory is never silently skipped.
    pub fn factories(&self) -> Option<&[&'static NativeLinkedFactory]> {
        self.factories.as_deref()
    }

    pub fn missing_operations(&self) -> &[crate::OperationReference] {
        &self.missing
    }
}
