//! Memory-facing requirements derived from declared operation ports.

use crate::{
    AccessMode, AddressingContract, AliasPolicy, ChangeDetectionPolicy, DeliveryMode,
    InputPortPolicy, MemoryTopology, OperationContractDeclaration, OperationContractError,
    OutputConstruction, OutputPortPolicy, PositionalAddressingCapability, RegionPolicy,
    ResolvedTypeMemoryContract, Schema, SchemaStorageCompatibilityError, ShapeInstance,
    StorageCapabilityDescriptor,
};

#[cfg(feature = "no_std")]
use alloc::{boxed::Box, vec::Vec};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, vec::Vec};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OwnershipRequirement {
    SharedRead,
    ExclusiveWrite,
    OwnedValue,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AddressingRequirement {
    WholeValue,
    Positional { minimum_rank: u64 },
    CollectionEntry,
    ArbitraryRegion,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublicationRequirement {
    None,
    AtomicReplace,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PortMemoryRequirement {
    pub access: AccessMode,
    pub delivery: DeliveryMode,
    pub ownership: OwnershipRequirement,
    pub construction: Option<OutputConstruction>,
    pub addressing: AddressingRequirement,
    pub alias: Option<AliasPolicy>,
    pub publication: PublicationRequirement,
    pub change_detection: Option<ChangeDetectionPolicy>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationMemoryRequirements {
    pub inputs: Box<[PortMemoryRequirement]>,
    pub outputs: Box<[PortMemoryRequirement]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PortStorageCompatibilityError {
    SchemaStorage(SchemaStorageCompatibilityError),
    SemanticAddressingUnsupported {
        required: AddressingRequirement,
        available: AddressingContract,
    },
    ReadUnsupported,
    WriteUnsupported,
    ReplaceUnsupported,
    RegionMutationUnsupported,
    SharedReadUnsupported,
    ExclusiveWriteUnsupported,
    OwnedValueUnsupported,
    WholeValueAddressingUnsupported,
    PositionalAddressingUnsupported {
        minimum_rank: u64,
        available: PositionalAddressingCapability,
    },
    CollectionEntryAddressingUnsupported,
    ArbitraryRegionUnsupported,
    AtomicPublicationUnsupported,
    FailureAtomicityUnsupported,
    CanonicalSnapshotUnsupported,
    ExactScalarChangeDetectionRequiresScalar,
}

impl From<SchemaStorageCompatibilityError> for PortStorageCompatibilityError {
    fn from(error: SchemaStorageCompatibilityError) -> Self {
        Self::SchemaStorage(error)
    }
}

impl core::fmt::Display for PortStorageCompatibilityError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::SchemaStorage(_) => formatter
                .write_str("schema-derived memory requirements are incompatible with storage"),
            Self::SemanticAddressingUnsupported {
                required,
                available,
            } => write!(
                formatter,
                "semantic addressing {:?} does not support operation requirement {:?}",
                available, required
            ),
            Self::ReadUnsupported => formatter.write_str("storage does not support reading"),
            Self::WriteUnsupported => formatter.write_str("storage does not support writing"),
            Self::ReplaceUnsupported => {
                formatter.write_str("storage does not support whole-value replacement")
            }
            Self::RegionMutationUnsupported => {
                formatter.write_str("storage does not support regional mutation")
            }
            Self::SharedReadUnsupported => {
                formatter.write_str("storage does not support shared-read ownership")
            }
            Self::ExclusiveWriteUnsupported => {
                formatter.write_str("storage does not support exclusive-write ownership")
            }
            Self::OwnedValueUnsupported => formatter
                .write_str("storage provides neither owned-value nor detachable-value access"),
            Self::WholeValueAddressingUnsupported => {
                formatter.write_str("storage does not support whole-value addressing")
            }
            Self::PositionalAddressingUnsupported {
                minimum_rank,
                available,
            } => write!(
                formatter,
                "storage positional addressing {:?} does not satisfy minimum rank {}",
                available, minimum_rank
            ),
            Self::CollectionEntryAddressingUnsupported => {
                formatter.write_str("storage does not support collection-entry addressing")
            }
            Self::ArbitraryRegionUnsupported => {
                formatter.write_str("storage does not support arbitrary-region addressing")
            }
            Self::AtomicPublicationUnsupported => {
                formatter.write_str("storage does not support atomic replacement publication")
            }
            Self::FailureAtomicityUnsupported => formatter
                .write_str("storage does not preserve the previous value when publication fails"),
            Self::CanonicalSnapshotUnsupported => {
                formatter.write_str("storage does not support canonical snapshots")
            }
            Self::ExactScalarChangeDetectionRequiresScalar => formatter
                .write_str("exact-scalar change detection requires a semantic scalar schema"),
        }
    }
}

#[cfg(any(not(feature = "no_std"), feature = "std"))]
impl std::error::Error for PortStorageCompatibilityError {}

impl OwnershipRequirement {
    fn from_access(access: AccessMode) -> Self {
        match access {
            AccessMode::Read => Self::SharedRead,
            AccessMode::Write | AccessMode::ReadWrite => Self::ExclusiveWrite,
            AccessMode::Consume => Self::OwnedValue,
        }
    }
}

impl PortMemoryRequirement {
    pub fn for_input(policy: InputPortPolicy) -> Self {
        Self {
            access: policy.access,
            delivery: policy.delivery,
            ownership: OwnershipRequirement::from_access(policy.access),
            construction: None,
            addressing: AddressingRequirement::WholeValue,
            alias: None,
            publication: PublicationRequirement::None,
            change_detection: None,
        }
    }

    pub fn for_output(policy: &OutputPortPolicy) -> Result<Self, OperationContractError> {
        let addressing = match &policy.construction {
            OutputConstruction::FullWrite { .. }
            | OutputConstruction::Replace { .. }
            | OutputConstruction::Build { .. }
            | OutputConstruction::ReadModifyWrite {
                regions: RegionPolicy::WholeValue,
                ..
            } => AddressingRequirement::WholeValue,
            OutputConstruction::ReadModifyWrite {
                regions: RegionPolicy::IndexedAxis { axis },
                ..
            } => {
                let axis = u64::try_from(*axis).map_err(|_| {
                    OperationContractError::InvalidCanonicalEncoding {
                        reason: "indexed-axis rank conversion overflow",
                    }
                })?;
                AddressingRequirement::Positional {
                    minimum_rank: axis.checked_add(1).ok_or(
                        OperationContractError::InvalidCanonicalEncoding {
                            reason: "indexed-axis rank addition overflow",
                        },
                    )?,
                }
            }
            OutputConstruction::ReadModifyWrite {
                regions: RegionPolicy::SingleElement | RegionPolicy::ContiguousRange,
                ..
            } => AddressingRequirement::Positional { minimum_rank: 1 },
            OutputConstruction::ReadModifyWrite {
                regions: RegionPolicy::RectangularRegion,
                ..
            } => AddressingRequirement::Positional { minimum_rank: 2 },
            OutputConstruction::ReadModifyWrite {
                regions: RegionPolicy::CollectionEntry,
                ..
            } => AddressingRequirement::CollectionEntry,
            OutputConstruction::ReadModifyWrite {
                regions: RegionPolicy::Arbitrary,
                ..
            } => AddressingRequirement::ArbitraryRegion,
        };

        Ok(Self {
            access: policy.access,
            delivery: policy.delivery,
            ownership: OwnershipRequirement::from_access(policy.access),
            construction: Some(policy.construction.clone()),
            addressing,
            alias: Some(policy.alias),
            publication: PublicationRequirement::AtomicReplace,
            change_detection: Some(policy.change_detection),
        })
    }
}

fn check_access(
    access: AccessMode,
    storage: &StorageCapabilityDescriptor,
) -> Result<(), PortStorageCompatibilityError> {
    if matches!(
        access,
        AccessMode::Read | AccessMode::ReadWrite | AccessMode::Consume
    ) && !storage.access.readable
    {
        return Err(PortStorageCompatibilityError::ReadUnsupported);
    }
    if matches!(access, AccessMode::Write | AccessMode::ReadWrite) && !storage.access.writable {
        return Err(PortStorageCompatibilityError::WriteUnsupported);
    }
    Ok(())
}

fn check_ownership(
    ownership: OwnershipRequirement,
    storage: &StorageCapabilityDescriptor,
) -> Result<(), PortStorageCompatibilityError> {
    match ownership {
        OwnershipRequirement::SharedRead if !storage.ownership.shared_read => {
            Err(PortStorageCompatibilityError::SharedReadUnsupported)
        }
        OwnershipRequirement::ExclusiveWrite if !storage.ownership.exclusive_write => {
            Err(PortStorageCompatibilityError::ExclusiveWriteUnsupported)
        }
        OwnershipRequirement::OwnedValue
            if !storage.ownership.owned_value && !storage.ownership.detachable =>
        {
            Err(PortStorageCompatibilityError::OwnedValueUnsupported)
        }
        OwnershipRequirement::SharedRead
        | OwnershipRequirement::ExclusiveWrite
        | OwnershipRequirement::OwnedValue => Ok(()),
    }
}

fn require_readable(
    storage: &StorageCapabilityDescriptor,
) -> Result<(), PortStorageCompatibilityError> {
    if storage.access.readable {
        Ok(())
    } else {
        Err(PortStorageCompatibilityError::ReadUnsupported)
    }
}

fn require_writable(
    storage: &StorageCapabilityDescriptor,
) -> Result<(), PortStorageCompatibilityError> {
    if storage.access.writable {
        Ok(())
    } else {
        Err(PortStorageCompatibilityError::WriteUnsupported)
    }
}

fn check_construction(
    construction: Option<&OutputConstruction>,
    storage: &StorageCapabilityDescriptor,
) -> Result<(), PortStorageCompatibilityError> {
    let Some(construction) = construction else {
        return Ok(());
    };
    match construction {
        OutputConstruction::FullWrite { .. }
        | OutputConstruction::Replace { .. }
        | OutputConstruction::Build { .. } => {
            require_writable(storage)?;
            if !storage.access.replaceable {
                return Err(PortStorageCompatibilityError::ReplaceUnsupported);
            }
        }
        OutputConstruction::ReadModifyWrite {
            regions: RegionPolicy::WholeValue,
            ..
        } => {
            require_readable(storage)?;
            require_writable(storage)?;
            if !storage.access.replaceable {
                return Err(PortStorageCompatibilityError::ReplaceUnsupported);
            }
        }
        OutputConstruction::ReadModifyWrite { .. } => {
            require_readable(storage)?;
            require_writable(storage)?;
            if !storage.access.region_mutable {
                return Err(PortStorageCompatibilityError::RegionMutationUnsupported);
            }
        }
    }
    Ok(())
}

fn check_semantic_addressing(
    contract: &ResolvedTypeMemoryContract,
    requirement: AddressingRequirement,
) -> Result<(), PortStorageCompatibilityError> {
    let available = contract.addressing;
    let supported = match requirement {
        AddressingRequirement::WholeValue => true,
        AddressingRequirement::Positional { minimum_rank } => {
            matches!(available.positional_rank, Some(rank) if rank >= minimum_rank)
        }
        AddressingRequirement::CollectionEntry => {
            matches!(available.positional_rank, Some(1..))
                || available.named_members
                || available.keyed_members
        }
        AddressingRequirement::ArbitraryRegion => {
            available.positional_rank.is_some()
                || available.named_members
                || available.keyed_members
        }
    };
    if supported {
        Ok(())
    } else {
        Err(
            PortStorageCompatibilityError::SemanticAddressingUnsupported {
                required: requirement,
                available,
            },
        )
    }
}

fn check_storage_addressing(
    requirement: AddressingRequirement,
    storage: &StorageCapabilityDescriptor,
) -> Result<(), PortStorageCompatibilityError> {
    match requirement {
        AddressingRequirement::WholeValue if !storage.addressing.whole_value => {
            Err(PortStorageCompatibilityError::WholeValueAddressingUnsupported)
        }
        AddressingRequirement::Positional { minimum_rank } => {
            let available = storage.addressing.positional;
            if matches!(available, PositionalAddressingCapability::AnyRank)
                || matches!(available, PositionalAddressingCapability::Rank(rank) if rank >= minimum_rank)
            {
                Ok(())
            } else {
                Err(
                    PortStorageCompatibilityError::PositionalAddressingUnsupported {
                        minimum_rank,
                        available,
                    },
                )
            }
        }
        AddressingRequirement::CollectionEntry => {
            let positional = matches!(
                storage.addressing.positional,
                PositionalAddressingCapability::AnyRank | PositionalAddressingCapability::Rank(1..)
            );
            if positional || storage.addressing.named_members || storage.addressing.keyed_members {
                Ok(())
            } else {
                Err(PortStorageCompatibilityError::CollectionEntryAddressingUnsupported)
            }
        }
        AddressingRequirement::ArbitraryRegion if !storage.addressing.arbitrary_regions => {
            Err(PortStorageCompatibilityError::ArbitraryRegionUnsupported)
        }
        AddressingRequirement::WholeValue | AddressingRequirement::ArbitraryRegion => Ok(()),
    }
}

fn check_publication(
    requirement: PublicationRequirement,
    storage: &StorageCapabilityDescriptor,
) -> Result<(), PortStorageCompatibilityError> {
    if requirement == PublicationRequirement::None {
        return Ok(());
    }
    if !storage.publication.atomic_replace {
        return Err(PortStorageCompatibilityError::AtomicPublicationUnsupported);
    }
    if !storage.publication.preserves_previous_on_failure {
        return Err(PortStorageCompatibilityError::FailureAtomicityUnsupported);
    }
    Ok(())
}

fn check_change_detection(
    contract: &ResolvedTypeMemoryContract,
    policy: Option<ChangeDetectionPolicy>,
    storage: &StorageCapabilityDescriptor,
) -> Result<(), PortStorageCompatibilityError> {
    match policy {
        None
        | Some(ChangeDetectionPolicy::KernelReported | ChangeDetectionPolicy::AlwaysChanged) => {
            Ok(())
        }
        Some(ChangeDetectionPolicy::SemanticHash) => {
            require_readable(storage)?;
            if storage.access.canonical_snapshot {
                Ok(())
            } else {
                Err(PortStorageCompatibilityError::CanonicalSnapshotUnsupported)
            }
        }
        Some(ChangeDetectionPolicy::ExactScalar) => {
            if !matches!(contract.topology, MemoryTopology::Scalar(_)) {
                return Err(
                    PortStorageCompatibilityError::ExactScalarChangeDetectionRequiresScalar,
                );
            }
            require_readable(storage)?;
            if storage.access.canonical_snapshot {
                Ok(())
            } else {
                Err(PortStorageCompatibilityError::CanonicalSnapshotUnsupported)
            }
        }
    }
}

pub fn check_port_storage_compatibility(
    schema: &Schema,
    shape: &ShapeInstance,
    requirement: &PortMemoryRequirement,
    storage: &StorageCapabilityDescriptor,
) -> Result<(), PortStorageCompatibilityError> {
    let contract = schema
        .resolved_type_memory_contract(shape)
        .map_err(|error| {
            PortStorageCompatibilityError::SchemaStorage(SchemaStorageCompatibilityError::Semantic(
                error,
            ))
        })?;
    crate::check_resolved_type_storage_compatibility(schema.body(), &contract, storage).map_err(
        |error| {
            PortStorageCompatibilityError::SchemaStorage(SchemaStorageCompatibilityError::Storage(
                error,
            ))
        },
    )?;
    check_semantic_addressing(&contract, requirement.addressing)?;
    check_access(requirement.access, storage)?;
    check_ownership(requirement.ownership, storage)?;
    check_construction(requirement.construction.as_ref(), storage)?;
    check_storage_addressing(requirement.addressing, storage)?;
    check_publication(requirement.publication, storage)?;
    check_change_detection(&contract, requirement.change_detection, storage)
}

impl OperationContractDeclaration {
    pub fn memory_requirements(
        &self,
        input_count: usize,
    ) -> Result<OperationMemoryRequirements, OperationContractError> {
        let inputs = self
            .inputs
            .resolve(input_count)?
            .iter()
            .copied()
            .map(PortMemoryRequirement::for_input)
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let outputs = self
            .outputs
            .iter()
            .map(PortMemoryRequirement::for_output)
            .collect::<Result<Vec<_>, _>>()?
            .into_boxed_slice();
        Ok(OperationMemoryRequirements { inputs, outputs })
    }
}
