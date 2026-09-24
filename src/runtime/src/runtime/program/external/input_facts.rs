use std::collections::BTreeSet;

use mech_core::{
    ApplicationRequirementId, CellSlotId, MResult, MechError, MechErrorKind, NodeId, SchemaKey,
    SchemaTable, ShapeInstance, Value, ValueHash,
};

use crate::turn_record::{AccountedRecord, InputSequence, InputSequenceRange, sealed::Sealed};

#[derive(Clone, Debug)]
pub struct CapturedInputFact {
    pub sequence: InputSequence,
    pub requirement: ApplicationRequirementId,
    pub node: NodeId,
    pub slot: CellSlotId,
    pub schema_key: SchemaKey,
    pub shape: ShapeInstance,
    pub value: Value,
    /// Whether this observation scheduled activation control in the admitted
    /// turn. Sample-only facts remain part of the retained input snapshot.
    pub trigger: bool,
    pub payload_hash: ValueHash,
    retained_bytes: usize,
}

impl CapturedInputFact {
    pub fn new(
        sequence: InputSequence,
        requirement: ApplicationRequirementId,
        node: NodeId,
        slot: CellSlotId,
        schema_key: SchemaKey,
        shape: ShapeInstance,
        value: Value,
        schemas: &SchemaTable,
    ) -> MResult<Self> {
        Self::new_with_trigger(
            sequence,
            requirement,
            node,
            slot,
            schema_key,
            shape,
            value,
            true,
            schemas,
        )
    }

    pub fn new_with_trigger(
        sequence: InputSequence,
        requirement: ApplicationRequirementId,
        node: NodeId,
        slot: CellSlotId,
        schema_key: SchemaKey,
        shape: ShapeInstance,
        value: Value,
        trigger: bool,
        schemas: &SchemaTable,
    ) -> MResult<Self> {
        let schema = schemas
            .entry(value.schema())
            .ok_or_else(|| invalid_error("value schema"))?;
        if schema.key() != schema_key || value.schema_key() != schema_key || value.shape() != &shape
        {
            return Err(invalid_error("schema key or shape"));
        }
        let payload_hash = value
            .value_hash(schemas)
            .map_err(|_| invalid_error("payload hash"))?;
        let retained_bytes = retained_input_value_bytes(&value, &shape, schemas)?;
        Ok(Self {
            sequence,
            requirement,
            node,
            slot,
            schema_key,
            shape,
            value,
            trigger,
            payload_hash,
            retained_bytes,
        })
    }

    pub const fn retained_bytes(&self) -> usize {
        self.retained_bytes
    }
}

pub(crate) fn retained_input_value_bytes(
    value: &Value,
    shape: &ShapeInstance,
    schemas: &SchemaTable,
) -> MResult<usize> {
    value
        .canonical_payload_bytes(schemas)
        .map_err(|_| invalid_error("canonical payload"))?
        .len()
        .checked_add(shape.parameter_values().len() * size_of::<u64>())
        .and_then(|bytes| bytes.checked_add(size_of::<CapturedInputFact>()))
        .ok_or_else(|| invalid_error("retained byte accounting"))
}

#[derive(Clone, Debug)]
pub struct CapturedInputBatch {
    pub range: InputSequenceRange,
    pub facts: Box<[CapturedInputFact]>,
    pub batch_hash: [u8; 32],
    retained_bytes: usize,
}

impl CapturedInputBatch {
    pub fn new(facts: Vec<CapturedInputFact>) -> MResult<Self> {
        let first = facts.first().ok_or_else(|| invalid_error("empty batch"))?;
        let mut identities = BTreeSet::new();
        let mut expected = first.sequence.get();
        let mut retained_bytes = size_of::<Self>();
        let mut hash = blake3::Hasher::new();
        hash.update(b"mech-resident-input-batch-v2");
        for fact in &facts {
            if fact.sequence.get() != expected || !identities.insert((fact.node, fact.slot)) {
                return Err(invalid_error("sequence or duplicate identity"));
            }
            expected = expected
                .checked_add(1)
                .ok_or_else(|| invalid_error("sequence overflow"))?;
            retained_bytes = retained_bytes
                .checked_add(fact.retained_bytes())
                .ok_or_else(|| invalid_error("retained byte accounting"))?;
            hash.update(&fact.sequence.get().to_le_bytes());
            hash.update(&fact.requirement.get().to_le_bytes());
            hash.update(&fact.node.get().to_le_bytes());
            hash.update(&fact.slot.get().to_le_bytes());
            hash.update(fact.schema_key.as_bytes());
            hash.update(fact.payload_hash.as_bytes());
            hash.update(&[u8::from(fact.trigger)]);
        }
        let last = facts.last().expect("nonempty facts").sequence;
        Ok(Self {
            range: InputSequenceRange::new(first.sequence, last)?,
            facts: facts.into_boxed_slice(),
            batch_hash: *hash.finalize().as_bytes(),
            retained_bytes,
        })
    }

    pub const fn retained_bytes(&self) -> usize {
        self.retained_bytes
    }
}

impl Sealed for CapturedInputBatch {}

impl AccountedRecord for CapturedInputBatch {
    fn validate_for_recording(&self) -> MResult<()> {
        if self.facts.is_empty() {
            return Err(invalid_error("empty batch"));
        }
        Ok(())
    }

    fn retained_bytes(&self) -> usize {
        self.retained_bytes
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InvalidCapturedInputFact {
    pub reason: &'static str,
}

impl MechErrorKind for InvalidCapturedInputFact {
    fn name(&self) -> &str {
        "InvalidCapturedInputFact"
    }

    fn message(&self) -> String {
        format!("captured resident input fact is invalid: {}", self.reason)
    }
}

fn invalid_error(reason: &'static str) -> MechError {
    MechError::new(InvalidCapturedInputFact { reason }, None)
}
