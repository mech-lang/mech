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

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
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

// The complete catalog and the selected linked catalog install the same
// declarations. Each declaration references only its own actual binder.
pub mod linkage_factories {
    use super::*;
    use crate::resident::native_factory_linkage::NativeLinkedFactory;

    macro_rules! factory {
        ($token:ident, $module:expr, $name:literal, $memory:expr, $binder:expr, $primitive:expr) => {
            factory!(@features $token, &["pure_text"], $module, $name, $memory, $binder, $primitive);
        };
        (@features $token:ident, $features:expr, $module:expr, $name:literal, $memory:expr, $binder:expr, $primitive:expr) => {
            pub const $token: NativeLinkedFactory = NativeLinkedFactory::new(
                concat!("mech_engine::linked_factories::", stringify!($token)),
                $features,
                || super::make_entry($module, $name, $memory, $binder, $primitive, &$token),
            );
        };
    }
    use ImplementationMemoryClass::{CanonicalFinalize, NoAdditionalScratch};

    factory!(
        CORE_COMPOSITE_PACK,
        &["core"],
        "composite-pack",
        CanonicalFinalize,
        super::super::composite::bind_composite_pack,
        NativeCollectionPrimitive::CompositePack(0)
    );
    factory!(
        CORE_ENUM_PACK,
        &["core"],
        "enum-pack",
        CanonicalFinalize,
        super::super::composite::bind_enum_pack,
        NativeCollectionPrimitive::EnumPack
    );
    factory!(
        RANGE_EXCLUSIVE,
        &["range"],
        "exclusive",
        NoAdditionalScratch,
        super::super::numeric::bind_range_exclusive,
        NativeCollectionPrimitive::RangeExclusive
    );
    factory!(
        RANGE_INCLUSIVE,
        &["range"],
        "inclusive",
        NoAdditionalScratch,
        super::super::numeric::bind_range_inclusive,
        NativeCollectionPrimitive::RangeInclusive
    );
    factory!(
        RANGE_EXCLUSIVE_INCREMENT,
        &["range"],
        "exclusive-increment",
        NoAdditionalScratch,
        super::super::numeric::bind_range_increment_exclusive,
        NativeCollectionPrimitive::RangeIncrementExclusive
    );
    factory!(
        RANGE_INCLUSIVE_INCREMENT,
        &["range"],
        "inclusive-increment",
        NoAdditionalScratch,
        super::super::numeric::bind_range_increment_inclusive,
        NativeCollectionPrimitive::RangeIncrementInclusive
    );
    factory!(
        ACCESS_SCALAR,
        &["access"],
        "scalar",
        CanonicalFinalize,
        super::super::numeric::bind_semantic_scalar_access,
        NativeCollectionPrimitive::StringScalarAccess
    );
    factory!(
        ACCESS_RANGE,
        &["access"],
        "range",
        ImplementationMemoryClass::PackedStringMatrixOrCanonicalFinalize,
        super::super::numeric::bind_semantic_range_access,
        NativeCollectionPrimitive::StringRangeAccess
    );
    #[cfg(feature = "convert")]
    factory!(
        @features CONVERT_KIND, &["pure_text", "convert"],
        &["convert"],
        "kind",
        CanonicalFinalize,
        super::super::conversion::bind_kind_conversion,
        NativeCollectionPrimitive::MatrixConversion
    );
    #[cfg(feature = "convert")]
    factory!(
        @features CONVERT_SET, &["pure_text", "convert"],
        &["convert"],
        "set",
        ImplementationMemoryClass::CanonicalSortUnique,
        super::super::set_conversion::bind,
        NativeCollectionPrimitive::MatrixToSetConversion
    );
    factory!(
        SET_ELEMENT_OF,
        &["set"],
        "element-of",
        NoAdditionalScratch,
        super::super::set::bind_element_of,
        NativeCollectionPrimitive::SetContains
    );
    factory!(
        SET_NOT_ELEMENT_OF,
        &["set"],
        "not-element-of",
        NoAdditionalScratch,
        super::super::set::bind_not_element_of,
        NativeCollectionPrimitive::SetNotContains
    );
    factory!(
        COMPARE_EQ,
        &["compare"],
        "eq",
        NoAdditionalScratch,
        super::super::numeric::bind_semantic_equal,
        NativeCollectionPrimitive::StringEqual
    );
    factory!(
        COMPARE_NEQ,
        &["compare"],
        "neq",
        NoAdditionalScratch,
        super::super::numeric::bind_semantic_not_equal,
        NativeCollectionPrimitive::StringNotEqual
    );
    factory!(
        STRING_CONCAT,
        &["string"],
        "concat",
        NoAdditionalScratch,
        super::super::text::bind_concat,
        NativeCollectionPrimitive::StringConcat
    );
    factory!(
        STATS_SUM_COLUMN,
        &["stats", "sum"],
        "column",
        NoAdditionalScratch,
        super::super::numeric::bind_sum_columns,
        NativeCollectionPrimitive::SumColumns
    );
    factory!(
        STATS_SUM_ROW,
        &["stats", "sum"],
        "row",
        NoAdditionalScratch,
        super::super::numeric::bind_sum_rows,
        NativeCollectionPrimitive::SumRows
    );
    #[cfg(feature = "sequence_sort")]
    factory!(
        @features SEQUENCE_SORT_INDICES, &["pure_text", "sequence_sort"],
        &["sequence"],
        "sort-indices",
        CanonicalFinalize,
        super::super::sequence_sort::bind,
        NativeCollectionPrimitive::SortIndices
    );
    #[cfg(feature = "pure_text")]
    factory!(
        TEXT_LINES,
        &["text"],
        "lines",
        NoAdditionalScratch,
        super::super::text_ops::bind_lines,
        NativeCollectionPrimitive::TextLines
    );
    #[cfg(feature = "pure_text")]
    factory!(
        TEXT_JOIN,
        &["text"],
        "join",
        CanonicalFinalize,
        super::super::text_ops::bind_join,
        NativeCollectionPrimitive::TextJoin
    );
    #[cfg(feature = "pure_text")]
    factory!(
        NUMBER_FORMAT,
        &["number"],
        "format",
        CanonicalFinalize,
        super::super::pure_text::bind_format,
        NativeCollectionPrimitive::NumberFormat
    );
    #[cfg(feature = "pure_text")]
    factory!(
        SEQUENCE_LENGTH,
        &["sequence"],
        "length",
        CanonicalFinalize,
        super::super::pure_text::bind_length,
        NativeCollectionPrimitive::SequenceLength
    );
    #[cfg(feature = "map_ops")]
    factory!(
        @features MAP_NEW, &["map_ops"], &["map"], "new", CanonicalFinalize,
        super::super::composite::bind_composite_pack,
        NativeCollectionPrimitive::CompositePack(0)
    );
    #[cfg(feature = "map_ops")]
    factory!(
        @features MAP_HISTOGRAM, &["map_ops"],
        &["map"],
        "histogram",
        crate::intrinsics::map_ops::Operation::Histogram.memory(),
        super::bind_histogram,
        NativeCollectionPrimitive::StringHistogram
    );
    #[cfg(feature = "map_ops")]
    factory!(
        @features MAP_KEYS, &["map_ops"],
        &["map"],
        "keys",
        crate::intrinsics::map_ops::Operation::Keys.memory(),
        super::bind_keys,
        NativeCollectionPrimitive::StringMapKeys
    );
    #[cfg(feature = "map_ops")]
    factory!(
        @features MAP_VALUES, &["map_ops"],
        &["map"],
        "values",
        crate::intrinsics::map_ops::Operation::Values.memory(),
        super::bind_values,
        NativeCollectionPrimitive::StringMapValues
    );
}

