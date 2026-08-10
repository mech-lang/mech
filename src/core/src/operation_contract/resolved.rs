use crate::{OperationContractId, SchemaId};

use super::{
    AccessMode, AliasPolicy, ChangeDetectionPolicy, DeliveryMode, ExternalInteraction,
    OutputConstruction,
};

#[cfg(feature = "no_std")]
use alloc::{boxed::Box, vec, vec::Vec};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, vec, vec::Vec};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ResolvedOperationContract {
    Declared(DeclaredOperationContract),
    LegacyOpaque(LegacyOpaqueOperationContract),
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DeclaredOperationContract {
    pub inputs: Box<[ResolvedInputPort]>,
    pub outputs: Box<[ResolvedOutputPort]>,
    pub interaction: ExternalInteraction,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ResolvedInputPort {
    pub schema: SchemaId,
    pub access: AccessMode,
    pub delivery: DeliveryMode,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ResolvedOutputPort {
    pub schema: SchemaId,
    pub access: AccessMode,
    pub delivery: DeliveryMode,
    pub construction: OutputConstruction,
    pub alias: AliasPolicy,
    pub change_detection: ChangeDetectionPolicy,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LegacyOpaqueOperationContract {
    pub input_schemas: Box<[SchemaId]>,
    pub output_schemas: Box<[SchemaId]>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct OperationContractHandle {
    ordinal: u32,
}

#[derive(Clone, Debug, Default)]
pub struct OperationContractTableBuilder {
    pending: Vec<ResolvedOperationContract>,
}

#[derive(Clone, Debug)]
pub struct OperationContractTableBuild {
    pub table: OperationContractTable,
    remap: Box<[OperationContractId]>,
}

#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationContractTable {
    entries: Box<[ResolvedOperationContract]>,
}

impl OperationContractTableBuilder {
    pub const fn new() -> Self {
        Self {
            pending: Vec::new(),
        }
    }

    pub fn insert(
        &mut self,
        contract: ResolvedOperationContract,
    ) -> Result<OperationContractHandle, super::OperationContractError> {
        super::validate_resolved_contract(&contract)?;
        let ordinal = u32::try_from(self.pending.len()).map_err(|_| {
            super::OperationContractError::IdentityExhausted {
                identity: "OperationContractHandle",
            }
        })?;
        self.pending.push(contract);
        Ok(OperationContractHandle { ordinal })
    }

    pub fn finish(self) -> Result<OperationContractTableBuild, super::OperationContractError> {
        self.finish_with_limit(u32::MAX as usize)
    }

    fn finish_with_limit(
        self,
        unique_limit: usize,
    ) -> Result<OperationContractTableBuild, super::OperationContractError> {
        let mut pending = self
            .pending
            .into_iter()
            .enumerate()
            .map(|(handle, contract)| {
                let canonical_bytes = contract.canonical_bytes()?;
                Ok((canonical_bytes, handle, contract))
            })
            .collect::<Result<Vec<_>, _>>()?;
        pending.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));

        let mut remap = vec![OperationContractId::new(0); pending.len()];
        let mut entries: Vec<ResolvedOperationContract> = Vec::new();
        let mut prior_bytes: Option<Box<[u8]>> = None;
        for (canonical_bytes, handle, contract) in pending {
            if prior_bytes.as_deref() == Some(canonical_bytes.as_ref()) {
                remap[handle] = OperationContractId::new((entries.len() - 1) as u32);
                continue;
            }
            if entries.len() >= unique_limit {
                return Err(super::OperationContractError::IdentityExhausted {
                    identity: "OperationContractId",
                });
            }
            let id = u32::try_from(entries.len())
                .map(OperationContractId::new)
                .map_err(|_| super::OperationContractError::IdentityExhausted {
                    identity: "OperationContractId",
                })?;
            remap[handle] = id;
            prior_bytes = Some(canonical_bytes);
            entries.push(contract);
        }
        Ok(OperationContractTableBuild {
            table: OperationContractTable {
                entries: entries.into_boxed_slice(),
            },
            remap: remap.into_boxed_slice(),
        })
    }
}

impl OperationContractTableBuild {
    pub fn resolve(
        &self,
        handle: OperationContractHandle,
    ) -> Result<OperationContractId, super::OperationContractError> {
        self.remap
            .get(handle.ordinal as usize)
            .copied()
            .ok_or(super::OperationContractError::InvalidContractHandle)
    }

    pub fn into_parts(self) -> (OperationContractTable, Box<[OperationContractId]>) {
        (self.table, self.remap)
    }
}

impl OperationContractTable {
    pub fn empty() -> Self {
        Self {
            entries: Box::new([]),
        }
    }

    pub(super) const fn from_entries_unchecked(entries: Box<[ResolvedOperationContract]>) -> Self {
        Self { entries }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn get(&self, id: OperationContractId) -> Option<&ResolvedOperationContract> {
        self.entries.get(id.get() as usize)
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = &ResolvedOperationContract> {
        self.entries.iter()
    }

    pub fn validate_canonical_order(&self) -> Result<(), super::OperationContractError> {
        let mut prior: Option<Box<[u8]>> = None;
        for contract in &self.entries {
            super::validate_resolved_contract(contract)?;
            let bytes = contract.canonical_bytes()?;
            if prior
                .as_ref()
                .is_some_and(|prior| prior.as_ref() >= bytes.as_ref())
            {
                return Err(super::OperationContractError::NonCanonicalContractOrder);
            }
            prior = Some(bytes);
        }
        Ok(())
    }
}
