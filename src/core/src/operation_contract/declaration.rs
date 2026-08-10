#[cfg(feature = "no_std")]
use alloc::{boxed::Box, string::String};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, string::String};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct OperationContractDeclaration {
    pub inputs: InputPortLayout,
    pub outputs: Box<[OutputPortPolicy]>,
    pub interaction: ExternalInteraction,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum InputPortLayout {
    Fixed(Box<[InputPortPolicy]>),
    Variadic {
        prefix: Box<[InputPortPolicy]>,
        repeated: InputPortPolicy,
        min_repetitions: u32,
    },
}

impl InputPortLayout {
    pub fn resolve(
        &self,
        input_count: usize,
    ) -> Result<Box<[InputPortPolicy]>, OperationContractError> {
        match self {
            Self::Fixed(inputs) => {
                if inputs.len() != input_count {
                    return Err(OperationContractError::PortCountMismatch {
                        direction: PortDirection::Input,
                        expected: inputs.len() as u64,
                        actual: input_count as u64,
                    });
                }
                Ok(inputs.clone())
            }
            Self::Variadic {
                prefix,
                repeated,
                min_repetitions,
            } => {
                let repetitions = input_count.checked_sub(prefix.len()).ok_or(
                    OperationContractError::VariadicInputCount {
                        prefix: prefix.len() as u64,
                        minimum_repetitions: *min_repetitions,
                        actual: input_count as u64,
                    },
                )?;
                if repetitions < *min_repetitions as usize {
                    return Err(OperationContractError::VariadicInputCount {
                        prefix: prefix.len() as u64,
                        minimum_repetitions: *min_repetitions,
                        actual: input_count as u64,
                    });
                }
                let mut resolved = prefix.to_vec();
                resolved.resize(input_count, *repeated);
                Ok(resolved.into_boxed_slice())
            }
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct InputPortPolicy {
    pub access: AccessMode,
    pub delivery: DeliveryMode,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct OutputPortPolicy {
    pub access: AccessMode,
    pub delivery: DeliveryMode,
    pub construction: OutputConstruction,
    pub alias: AliasPolicy,
    pub change_detection: ChangeDetectionPolicy,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AccessMode {
    Read,
    Write,
    ReadWrite,
    Consume,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DeliveryMode {
    Signal,
    Stream,
    Future,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum OutputConstruction {
    FullWrite {
        shape: ShapeRule,
    },
    ReadModifyWrite {
        base_input: u16,
        regions: RegionPolicy,
    },
    Replace {
        shape: ShapeRule,
    },
    Build {
        postcondition: ShapeContractReference,
    },
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ShapeRule {
    Declared,
    SameAsInput { input: u16 },
    TransposeOf { input: u16 },
    MatrixProduct { lhs: u16, rhs: u16 },
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ShapeContractReference {
    pub module_path: Box<[String]>,
    pub contract_name: String,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RegionPolicy {
    SingleElement,
    ContiguousRange,
    RectangularRegion,
    CollectionEntry,
    Arbitrary,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AliasPolicy {
    NoAlias,
    MayAlias { input: u16 },
    InPlaceRequired { input: u16 },
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ChangeDetectionPolicy {
    KernelReported,
    ExactScalar,
    SemanticHash,
    AlwaysChanged,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ExternalInteraction {
    Pure,
    Observation(ObservationContract),
    Effect(EffectContract),
    TransactionalExternal(TransactionalExternalContract),
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ObservationContract {
    pub replay: ObservationReplayPolicy,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ObservationReplayPolicy {
    CaptureAsInputFact,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EffectContract {
    pub delivery: EffectDeliveryPolicy,
    pub idempotency: IdempotencyRequirement,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EffectDeliveryPolicy {
    ProviderDefined,
    AtMostOnce,
    AtLeastOnce,
    IdempotentRetry,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum IdempotencyRequirement {
    NotRequired,
    Optional,
    Required,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TransactionalExternalContract {
    pub protocol: TransactionalEffectProtocol,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum TransactionalEffectProtocol {
    PrepareCommit,
    PrepareCommitCompensate,
}

use super::{OperationContractError, PortDirection};
