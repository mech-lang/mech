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
    CompositePack(usize),
    EnumPack,
    RangeExclusive,
    RangeInclusive,
    RangeIncrementExclusive,
    RangeIncrementInclusive,
    StringScalarAccess,
    #[cfg(feature = "native-artifact")]
    MatrixScalarAccess,
    #[cfg(feature = "native-artifact")]
    MatrixRangeAccess,
    #[cfg(feature = "native-artifact")]
    MapKeyAccess,
    SetContains,
    SetNotContains,
    StringEqual,
    StringNotEqual,
    StringConcat,
    SumColumns,
    SumRows,
    #[cfg(feature = "sequence_sort")]
    SortIndices,
    StringRangeAccess,
    #[cfg(feature = "native-artifact")]
    StringAllAccess,
    #[cfg(feature = "convert")]
    MatrixConversion,
    #[cfg(feature = "convert")]
    MatrixToSetConversion,
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

pub(super) fn register_composite_pack(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    register(
        builder,
        "core",
        "composite-pack",
        ImplementationMemoryClass::CanonicalFinalize,
        super::composite::bind_composite_pack,
        NativeCollectionPrimitive::CompositePack(0),
    )
}

pub(super) fn register_enum_pack(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    register(
        builder,
        "core",
        "enum-pack",
        ImplementationMemoryClass::CanonicalFinalize,
        super::composite::bind_enum_pack,
        NativeCollectionPrimitive::EnumPack,
    )
}

pub(super) fn register_ranges(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    let factories: [(&str, ResidentKernelFactory, NativeCollectionPrimitive); 4] = [
        (
            "exclusive",
            super::numeric::bind_range_exclusive,
            NativeCollectionPrimitive::RangeExclusive,
        ),
        (
            "inclusive",
            super::numeric::bind_range_inclusive,
            NativeCollectionPrimitive::RangeInclusive,
        ),
        (
            "exclusive-increment",
            super::numeric::bind_range_increment_exclusive,
            NativeCollectionPrimitive::RangeIncrementExclusive,
        ),
        (
            "inclusive-increment",
            super::numeric::bind_range_increment_inclusive,
            NativeCollectionPrimitive::RangeIncrementInclusive,
        ),
    ];
    for (name, factory, primitive) in factories {
        register(
            builder,
            "range",
            name,
            ImplementationMemoryClass::NoAdditionalScratch,
            factory,
            primitive,
        )?;
    }
    Ok(())
}

pub(super) fn register_access(builder: &mut FunctionCatalogBuilder, scalar: bool) -> MResult<()> {
    let (name, factory, primitive): (_, ResidentKernelFactory, _) = if scalar {
        (
            "scalar",
            super::numeric::bind_semantic_scalar_access,
            NativeCollectionPrimitive::StringScalarAccess,
        )
    } else {
        (
            "range",
            super::numeric::bind_semantic_range_access,
            NativeCollectionPrimitive::StringRangeAccess,
        )
    };
    register(
        builder,
        "access",
        name,
        if scalar {
            ImplementationMemoryClass::CanonicalFinalize
        } else {
            ImplementationMemoryClass::PackedStringMatrixOrCanonicalFinalize
        },
        factory,
        primitive,
    )
}

#[cfg(feature = "convert")]
pub(super) fn register_conversion(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    register(
        builder,
        "convert",
        "kind",
        ImplementationMemoryClass::CanonicalFinalize,
        super::conversion::bind_kind_conversion,
        NativeCollectionPrimitive::MatrixConversion,
    )
}

#[cfg(feature = "convert")]
pub(super) fn register_set_conversion(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    register(
        builder,
        "convert",
        "set",
        ImplementationMemoryClass::CanonicalSortUnique,
        super::set_conversion::bind,
        NativeCollectionPrimitive::MatrixToSetConversion,
    )
}

pub(super) fn register_set_membership(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    register(
        builder,
        "set",
        "element-of",
        ImplementationMemoryClass::NoAdditionalScratch,
        super::set::bind_element_of,
        NativeCollectionPrimitive::SetContains,
    )?;
    register(
        builder,
        "set",
        "not-element-of",
        ImplementationMemoryClass::NoAdditionalScratch,
        super::set::bind_not_element_of,
        NativeCollectionPrimitive::SetNotContains,
    )
}

pub(super) fn register_string_comparisons(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    register(
        builder,
        "compare",
        "eq",
        ImplementationMemoryClass::NoAdditionalScratch,
        super::numeric::bind_semantic_equal,
        NativeCollectionPrimitive::StringEqual,
    )?;
    register(
        builder,
        "compare",
        "neq",
        ImplementationMemoryClass::NoAdditionalScratch,
        super::numeric::bind_semantic_not_equal,
        NativeCollectionPrimitive::StringNotEqual,
    )
}

pub(super) fn register_string_concat(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    register(
        builder,
        "string",
        "concat",
        ImplementationMemoryClass::NoAdditionalScratch,
        super::text::bind_concat,
        NativeCollectionPrimitive::StringConcat,
    )
}

pub(super) fn register_sums(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    for (name, factory, primitive) in [
        (
            "column",
            super::numeric::bind_sum_columns as ResidentKernelFactory,
            NativeCollectionPrimitive::SumColumns,
        ),
        (
            "row",
            super::numeric::bind_sum_rows as ResidentKernelFactory,
            NativeCollectionPrimitive::SumRows,
        ),
    ] {
        register_path(
            builder,
            &["stats", "sum"],
            name,
            ImplementationMemoryClass::NoAdditionalScratch,
            factory,
            primitive,
        )?;
    }
    Ok(())
}

#[cfg(feature = "sequence_sort")]
pub(super) fn register_sort(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    register(
        builder,
        "sequence",
        "sort-indices",
        ImplementationMemoryClass::CanonicalFinalize,
        super::sequence_sort::bind,
        NativeCollectionPrimitive::SortIndices,
    )
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
    register_path(builder, &[module], name, memory, factory, primitive)
}

fn register_path(
    builder: &mut FunctionCatalogBuilder,
    module: &[&str],
    name: &str,
    memory: ImplementationMemoryClass,
    factory: ResidentKernelFactory,
    primitive: NativeCollectionPrimitive,
) -> MResult<()> {
    let key = ResidentOperationKey::new(
        module.iter().map(|part| (*part).into()).collect(),
        name.into(),
    )
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
        // The checked compact String constructor consumes its owned cells;
        // there is no generic draft/repacking finalizer workspace. The leaf
        // admits its complete actual container, payload and shape scratch
        // before allocation; the common planner still stages publication.
        ImplementationMemoryClass::NoAdditionalScratch,
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
