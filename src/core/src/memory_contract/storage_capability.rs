//! Descriptive storage capabilities and pure schema compatibility checks.

use crate::{
    ExtentEvolution, MemoryTopology, PayloadAccounting, PopulationAccounting, ResolvedMemoryExtent,
    ResolvedTypeMemoryContract, ScalarMemoryKind, Schema, SchemaBody, SemanticModelError,
    ShapeInstance,
};

#[cfg(feature = "no_std")]
use alloc::boxed::Box;
#[cfg(not(feature = "no_std"))]
use std::boxed::Box;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageElementKind {
    CanonicalValue,
    Scalar(ScalarMemoryKind),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageTopology {
    Opaque,
    CanonicalValue,
    Scalar(ScalarMemoryKind),
    Tagged,
    Product,
    DenseSequence { element: StorageElementKind },
    Columnar,
    OrderedSet,
    OrderedMap,
    ReifiedType,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StorageExtentCapability {
    Any,
    Single,
    FixedArity(u64),
    FixedDimensions(Box<[u64]>),
    ResizableDimensions(Box<[Option<u64>]>),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PositionalAddressingCapability {
    None,
    Rank(u64),
    AnyRank,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StorageAddressingCapabilities {
    pub whole_value: bool,
    pub positional: PositionalAddressingCapability,
    pub named_members: bool,
    pub keyed_members: bool,
    pub arbitrary_regions: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StorageCanonicalizationCapabilities {
    pub self_describing: bool,
    pub recursive: bool,
    pub tagged: bool,
    pub ordered_keys: bool,
    pub unique_keys: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StorageAccessCapabilities {
    pub readable: bool,
    pub writable: bool,
    pub replaceable: bool,
    pub region_mutable: bool,
    pub canonical_snapshot: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StorageOwnershipCapabilities {
    pub shared_read: bool,
    pub exclusive_write: bool,
    pub owned_value: bool,
    pub detachable: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StoragePublicationCapabilities {
    pub atomic_replace: bool,
    pub preserves_previous_on_failure: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageAccountingCapability {
    FixedScalar,
    CanonicalSnapshot,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageCapabilityDescriptor {
    pub topology: StorageTopology,
    pub extent: StorageExtentCapability,
    pub addressing: StorageAddressingCapabilities,
    pub canonicalization: StorageCanonicalizationCapabilities,
    pub access: StorageAccessCapabilities,
    pub ownership: StorageOwnershipCapabilities,
    pub publication: StoragePublicationCapabilities,
    pub accounting: StorageAccountingCapability,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageCompatibilityError {
    OpaqueStorage,
    TopologyMismatch,
    ScalarKindMismatch,
    DenseElementMismatch,
    ExtentKindMismatch,
    RankMismatch,
    AxisMismatch,
    DynamicAxisUnsupported,
    PositionalAddressingUnsupported,
    NamedAddressingUnsupported,
    KeyedAddressingUnsupported,
    CanonicalizationUnsupported,
    AccountingUnsupported,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SchemaStorageCompatibilityError {
    Semantic(SemanticModelError),
    Storage(StorageCompatibilityError),
}

impl From<SemanticModelError> for SchemaStorageCompatibilityError {
    fn from(error: SemanticModelError) -> Self {
        Self::Semantic(error)
    }
}

impl From<StorageCompatibilityError> for SchemaStorageCompatibilityError {
    fn from(error: StorageCompatibilityError) -> Self {
        Self::Storage(error)
    }
}

impl core::fmt::Display for StorageCompatibilityError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(match self {
            Self::OpaqueStorage => "storage capabilities are opaque",
            Self::TopologyMismatch => "storage topology does not satisfy the semantic topology",
            Self::ScalarKindMismatch => {
                "storage scalar kind does not match the semantic scalar kind"
            }
            Self::DenseElementMismatch => {
                "dense storage element kind does not match the matrix element schema"
            }
            Self::ExtentKindMismatch => "storage extent kind does not satisfy the resolved extent",
            Self::RankMismatch => "storage rank does not match the resolved rank",
            Self::AxisMismatch => "storage axis does not match the resolved axis",
            Self::DynamicAxisUnsupported => {
                "fixed storage axis cannot satisfy a turn-varying extent"
            }
            Self::PositionalAddressingUnsupported => {
                "storage does not provide the required positional addressing"
            }
            Self::NamedAddressingUnsupported => {
                "storage does not provide required named-member addressing"
            }
            Self::KeyedAddressingUnsupported => {
                "storage does not provide required keyed-member addressing"
            }
            Self::CanonicalizationUnsupported => {
                "storage does not satisfy the semantic canonicalization obligations"
            }
            Self::AccountingUnsupported => {
                "storage accounting does not satisfy the semantic accounting obligations"
            }
        })
    }
}

#[cfg(any(not(feature = "no_std"), feature = "std"))]
impl std::error::Error for StorageCompatibilityError {}

fn scalar_kind(body: &SchemaBody) -> Option<ScalarMemoryKind> {
    match body {
        SchemaBody::Bool => Some(ScalarMemoryKind::Bool),
        SchemaBody::UnsignedInteger(width) => Some(ScalarMemoryKind::Unsigned(*width)),
        SchemaBody::SignedInteger(width) => Some(ScalarMemoryKind::Signed(*width)),
        SchemaBody::FloatingPoint(width) => Some(ScalarMemoryKind::Floating(*width)),
        SchemaBody::Complex(width) => Some(ScalarMemoryKind::Complex(*width)),
        SchemaBody::Rational64 => Some(ScalarMemoryKind::Rational64),
        SchemaBody::String => Some(ScalarMemoryKind::String),
        SchemaBody::Id => Some(ScalarMemoryKind::Id),
        SchemaBody::Index => Some(ScalarMemoryKind::Index),
        SchemaBody::Atom(_) => Some(ScalarMemoryKind::Atom),
        _ => None,
    }
}

fn check_topology(
    schema: &SchemaBody,
    semantic: MemoryTopology,
    storage: StorageTopology,
) -> Result<(), StorageCompatibilityError> {
    if storage == StorageTopology::Opaque {
        return Err(StorageCompatibilityError::OpaqueStorage);
    }
    if storage == StorageTopology::CanonicalValue {
        return Ok(());
    }

    match (semantic, storage) {
        (MemoryTopology::Scalar(expected), StorageTopology::Scalar(found)) => {
            if expected == found {
                Ok(())
            } else {
                Err(StorageCompatibilityError::ScalarKindMismatch)
            }
        }
        (MemoryTopology::Tagged { .. }, StorageTopology::Tagged)
        | (MemoryTopology::Product { .. }, StorageTopology::Product)
        | (MemoryTopology::Columnar { .. }, StorageTopology::Columnar)
        | (MemoryTopology::OrderedSet, StorageTopology::OrderedSet)
        | (MemoryTopology::OrderedMap, StorageTopology::OrderedMap)
        | (MemoryTopology::ReifiedType, StorageTopology::ReifiedType) => Ok(()),
        (
            MemoryTopology::DenseSequence { .. },
            StorageTopology::DenseSequence { element: found },
        ) => match found {
            StorageElementKind::CanonicalValue => Ok(()),
            StorageElementKind::Scalar(found) => {
                let SchemaBody::Matrix { element, .. } = schema else {
                    return Err(StorageCompatibilityError::DenseElementMismatch);
                };
                if scalar_kind(element) == Some(found) {
                    Ok(())
                } else {
                    Err(StorageCompatibilityError::DenseElementMismatch)
                }
            }
        },
        _ => Err(StorageCompatibilityError::TopologyMismatch),
    }
}

fn fixed_axis_accepts(evolution: ExtentEvolution) -> bool {
    matches!(
        evolution,
        ExtentEvolution::Fixed | ExtentEvolution::ActivationFixed
    )
}

fn check_extent(
    semantic: &ResolvedMemoryExtent,
    storage: &StorageExtentCapability,
) -> Result<(), StorageCompatibilityError> {
    match storage {
        StorageExtentCapability::Any => Ok(()),
        StorageExtentCapability::Single if semantic == &ResolvedMemoryExtent::Single => Ok(()),
        StorageExtentCapability::Single => Err(StorageCompatibilityError::ExtentKindMismatch),
        StorageExtentCapability::FixedArity(found) => match semantic {
            ResolvedMemoryExtent::FixedArity(required) if required == found => Ok(()),
            _ => Err(StorageCompatibilityError::ExtentKindMismatch),
        },
        StorageExtentCapability::FixedDimensions(found) => {
            let ResolvedMemoryExtent::Dimensions(required) = semantic else {
                return Err(StorageCompatibilityError::ExtentKindMismatch);
            };
            if required.len() != found.len() {
                return Err(StorageCompatibilityError::RankMismatch);
            }
            for (required, found) in required.iter().zip(found) {
                if required.value != *found {
                    return Err(StorageCompatibilityError::AxisMismatch);
                }
                if !fixed_axis_accepts(required.evolution) {
                    return Err(StorageCompatibilityError::DynamicAxisUnsupported);
                }
            }
            Ok(())
        }
        StorageExtentCapability::ResizableDimensions(pattern) => {
            let ResolvedMemoryExtent::Dimensions(required) = semantic else {
                return Err(StorageCompatibilityError::ExtentKindMismatch);
            };
            if required.len() != pattern.len() {
                return Err(StorageCompatibilityError::RankMismatch);
            }
            for (required, fixed) in required.iter().zip(pattern) {
                if let Some(fixed) = fixed {
                    if required.value != *fixed {
                        return Err(StorageCompatibilityError::AxisMismatch);
                    }
                    if !fixed_axis_accepts(required.evolution) {
                        return Err(StorageCompatibilityError::DynamicAxisUnsupported);
                    }
                }
            }
            Ok(())
        }
    }
}

fn check_addressing(
    contract: &ResolvedTypeMemoryContract,
    storage: &StorageCapabilityDescriptor,
) -> Result<(), StorageCompatibilityError> {
    if let Some(required) = contract.addressing.positional_rank {
        match storage.addressing.positional {
            PositionalAddressingCapability::AnyRank => {}
            PositionalAddressingCapability::Rank(found) if found >= required => {}
            _ => return Err(StorageCompatibilityError::PositionalAddressingUnsupported),
        }
    }
    if contract.addressing.named_members && !storage.addressing.named_members {
        return Err(StorageCompatibilityError::NamedAddressingUnsupported);
    }
    if contract.addressing.keyed_members && !storage.addressing.keyed_members {
        return Err(StorageCompatibilityError::KeyedAddressingUnsupported);
    }
    Ok(())
}

fn check_canonicalization(
    contract: &ResolvedTypeMemoryContract,
    storage: &StorageCapabilityDescriptor,
) -> Result<(), StorageCompatibilityError> {
    let required = contract.canonicalization;
    let found = storage.canonicalization;
    if (required.self_describing && !found.self_describing)
        || (required.recursive && !found.recursive)
        || (required.tagged && !found.tagged)
        || (required.ordered_keys && !found.ordered_keys)
        || (required.unique_keys && !found.unique_keys)
    {
        Err(StorageCompatibilityError::CanonicalizationUnsupported)
    } else {
        Ok(())
    }
}

fn check_accounting(
    contract: &ResolvedTypeMemoryContract,
    storage: &StorageCapabilityDescriptor,
) -> Result<(), StorageCompatibilityError> {
    match storage.accounting {
        StorageAccountingCapability::CanonicalSnapshot => Ok(()),
        StorageAccountingCapability::FixedScalar
            if matches!(contract.topology, MemoryTopology::Scalar(_))
                && contract.accounting.payload == PayloadAccounting::FixedWidth
                && contract.accounting.population == PopulationAccounting::Single =>
        {
            Ok(())
        }
        StorageAccountingCapability::FixedScalar => {
            Err(StorageCompatibilityError::AccountingUnsupported)
        }
    }
}

pub(crate) fn check_resolved_type_storage_compatibility(
    schema: &SchemaBody,
    contract: &ResolvedTypeMemoryContract,
    storage: &StorageCapabilityDescriptor,
) -> Result<(), StorageCompatibilityError> {
    check_topology(schema, contract.topology, storage.topology)?;
    check_extent(&contract.extent, &storage.extent)?;
    check_addressing(contract, storage)?;
    check_canonicalization(contract, storage)?;
    check_accounting(contract, storage)
}

pub fn check_schema_storage_compatibility(
    schema: &Schema,
    shape: &ShapeInstance,
    storage: &StorageCapabilityDescriptor,
) -> Result<(), SchemaStorageCompatibilityError> {
    let contract = schema.resolved_type_memory_contract(shape)?;
    check_resolved_type_storage_compatibility(schema.body(), &contract, storage)?;
    Ok(())
}
