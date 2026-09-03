//! Memory-facing requirements derived from declared operation ports.

use crate::{
    AccessMode, AliasPolicy, ChangeDetectionPolicy, DeliveryMode, InputPortPolicy,
    OperationContractDeclaration, OperationContractError, OutputConstruction, OutputPortPolicy,
    RegionPolicy,
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
