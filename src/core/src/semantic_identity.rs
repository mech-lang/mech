//! Final semantic, program, schema, and resident identities.

use crate::{SemanticIdentityKind, SemanticModelError};

macro_rules! opaque_u32_id {
    ($name:ident) => {
        #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        #[repr(transparent)]
        pub struct $name(u32);

        impl $name {
            pub const fn new(raw: u32) -> Self {
                Self(raw)
            }

            pub const fn get(self) -> u32 {
                self.0
            }
        }
    };
}

macro_rules! opaque_hash_id {
    ($name:ident) => {
        #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        #[repr(transparent)]
        pub struct $name([u8; 32]);

        impl $name {
            pub const fn from_bytes(bytes: [u8; 32]) -> Self {
                Self(bytes)
            }

            pub const fn as_bytes(&self) -> &[u8; 32] {
                &self.0
            }

            pub const fn into_bytes(self) -> [u8; 32] {
                self.0
            }
        }
    };
}

opaque_hash_id!(ProgramRevision);
opaque_hash_id!(SchemaKey);
opaque_hash_id!(NominalKey);
opaque_hash_id!(ValueHash);
opaque_hash_id!(KeyHash);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct ConstantId(u32);

impl ConstantId {
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    pub const fn get(self) -> u32 {
        self.0
    }
}

opaque_u32_id!(KindId);
opaque_u32_id!(KindParameterId);
opaque_u32_id!(DimensionParameterId);
opaque_u32_id!(SchemaId);

macro_rules! artifact_id {
    ($name:ident) => {
        #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
        #[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
        #[repr(transparent)]
        pub struct $name(pub u32);

        impl $name {
            pub const fn new(raw: u32) -> Self {
                Self(raw)
            }

            pub const fn get(self) -> u32 {
                self.0
            }
        }
    };
}

artifact_id!(NodeId);
artifact_id!(BindingId);
artifact_id!(InputId);
artifact_id!(OutputId);
artifact_id!(IntegrityConstraintId);
artifact_id!(OperationContractId);
artifact_id!(ApplicationRequirementId);

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct CellSlotId(pub u32);

impl CellSlotId {
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    pub const fn get(self) -> u32 {
        self.0
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct SlotIndex(pub u32);

impl SlotIndex {
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    pub const fn get(self) -> u32 {
        self.0
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ReactiveInstanceId {
    index: u32,
    generation: u32,
}

impl ReactiveInstanceId {
    pub const fn new(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }

    pub const fn index(self) -> u32 {
        self.index
    }

    pub const fn generation(self) -> u32 {
        self.generation
    }

    pub fn checked_next_generation(self) -> Result<Self, SemanticModelError> {
        Ok(Self {
            index: self.index,
            generation: self.generation.checked_add(1).ok_or(
                SemanticModelError::IdentityExhausted {
                    identity: SemanticIdentityKind::ReactiveInstanceGeneration,
                },
            )?,
        })
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CellId {
    instance: ReactiveInstanceId,
    slot: CellSlotId,
}

impl CellId {
    pub const fn new(instance: ReactiveInstanceId, slot: CellSlotId) -> Self {
        Self { instance, slot }
    }

    pub const fn instance(self) -> ReactiveInstanceId {
        self.instance
    }

    pub const fn slot(self) -> CellSlotId {
        self.slot
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct InstanceEpoch(pub u64);

impl InstanceEpoch {
    pub const ZERO: Self = Self(0);

    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn get(self) -> u64 {
        self.0
    }

    #[inline]
    pub fn checked_next(self) -> Result<Self, SemanticModelError> {
        self.0
            .checked_add(1)
            .map(Self)
            .ok_or(SemanticModelError::IdentityExhausted {
                identity: SemanticIdentityKind::InstanceEpoch,
            })
    }
}

macro_rules! opaque_generation {
    ($name:ident, $kind:ident) => {
        #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        #[repr(transparent)]
        pub struct $name(u64);

        impl $name {
            pub const ZERO: Self = Self(0);

            pub const fn new(raw: u64) -> Self {
                Self(raw)
            }

            pub const fn get(self) -> u64 {
                self.0
            }

            pub fn checked_next(self) -> Result<Self, SemanticModelError> {
                self.0
                    .checked_add(1)
                    .map(Self)
                    .ok_or(SemanticModelError::IdentityExhausted {
                        identity: SemanticIdentityKind::$kind,
                    })
            }
        }
    };
}

opaque_generation!(PlanGeneration, PlanGeneration);
opaque_generation!(LayoutGeneration, LayoutGeneration);
