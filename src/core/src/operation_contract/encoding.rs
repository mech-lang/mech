use crate::SchemaId;

use super::{
    AccessMode, AliasPolicy, ChangeDetectionPolicy, DeclaredOperationContract, DeliveryMode,
    EffectContract, EffectDeliveryPolicy, ExternalInteraction, IdempotencyRequirement,
    ObservationContract, ObservationReplayPolicy, OperationContractError, OperationContractTable,
    OutputConstruction, RegionPolicy, ResolvedInputPort, ResolvedOperationContract,
    ResolvedOutputPort, ShapeContractReference, ShapeRule, TransactionalEffectProtocol,
    TransactionalExternalContract,
};

#[cfg(feature = "no_std")]
use alloc::{boxed::Box, string::String, vec::Vec};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, string::String, vec::Vec};

const CONTRACT_ENCODING_VERSION: u8 = 1;

impl ResolvedOperationContract {
    pub fn canonical_bytes(&self) -> Result<Box<[u8]>, OperationContractError> {
        super::validate_resolved_contract(self)?;
        let mut bytes = Vec::new();
        bytes.push(CONTRACT_ENCODING_VERSION);
        match self {
            Self::Declared(contract) => {
                bytes.push(0);
                encode_declared(contract, &mut bytes)?;
            }
        }
        Ok(bytes.into_boxed_slice())
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, OperationContractError> {
        let mut reader = Reader::new(bytes);
        if reader.u8()? != CONTRACT_ENCODING_VERSION {
            return Err(OperationContractError::InvalidCanonicalEncoding {
                reason: "unknown operation-contract encoding version",
            });
        }
        let contract = match reader.u8()? {
            0 => Self::Declared(decode_declared(&mut reader)?),
            _ => {
                return Err(OperationContractError::InvalidCanonicalEncoding {
                    reason: "unknown operation-contract tag",
                });
            }
        };
        reader.finish()?;
        super::validate_resolved_contract(&contract)?;
        if contract.canonical_bytes()?.as_ref() != bytes {
            return Err(OperationContractError::NonCanonicalContractBytes);
        }
        Ok(contract)
    }
}

impl OperationContractTable {
    pub fn canonical_bytes(&self) -> Result<Box<[u8]>, OperationContractError> {
        self.validate_canonical_order()?;
        let mut bytes = Vec::new();
        write_len(self.len(), &mut bytes)?;
        for contract in self.iter() {
            let contract = contract.canonical_bytes()?;
            write_len(contract.len(), &mut bytes)?;
            bytes.extend_from_slice(&contract);
        }
        Ok(bytes.into_boxed_slice())
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, OperationContractError> {
        let mut reader = Reader::new(bytes);
        let count = reader.len()?;
        // The byte length bounds the number of complete length-prefixed
        // entries. Do not let an untrusted count request an allocation before
        // the reader has proved those entries exist.
        let mut entries = Vec::with_capacity(count.min(bytes.len() / 4));
        for _ in 0..count {
            let len = reader.len()?;
            entries.push(ResolvedOperationContract::from_canonical_bytes(
                reader.take(len)?,
            )?);
        }
        reader.finish()?;
        let table = Self::from_entries_unchecked(entries.into_boxed_slice());
        table.validate_canonical_order()?;
        if table.canonical_bytes()?.as_ref() != bytes {
            return Err(OperationContractError::NonCanonicalContractBytes);
        }
        Ok(table)
    }
}

fn encode_declared(
    contract: &DeclaredOperationContract,
    bytes: &mut Vec<u8>,
) -> Result<(), OperationContractError> {
    write_len(contract.inputs.len(), bytes)?;
    for input in &contract.inputs {
        bytes.extend_from_slice(&input.schema.get().to_le_bytes());
        bytes.push(access_tag(input.access));
        bytes.push(delivery_tag(input.delivery));
    }
    write_len(contract.outputs.len(), bytes)?;
    for output in &contract.outputs {
        bytes.extend_from_slice(&output.schema.get().to_le_bytes());
        bytes.push(access_tag(output.access));
        bytes.push(delivery_tag(output.delivery));
        encode_construction(&output.construction, bytes)?;
        encode_alias(output.alias, bytes);
        bytes.push(change_detection_tag(output.change_detection));
    }
    encode_interaction(&contract.interaction, bytes);
    Ok(())
}

fn decode_declared(
    reader: &mut Reader<'_>,
) -> Result<DeclaredOperationContract, OperationContractError> {
    let input_count = reader.bounded_len(
        6,
        "operation-contract input count exceeds the remaining bytes",
    )?;
    let mut inputs = Vec::with_capacity(input_count);
    for _ in 0..input_count {
        inputs.push(ResolvedInputPort {
            schema: SchemaId::new(reader.u32()?),
            access: decode_access(reader.u8()?)?,
            delivery: decode_delivery(reader.u8()?)?,
        });
    }
    let output_count = reader.bounded_len(
        10,
        "operation-contract output count exceeds the remaining bytes",
    )?;
    let mut outputs = Vec::with_capacity(output_count);
    for _ in 0..output_count {
        outputs.push(ResolvedOutputPort {
            schema: SchemaId::new(reader.u32()?),
            access: decode_access(reader.u8()?)?,
            delivery: decode_delivery(reader.u8()?)?,
            construction: decode_construction(reader)?,
            alias: decode_alias(reader)?,
            change_detection: decode_change_detection(reader.u8()?)?,
        });
    }
    Ok(DeclaredOperationContract {
        inputs: inputs.into_boxed_slice(),
        outputs: outputs.into_boxed_slice(),
        interaction: decode_interaction(reader)?,
    })
}

fn encode_construction(
    construction: &OutputConstruction,
    bytes: &mut Vec<u8>,
) -> Result<(), OperationContractError> {
    match construction {
        OutputConstruction::FullWrite { shape } => {
            bytes.push(0);
            encode_shape(*shape, bytes);
        }
        OutputConstruction::ReadModifyWrite {
            base_input,
            regions,
        } => {
            bytes.push(1);
            bytes.extend_from_slice(&base_input.to_le_bytes());
            encode_region(*regions, bytes);
        }
        OutputConstruction::Replace { shape } => {
            bytes.push(2);
            encode_shape(*shape, bytes);
        }
        OutputConstruction::Build { postcondition } => {
            bytes.push(3);
            write_len(postcondition.module_path.len(), bytes)?;
            for segment in &postcondition.module_path {
                write_string(segment, bytes)?;
            }
            write_string(&postcondition.contract_name, bytes)?;
        }
    }
    Ok(())
}

fn decode_construction(
    reader: &mut Reader<'_>,
) -> Result<OutputConstruction, OperationContractError> {
    Ok(match reader.u8()? {
        0 => OutputConstruction::FullWrite {
            shape: decode_shape(reader)?,
        },
        1 => OutputConstruction::ReadModifyWrite {
            base_input: reader.u16()?,
            regions: decode_region(reader.u8()?, reader)?,
        },
        2 => OutputConstruction::Replace {
            shape: decode_shape(reader)?,
        },
        3 => {
            let count = reader.bounded_len(
                4,
                "shape-contract module-path count exceeds the remaining bytes",
            )?;
            let mut module_path = Vec::with_capacity(count);
            for _ in 0..count {
                module_path.push(reader.string()?);
            }
            OutputConstruction::Build {
                postcondition: ShapeContractReference {
                    module_path: module_path.into_boxed_slice(),
                    contract_name: reader.string()?,
                },
            }
        }
        _ => return invalid("unknown output-construction tag"),
    })
}

fn encode_shape(shape: ShapeRule, bytes: &mut Vec<u8>) {
    match shape {
        ShapeRule::Declared => bytes.push(0),
        ShapeRule::SameAsInput { input } => {
            bytes.push(1);
            bytes.extend_from_slice(&input.to_le_bytes());
        }
        ShapeRule::TransposeOf { input } => {
            bytes.push(2);
            bytes.extend_from_slice(&input.to_le_bytes());
        }
        ShapeRule::MatrixProduct { lhs, rhs } => {
            bytes.push(3);
            bytes.extend_from_slice(&lhs.to_le_bytes());
            bytes.extend_from_slice(&rhs.to_le_bytes());
        }
    }
}

fn decode_shape(reader: &mut Reader<'_>) -> Result<ShapeRule, OperationContractError> {
    Ok(match reader.u8()? {
        0 => ShapeRule::Declared,
        1 => ShapeRule::SameAsInput {
            input: reader.u16()?,
        },
        2 => ShapeRule::TransposeOf {
            input: reader.u16()?,
        },
        3 => ShapeRule::MatrixProduct {
            lhs: reader.u16()?,
            rhs: reader.u16()?,
        },
        _ => return invalid("unknown shape-rule tag"),
    })
}

fn encode_alias(alias: AliasPolicy, bytes: &mut Vec<u8>) {
    match alias {
        AliasPolicy::NoAlias => bytes.push(0),
        AliasPolicy::MayAlias { input } => {
            bytes.push(1);
            bytes.extend_from_slice(&input.to_le_bytes());
        }
        AliasPolicy::InPlaceRequired { input } => {
            bytes.push(2);
            bytes.extend_from_slice(&input.to_le_bytes());
        }
    }
}

fn decode_alias(reader: &mut Reader<'_>) -> Result<AliasPolicy, OperationContractError> {
    Ok(match reader.u8()? {
        0 => AliasPolicy::NoAlias,
        1 => AliasPolicy::MayAlias {
            input: reader.u16()?,
        },
        2 => AliasPolicy::InPlaceRequired {
            input: reader.u16()?,
        },
        _ => return invalid("unknown alias-policy tag"),
    })
}

fn encode_interaction(interaction: &ExternalInteraction, bytes: &mut Vec<u8>) {
    match interaction {
        ExternalInteraction::Pure => bytes.push(0),
        ExternalInteraction::Observation(contract) => {
            bytes.push(1);
            bytes.push(match contract.replay {
                ObservationReplayPolicy::CaptureAsInputFact => 0,
            });
        }
        ExternalInteraction::Effect(contract) => {
            bytes.push(2);
            bytes.push(effect_delivery_tag(contract.delivery));
            bytes.push(idempotency_tag(contract.idempotency));
        }
        ExternalInteraction::TransactionalExternal(contract) => {
            bytes.push(3);
            bytes.push(match contract.protocol {
                TransactionalEffectProtocol::PrepareCommit => 0,
                TransactionalEffectProtocol::PrepareCommitCompensate => 1,
            });
        }
    }
}

fn decode_interaction(
    reader: &mut Reader<'_>,
) -> Result<ExternalInteraction, OperationContractError> {
    Ok(match reader.u8()? {
        0 => ExternalInteraction::Pure,
        1 => ExternalInteraction::Observation(ObservationContract {
            replay: match reader.u8()? {
                0 => ObservationReplayPolicy::CaptureAsInputFact,
                _ => return invalid("unknown observation replay tag"),
            },
        }),
        2 => ExternalInteraction::Effect(EffectContract {
            delivery: decode_effect_delivery(reader.u8()?)?,
            idempotency: decode_idempotency(reader.u8()?)?,
        }),
        3 => ExternalInteraction::TransactionalExternal(TransactionalExternalContract {
            protocol: match reader.u8()? {
                0 => TransactionalEffectProtocol::PrepareCommit,
                1 => TransactionalEffectProtocol::PrepareCommitCompensate,
                _ => return invalid("unknown transactional protocol tag"),
            },
        }),
        _ => return invalid("unknown external-interaction tag"),
    })
}

fn access_tag(access: AccessMode) -> u8 {
    match access {
        AccessMode::Read => 0,
        AccessMode::Write => 1,
        AccessMode::ReadWrite => 2,
        AccessMode::Consume => 3,
    }
}

fn decode_access(tag: u8) -> Result<AccessMode, OperationContractError> {
    Ok(match tag {
        0 => AccessMode::Read,
        1 => AccessMode::Write,
        2 => AccessMode::ReadWrite,
        3 => AccessMode::Consume,
        _ => return invalid("unknown access-mode tag"),
    })
}

fn delivery_tag(delivery: DeliveryMode) -> u8 {
    match delivery {
        DeliveryMode::Signal => 0,
        DeliveryMode::Stream => 1,
        DeliveryMode::Future => 2,
    }
}

fn decode_delivery(tag: u8) -> Result<DeliveryMode, OperationContractError> {
    Ok(match tag {
        0 => DeliveryMode::Signal,
        1 => DeliveryMode::Stream,
        2 => DeliveryMode::Future,
        _ => return invalid("unknown delivery-mode tag"),
    })
}

fn encode_region(region: RegionPolicy, bytes: &mut Vec<u8>) {
    bytes.push(region_tag(region));
    if let RegionPolicy::IndexedAxis { axis } = region {
        bytes.extend_from_slice(&axis.to_le_bytes());
    }
}

fn region_tag(region: RegionPolicy) -> u8 {
    match region {
        RegionPolicy::SingleElement => 0,
        RegionPolicy::ContiguousRange => 1,
        RegionPolicy::RectangularRegion => 2,
        RegionPolicy::CollectionEntry => 3,
        RegionPolicy::Arbitrary => 4,
        RegionPolicy::WholeValue => 5,
        RegionPolicy::IndexedAxis { .. } => 6,
    }
}

fn decode_region(tag: u8, reader: &mut Reader<'_>) -> Result<RegionPolicy, OperationContractError> {
    Ok(match tag {
        0 => RegionPolicy::SingleElement,
        1 => RegionPolicy::ContiguousRange,
        2 => RegionPolicy::RectangularRegion,
        3 => RegionPolicy::CollectionEntry,
        4 => RegionPolicy::Arbitrary,
        5 => RegionPolicy::WholeValue,
        6 => RegionPolicy::IndexedAxis {
            axis: reader.u16()?,
        },
        _ => return invalid("unknown region-policy tag"),
    })
}

fn change_detection_tag(policy: ChangeDetectionPolicy) -> u8 {
    match policy {
        ChangeDetectionPolicy::KernelReported => 0,
        ChangeDetectionPolicy::ExactScalar => 1,
        ChangeDetectionPolicy::SemanticHash => 2,
        ChangeDetectionPolicy::AlwaysChanged => 3,
    }
}

fn decode_change_detection(tag: u8) -> Result<ChangeDetectionPolicy, OperationContractError> {
    Ok(match tag {
        0 => ChangeDetectionPolicy::KernelReported,
        1 => ChangeDetectionPolicy::ExactScalar,
        2 => ChangeDetectionPolicy::SemanticHash,
        3 => ChangeDetectionPolicy::AlwaysChanged,
        _ => return invalid("unknown change-detection tag"),
    })
}

fn effect_delivery_tag(policy: EffectDeliveryPolicy) -> u8 {
    match policy {
        EffectDeliveryPolicy::ProviderDefined => 0,
        EffectDeliveryPolicy::AtMostOnce => 1,
        EffectDeliveryPolicy::AtLeastOnce => 2,
        EffectDeliveryPolicy::IdempotentRetry => 3,
    }
}

fn decode_effect_delivery(tag: u8) -> Result<EffectDeliveryPolicy, OperationContractError> {
    Ok(match tag {
        0 => EffectDeliveryPolicy::ProviderDefined,
        1 => EffectDeliveryPolicy::AtMostOnce,
        2 => EffectDeliveryPolicy::AtLeastOnce,
        3 => EffectDeliveryPolicy::IdempotentRetry,
        _ => return invalid("unknown effect-delivery tag"),
    })
}

fn idempotency_tag(requirement: IdempotencyRequirement) -> u8 {
    match requirement {
        IdempotencyRequirement::NotRequired => 0,
        IdempotencyRequirement::Optional => 1,
        IdempotencyRequirement::Required => 2,
    }
}

fn decode_idempotency(tag: u8) -> Result<IdempotencyRequirement, OperationContractError> {
    Ok(match tag {
        0 => IdempotencyRequirement::NotRequired,
        1 => IdempotencyRequirement::Optional,
        2 => IdempotencyRequirement::Required,
        _ => return invalid("unknown idempotency tag"),
    })
}

fn write_len(len: usize, bytes: &mut Vec<u8>) -> Result<(), OperationContractError> {
    let len = u32::try_from(len).map_err(|_| OperationContractError::IdentityExhausted {
        identity: "operation contract encoded length",
    })?;
    bytes.extend_from_slice(&len.to_le_bytes());
    Ok(())
}

fn write_string(value: &str, bytes: &mut Vec<u8>) -> Result<(), OperationContractError> {
    write_len(value.len(), bytes)?;
    bytes.extend_from_slice(value.as_bytes());
    Ok(())
}

fn invalid<T>(reason: &'static str) -> Result<T, OperationContractError> {
    Err(OperationContractError::InvalidCanonicalEncoding { reason })
}

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8], OperationContractError> {
        let end = self.offset.checked_add(len).ok_or(
            OperationContractError::InvalidCanonicalEncoding {
                reason: "operation contract length overflow",
            },
        )?;
        let value = self.bytes.get(self.offset..end).ok_or(
            OperationContractError::InvalidCanonicalEncoding {
                reason: "truncated operation contract",
            },
        )?;
        self.offset = end;
        Ok(value)
    }

    fn u8(&mut self) -> Result<u8, OperationContractError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, OperationContractError> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }

    fn u32(&mut self) -> Result<u32, OperationContractError> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    fn len(&mut self) -> Result<usize, OperationContractError> {
        usize::try_from(self.u32()?).map_err(|_| OperationContractError::InvalidCanonicalEncoding {
            reason: "operation contract length is not representable",
        })
    }

    fn bounded_len(
        &mut self,
        minimum_item_bytes: usize,
        reason: &'static str,
    ) -> Result<usize, OperationContractError> {
        let count = self.len()?;
        let remaining = self.bytes.len() - self.offset;
        if count > remaining / minimum_item_bytes {
            return Err(OperationContractError::InvalidCanonicalEncoding { reason });
        }
        Ok(count)
    }

    fn string(&mut self) -> Result<String, OperationContractError> {
        let len = self.len()?;
        let value = core::str::from_utf8(self.take(len)?).map_err(|_| {
            OperationContractError::InvalidCanonicalEncoding {
                reason: "operation contract string is not UTF-8",
            }
        })?;
        Ok(value.into())
    }

    fn finish(self) -> Result<(), OperationContractError> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            invalid("trailing operation-contract bytes")
        }
    }
}
