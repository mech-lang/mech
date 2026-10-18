//! Private registration evidence for maintained canonical collection leaves.
//!
//! The public opcode is descriptive, not authority. Only this module owns the
//! concrete witness, and every attachment below names the actual maintained
//! factory. Public factory semantic/cost replacements clear that attachment.
use mech_core::{
    FunctionCatalogBuilder, ImplementationMemoryClass, MResult, ResidentKernelFactory,
    ResidentKernelFactoryEntry, ResidentOperationKey,
};
#[cfg(feature = "native-artifact")]
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeCollectionPrimitive {
    #[cfg(feature = "pure_text")]
    TextLines,
    #[cfg(feature = "pure_text")]
    TextJoin,
    #[cfg(feature = "pure_text")]
    SequenceLength,
    #[cfg(feature = "pure_text")]
    NumberFormat,
    #[cfg(feature = "map_ops")]
    StringHistogram,
    #[cfg(feature = "map_ops")]
    StringMapKeys,
    #[cfg(feature = "map_ops")]
    StringMapValues,
}

#[cfg(feature = "native-artifact")]
struct FactoryWitness {
    primitive: NativeCollectionPrimitive,
}

fn register(
    builder: &mut FunctionCatalogBuilder,
    module: &str,
    name: &str,
    memory: ImplementationMemoryClass,
    factory: ResidentKernelFactory,
    primitive: NativeCollectionPrimitive,
) -> MResult<()> {
    let key = ResidentOperationKey::new(Box::new([module.into()]), name.into())
        .expect("maintained collection registration has a valid key");
    let entry = ResidentKernelFactoryEntry::new(key, memory, factory);
    // No native consumer exists in a resident-only build. Keep the actual
    // factory unchanged without allocating unused native evidence there.
    #[cfg(not(feature = "native-artifact"))]
    let _ = primitive;
    #[cfg(feature = "native-artifact")]
    let entry = entry.with_lowering_witness(Arc::new(FactoryWitness { primitive }));
    builder.insert_resident_factory_entry(entry)
}

#[cfg(feature = "pure_text")]
pub(super) fn register_text(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    register(
        builder,
        "text",
        "lines",
        ImplementationMemoryClass::CanonicalFinalize,
        super::text_ops::bind_lines,
        NativeCollectionPrimitive::TextLines,
    )?;
    register(
        builder,
        "text",
        "join",
        ImplementationMemoryClass::CanonicalFinalize,
        super::text_ops::bind_join,
        NativeCollectionPrimitive::TextJoin,
    )
}

#[cfg(feature = "pure_text")]
pub(super) fn register_format_and_length(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    register(
        builder,
        "number",
        "format",
        ImplementationMemoryClass::CanonicalFinalize,
        super::pure_text::bind_format,
        NativeCollectionPrimitive::NumberFormat,
    )?;
    register(
        builder,
        "sequence",
        "length",
        ImplementationMemoryClass::CanonicalFinalize,
        super::pure_text::bind_length,
        NativeCollectionPrimitive::SequenceLength,
    )
}

#[cfg(feature = "map_ops")]
pub(super) fn register_maps(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    use crate::intrinsics::map_ops::Operation;
    use mech_core::{BoundResidentKernel, ResidentKernelBindError, ResidentKernelBindRequest};
    fn histogram(
        request: &ResidentKernelBindRequest<'_>,
    ) -> Result<BoundResidentKernel, ResidentKernelBindError> {
        super::map_ops::bind(request, Operation::Histogram)
    }
    fn keys(
        request: &ResidentKernelBindRequest<'_>,
    ) -> Result<BoundResidentKernel, ResidentKernelBindError> {
        super::map_ops::bind(request, Operation::Keys)
    }
    fn values(
        request: &ResidentKernelBindRequest<'_>,
    ) -> Result<BoundResidentKernel, ResidentKernelBindError> {
        super::map_ops::bind(request, Operation::Values)
    }
    register(
        builder,
        "map",
        "histogram",
        Operation::Histogram.memory(),
        histogram,
        NativeCollectionPrimitive::StringHistogram,
    )?;
    register(
        builder,
        "map",
        "keys",
        Operation::Keys.memory(),
        keys,
        NativeCollectionPrimitive::StringMapKeys,
    )?;
    register(
        builder,
        "map",
        "values",
        Operation::Values.memory(),
        values,
        NativeCollectionPrimitive::StringMapValues,
    )
}

#[cfg(feature = "native-artifact")]
pub(crate) fn primitive(entry: &ResidentKernelFactoryEntry) -> Option<NativeCollectionPrimitive> {
    entry
        .lowering_witness::<FactoryWitness>()
        .map(|witness| witness.primitive)
}
