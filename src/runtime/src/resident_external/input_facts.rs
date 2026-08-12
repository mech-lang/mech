use std::collections::BTreeSet;

use mech_core::{
    ApplicationRequirementId, CellSlotId, MResult, MechError, MechErrorKind, NodeId, SchemaKey,
    SchemaTable, ShapeInstance, Value, ValueHash,
};

use crate::{InputSequence, InputSequenceRange};

#[derive(Clone, Debug)]
pub struct CapturedInputFact {
    pub sequence: InputSequence,
    pub requirement: ApplicationRequirementId,
    pub node: NodeId,
    pub slot: CellSlotId,
    pub schema_key: SchemaKey,
    pub shape: ShapeInstance,
    pub value: Value,
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
        let schema = schemas.entry(value.schema()).ok_or_else(|| invalid("value schema"))?;
        if schema.key() != schema_key || value.schema_key() != schema_key || value.shape() != &shape
        {
            return Err(invalid("schema key or shape").unwrap_err());
        }
        let payload_hash = value
            .value_hash(schemas)
            .map_err(|_| invalid::<()>("payload hash").unwrap_err())?;
        let retained_bytes = value
            .canonical_payload_bytes(schemas)
            .map_err(|_| invalid::<()>("canonical payload").unwrap_err())?
            .len()
            .checked_add(shape.parameter_values().len() * size_of::<u64>())
            .and_then(|bytes| bytes.checked_add(size_of::<Self>()))
            .ok_or_else(|| invalid::<()>("retained byte accounting").unwrap_err())?;
        Ok(Self {
            sequence,
            requirement,
            node,
            slot,
            schema_key,
            shape,
            value,
            payload_hash,
            retained_bytes,
        })
    }

    pub const fn retained_bytes(&self) -> usize {
        self.retained_bytes
    }
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
        let first = facts.first().ok_or_else(|| invalid::<()>("empty batch").unwrap_err())?;
        let mut requirements = BTreeSet::new();
        let mut slots = BTreeSet::new();
        let mut expected = first.sequence.get();
        let mut retained_bytes = size_of::<Self>();
        let mut hash = blake3::Hasher::new();
        hash.update(b"mech-resident-input-batch-v1");
        for fact in &facts {
            if fact.sequence.get() != expected
                || !requirements.insert(fact.requirement)
                || !slots.insert(fact.slot)
            {
                return Err(invalid("sequence or duplicate identity").unwrap_err());
            }
            expected = expected
                .checked_add(1)
                .ok_or_else(|| invalid::<()>("sequence overflow").unwrap_err())?;
            retained_bytes = retained_bytes
                .checked_add(fact.retained_bytes())
                .ok_or_else(|| invalid::<()>("retained byte accounting").unwrap_err())?;
            hash.update(&fact.sequence.get().to_le_bytes());
            hash.update(&fact.requirement.get().to_le_bytes());
            hash.update(&fact.node.get().to_le_bytes());
            hash.update(&fact.slot.get().to_le_bytes());
            hash.update(fact.schema_key.as_bytes());
            hash.update(fact.payload_hash.as_bytes());
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

fn invalid<T>(reason: &'static str) -> MResult<T> {
    Err(MechError::new(InvalidCapturedInputFact { reason }, None))
}