use linkage_factories::*;

pub(super) fn register_composite_pack(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    CORE_COMPOSITE_PACK.install(builder)
}

pub(super) fn register_enum_pack(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    CORE_ENUM_PACK.install(builder)
}

pub(super) fn register_ranges(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    for factory in [
        &RANGE_EXCLUSIVE,
        &RANGE_INCLUSIVE,
        &RANGE_EXCLUSIVE_INCREMENT,
        &RANGE_INCLUSIVE_INCREMENT,
    ] {
        factory.install(builder)?;
    }
    Ok(())
}

pub(super) fn register_access(builder: &mut FunctionCatalogBuilder, scalar: bool) -> MResult<()> {
    if scalar {
        ACCESS_SCALAR.install(builder)
    } else {
        ACCESS_RANGE.install(builder)
    }
}

#[cfg(feature = "convert")]
pub(super) fn register_conversion(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    CONVERT_KIND.install(builder)
}

#[cfg(feature = "convert")]
pub(super) fn register_set_conversion(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    CONVERT_SET.install(builder)
}

pub(super) fn register_set_membership(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    SET_ELEMENT_OF.install(builder)?;
    SET_NOT_ELEMENT_OF.install(builder)
}

pub(super) fn register_string_comparisons(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    COMPARE_EQ.install(builder)?;
    COMPARE_NEQ.install(builder)
}

