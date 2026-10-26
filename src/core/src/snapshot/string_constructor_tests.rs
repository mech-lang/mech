use super::*;
use crate::{SchemaDraft, SchemaTableBuilder};
use core::cell::{Cell, RefCell};

fn schema(body: SchemaBody) -> (SchemaTable, SchemaId) {
    let mut builder = SchemaTableBuilder::new();
    let handle = builder
        .insert(
            SchemaDraft {
                body,
                dimension_parameters: Box::new([]),
            }
            .finalize()
            .unwrap(),
        )
        .unwrap();
    let built = builder.finish().unwrap();
    let id = built.resolve(handle).unwrap();
    (built.table, id)
}

#[test]
fn prepared_and_ordinary_strings_share_validation_storage_and_identity_rules() {
    let (schemas, id) = schema(SchemaBody::String);
    for text in [
        "".to_owned(),
        "a\0é\r\n🧑‍🚀".to_owned(),
        "acgt".repeat(16_384),
    ] {
        let text = text.into_boxed_str().into_string();
        let pointer = text.as_ptr();
        let length = text.len();
        let prepared = StringSnapshotConstructor::new(
            id,
            Box::new([]),
            length,
            &SnapshotValidationContext::new(&schemas),
        )
        .unwrap();
        let output = prepared.finish(text.clone()).unwrap();
        let ordinary = ValueDraft {
            schema: id,
            shape_values: Box::new([]),
            data: ValueDataDraft::String(text),
        }
        .finalize(&SnapshotValidationContext::new(&schemas))
        .unwrap();
        assert!(
            matches!(ordinary.data(), ValueData::String(value) if value.len() == length && value.as_ptr() == pointer)
        );
        assert!(matches!((output.data(), ordinary.data()),
            (ValueData::String(left), ValueData::String(right)) if left == right));
        assert_eq!(output.schema_key(), ordinary.schema_key());
        assert_eq!(output.shape(), ordinary.shape());
        assert!(!output.shares_frozen_storage(&ordinary));
        output.validate_against(&schemas).unwrap();
        ordinary.validate_against(&schemas).unwrap();
    }
}

#[test]
fn length_mismatch_returns_the_original_unconsumed_string() {
    let (schemas, id) = schema(SchemaBody::String);
    let mut text = String::with_capacity(1024);
    text.push_str("untouched");
    let pointer = text.as_ptr();
    let capacity = text.capacity();
    let prepared = StringSnapshotConstructor::new(
        id,
        Box::new([]),
        text.len() + 1,
        &SnapshotValidationContext::new(&schemas),
    )
    .unwrap();
    let (text, _) = prepared.finish(text).unwrap_err();
    assert_eq!(text, "untouched");
    assert_eq!(text.as_ptr(), pointer);
    assert_eq!(text.capacity(), capacity);
}

struct Authority {
    fail_at: usize,
    calls: RefCell<Vec<(u64, u32)>>,
    count: Cell<usize>,
}
impl SnapshotConstructionAuthority for Authority {
    fn admit_snapshot_allocation(
        &self,
        bytes: u64,
        alignment: u32,
    ) -> Result<(), crate::MemoryRuntimeError> {
        self.calls.borrow_mut().push((bytes, alignment));
        let count = self.count.get() + 1;
        self.count.set(count);
        if count == self.fail_at {
            return Err(crate::MemoryRuntimeError::CandidateValidationFailed {
                object: None,
                reason: "injected preparation denial".into(),
            });
        }
        Ok(())
    }
    fn allocation_object(&self) -> Option<crate::MemoryObjectId> {
        None
    }
}

#[test]
fn preparation_uses_the_same_admission_sequence_and_fails_before_consuming_text() {
    let (schemas, id) = schema(SchemaBody::String);
    let text = "not consumed on any denial".to_owned();
    let pointer = text.as_ptr();
    for fail_at in 1..=5 {
        let authority = Authority {
            fail_at,
            calls: RefCell::new(Vec::new()),
            count: Cell::new(0),
        };
        let context =
            SnapshotValidationContext::new(&schemas).with_construction_authority(&authority);
        assert!(StringSnapshotConstructor::new(id, Box::new([]), text.len(), &context).is_err());
        assert_eq!(authority.count.get(), fail_at);
        assert_eq!(authority.calls.borrow()[0], (text.len() as u64, 1));
        assert_eq!(text.as_ptr(), pointer);
        assert_eq!(text, "not consumed on any denial");
    }
    let record = |prepared: bool| {
        let authority = Authority {
            fail_at: usize::MAX,
            calls: RefCell::new(Vec::new()),
            count: Cell::new(0),
        };
        let context =
            SnapshotValidationContext::new(&schemas).with_construction_authority(&authority);
        let output = if prepared {
            StringSnapshotConstructor::new(id, Box::new([]), text.len(), &context)
                .unwrap()
                .finish(text.clone())
                .unwrap()
        } else {
            ValueDraft {
                schema: id,
                shape_values: Box::new([]),
                data: ValueDataDraft::String(text.clone()),
            }
            .finalize(&context)
            .unwrap()
        };
        output.validate_against(&schemas).unwrap();
        authority.calls.into_inner()
    };
    let calls = record(true);
    assert_eq!(calls.len(), 5);
    assert_eq!(calls, record(false));
}

#[test]
fn non_string_schemas_cannot_prepare_string_storage() {
    let (schemas, id) = schema(SchemaBody::Bool);
    assert!(
        StringSnapshotConstructor::new(
            id,
            Box::new([]),
            0,
            &SnapshotValidationContext::new(&schemas)
        )
        .is_err()
    );
}