pub(super) fn register_string_concat(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    STRING_CONCAT.install(builder)
}

pub(super) fn register_sums(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    STATS_SUM_COLUMN.install(builder)?;
    STATS_SUM_ROW.install(builder)
}

#[cfg(feature = "sequence_sort")]
pub(super) fn register_sort(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    SEQUENCE_SORT_INDICES.install(builder)
}

#[cfg(feature = "native-artifact")]
struct FactoryWitness {
    primitive: NativeCollectionPrimitive,
    linkage: &'static super::native_factory_linkage::NativeLinkedFactory,
}

fn make_entry(
    module: &[&str],
    name: &str,
    memory: ImplementationMemoryClass,
    factory: ResidentKernelFactory,
    primitive: NativeCollectionPrimitive,
    linkage: &'static super::native_factory_linkage::NativeLinkedFactory,
) -> ResidentKernelFactoryEntry {
    let key = ResidentOperationKey::new(
        module.iter().map(|part| (*part).into()).collect(),
        name.into(),
    )
    .expect("maintained collection registration has a valid key");
    let entry = ResidentKernelFactoryEntry::new(key, memory, factory);
    // Retain the owner's ONE private certificate, including its original
    // lowering evidence. Public semantic/cost replacements invalidate both.
    #[cfg(not(feature = "native-artifact"))]
    let _ = (primitive, linkage);
    #[cfg(feature = "native-artifact")]
    let entry = entry.with_lowering_witness(Arc::new(FactoryWitness { primitive, linkage }));
    entry
}

#[cfg(feature = "pure_text")]
pub(super) fn register_text(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    // The compact lines constructor has no generic repacking workspace; its
    // leaf still admits the complete real container, payload and shape scratch.
    TEXT_LINES.install(builder)?;
    TEXT_JOIN.install(builder)
}

#[cfg(feature = "pure_text")]
pub(super) fn register_format_and_length(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    NUMBER_FORMAT.install(builder)?;
    SEQUENCE_LENGTH.install(builder)
}

#[cfg(feature = "map_ops")]
fn bind_histogram(
    request: &mech_core::ResidentKernelBindRequest<'_>,
) -> Result<mech_core::BoundResidentKernel, mech_core::ResidentKernelBindError> {
    super::map_ops::bind(request, crate::intrinsics::map_ops::Operation::Histogram)
}

#[cfg(feature = "map_ops")]
fn bind_keys(
    request: &mech_core::ResidentKernelBindRequest<'_>,
) -> Result<mech_core::BoundResidentKernel, mech_core::ResidentKernelBindError> {
    super::map_ops::bind(request, crate::intrinsics::map_ops::Operation::Keys)
}

#[cfg(feature = "map_ops")]
fn bind_values(
    request: &mech_core::ResidentKernelBindRequest<'_>,
) -> Result<mech_core::BoundResidentKernel, mech_core::ResidentKernelBindError> {
    super::map_ops::bind(request, crate::intrinsics::map_ops::Operation::Values)
}

#[cfg(feature = "map_ops")]
pub(super) fn register_maps(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    MAP_NEW.install(builder)?;
    // General functional entry insertion currently uses the admitted resident
    // kernel. A native lowering witness is a separate qualification step.
    builder.insert_resident_factory(
        ["map"],
        "insert",
        ImplementationMemoryClass::CanonicalFinalize,
        super::numeric::bind_map_insert,
    )?;
    MAP_HISTOGRAM.install(builder)?;
    MAP_KEYS.install(builder)?;
    MAP_VALUES.install(builder)
}

#[cfg(feature = "native-artifact")]
pub(crate) fn primitive(entry: &ResidentKernelFactoryEntry) -> Option<NativeCollectionPrimitive> {
    entry
        .lowering_witness::<FactoryWitness>()
        .map(|witness| witness.primitive)
}

#[cfg(feature = "native-artifact")]
pub(super) fn linkage(
    entry: &ResidentKernelFactoryEntry,
) -> Option<&'static super::native_factory_linkage::NativeLinkedFactory> {
    entry
        .lowering_witness::<FactoryWitness>()
        .map(|witness| witness.linkage)
}
