//! Differential execution of the actual selected constructor and its ordinary
//! current-shape binding, including fresh payloads, refusals and retries.
use super::tests::{Inputs, layout, schema};
use super::*;
use mech_core::snapshot::SnapshotValidationContext;
use mech_core::{Value, ValueDataDraft, ValueDraft};

struct Fixture {
    kernel: BoundResidentKernel,
    reference: BoundResidentKernel,
    schemas: mech_core::SchemaTable,
    scalar: mech_core::SchemaId,
    map: mech_core::SchemaId,
}

fn fixture(record: bool) -> Fixture {
    let scalar = SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64);
    let map = SchemaBody::Map {
        key: Box::new(SchemaBody::String),
        value: Box::new(scalar.clone()),
        cardinality: CardinalitySpec::Dynamic { upper_bound: None },
    };
    let fields = [scalar.clone(), scalar.clone(), map.clone()];
    let output = if record {
        SchemaBody::Record(
            fields
                .iter()
                .cloned()
                .enumerate()
                .map(|(i, schema)| mech_core::SchemaField {
                    name: format!("field{i}").into(),
                    schema,
                })
                .collect(),
        )
    } else {
        SchemaBody::Tuple(fields.into())
    };
    let mut builder = mech_core::SchemaTableBuilder::new();
    let scalar = builder.insert(schema(scalar)).unwrap();
    let map = builder.insert(schema(map)).unwrap();
    let output = builder.insert(schema(output)).unwrap();
    let built = builder.finish().unwrap();
    let (scalar, map, output) = (
        built.resolve(scalar).unwrap(),
        built.resolve(map).unwrap(),
        built.resolve(output).unwrap(),
    );
    let (schemas, _) = built.into_parts();
    let ids = [scalar, scalar, map];
    let contract = ResolvedOperationContract::Declared(mech_core::DeclaredOperationContract {
        inputs: ids
            .iter()
            .map(|id| mech_core::ResolvedInputPort {
                schema: *id,
                access: AccessMode::Read,
                delivery: DeliveryMode::Signal,
            })
            .collect(),
        outputs: Box::new([mech_core::ResolvedOutputPort {
            schema: output,
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            },
            alias: AliasPolicy::NoAlias,
            change_detection: ChangeDetectionPolicy::KernelReported,
        }]),
        interaction: ExternalInteraction::Pure,
    });
    let mut ports = ids.map(|id| layout(&schemas, id, ResidentValueKind::Snapshot));
    for port in &mut ports {
        port.activation_fixed_shape = false;
    }
    let mut output = layout(&schemas, output, ResidentValueKind::Snapshot);
    output.activation_fixed_shape = false;
    let kernel = bind_composite_pack(&ResidentKernelBindRequest {
        contract: &contract,
        schemas: &schemas,
        inputs: &ports,
        output,
    })
    .unwrap();
    let plan = kernel.retained_state::<CompositePackPlan>().unwrap();
    assert!(!plan.constructor_is_fixed);
    assert!(plan.reuse_closed_constructor);
    assert!(plan.normalization.compute_work > 0);
    let mut reference = plan.clone();
    reference.reuse_closed_constructor = false;
    let reference = BoundResidentKernel::new(composite_pack, Box::new([]))
        .with_retained_state(Arc::new(reference));
    Fixture {
        kernel,
        reference,
        schemas,
        scalar,
        map,
    }
}

impl Fixture {
    fn value(&self, id: mech_core::SchemaId, data: ValueDataDraft) -> Value {
        ValueDraft {
            schema: id,
            shape_values: Box::new([]),
            data,
        }
        .finalize(&SnapshotValidationContext::new(&self.schemas))
        .unwrap()
    }
    fn values(&self, count: usize, number: u64) -> [Option<Value>; 3] {
        let entries = (0..count)
            .map(|n| mech_core::snapshot::MapEntryDraft {
                items: Box::new([
                    ValueDataDraft::String(format!("{n:04}é\0🦀").into()),
                    ValueDataDraft::U64(number.wrapping_add(n as u64)),
                ]),
            })
            .collect();
        [
            Some(self.value(self.scalar, ValueDataDraft::U64(number))),
            Some(self.value(self.scalar, ValueDataDraft::U64(u64::MAX))),
            Some(self.value(self.map, ValueDataDraft::Map(entries))),
        ]
    }
}

#[test]
fn closed_constructor_reuses_schema_only_and_preserves_every_logical_debit() {
    for record in [false, true] {
        let f = fixture(record);
        for count in [0, 1, 28, 65] {
            let mut observations = Vec::new();
            for kernel in [&f.reference, &f.kernel] {
                let memory = mech_core::ManagedMemoryBudget::new(128 * 1024 * 1024);
                let work = super::super::budget::NativeLibraryWorkBudget::new(
                    1_000_000_000,
                    1_000_000_000,
                    &memory,
                )
                .unwrap();
                let mut output = [None];
                let mut runs = Vec::new();
                for number in [7, 7, 99, u64::MAX, 7] {
                    let values = f.values(count, number);
                    let refs = values
                        .each_ref()
                        .map(|v| ResidentValueRef::Snapshot(core::slice::from_ref(v)));
                    let inputs = Inputs(&refs);
                    let plan = f.kernel.retained_state::<CompositePackPlan>().unwrap();
                    assert!(closed_constructor_matches_current(plan, &inputs));
                    let result = work
                        .with_scope(|| {
                            kernel.execute(&inputs, ResidentValueMut::Snapshot(&mut output))
                        })
                        .unwrap();
                    assert!(result.is_ok(), "{result:?}");
                    runs.push((
                        result,
                        output[0].as_ref().unwrap().canonical_data_draft().unwrap(),
                        work.used(),
                    ));
                }
                drop(output);
                drop(work);
                assert_eq!(memory.used_bytes(), 0);
                observations.push(runs);
            }
            assert_eq!(
                observations[0], observations[1],
                "record={record}, count={count}"
            );
            assert_eq!(
                observations[1]
                    .iter()
                    .map(|r| r.0.clone().unwrap())
                    .collect::<Vec<_>>(),
                [true, false, true, true, true]
            );
        }
    }
}

#[test]
fn closed_constructor_preserves_first_denial_atomic_publication_and_retry() {
    let f = fixture(false);
    for limit in [
        0, 1, 63, 64, 127, 128, 255, 256, 511, 512, 1024, 4096, 16384, 65536, 1_000_000,
    ] {
        let mut observations = Vec::new();
        for kernel in [&f.reference, &f.kernel] {
            let memory = mech_core::ManagedMemoryBudget::new(128 * 1024 * 1024);
            let work =
                super::super::budget::NativeLibraryWorkBudget::new(limit, limit, &memory).unwrap();
            let values = f.values(28, 42);
            let refs = values
                .each_ref()
                .map(|v| ResidentValueRef::Snapshot(core::slice::from_ref(v)));
            let inputs = Inputs(&refs);
            let mut output = [None];
            let first = work
                .with_scope(|| kernel.execute(&inputs, ResidentValueMut::Snapshot(&mut output)))
                .unwrap();
            let after_first = output[0]
                .as_ref()
                .map(|v| v.canonical_data_draft().unwrap());
            if first.is_err() {
                assert!(output[0].is_none());
            }
            let spent = work.used();
            let retry = work
                .with_scope(|| kernel.execute(&inputs, ResidentValueMut::Snapshot(&mut output)))
                .unwrap();
            observations.push((
                first,
                after_first,
                spent,
                retry,
                output[0]
                    .as_ref()
                    .map(|v| v.canonical_data_draft().unwrap()),
                work.used(),
            ));
        }
        assert_eq!(observations[0], observations[1], "limit={limit}");
    }
    let mut values = f.values(1, 42);
    values.swap(0, 2);
    let refs = values
        .each_ref()
        .map(|v| ResidentValueRef::Snapshot(core::slice::from_ref(v)));
    let inputs = Inputs(&refs);
    let plan = f.kernel.retained_state::<CompositePackPlan>().unwrap();
    assert!(!closed_constructor_matches_current(plan, &inputs));
    for kernel in [&f.reference, &f.kernel] {
        let mut output = [None];
        assert_eq!(
            kernel.execute(&inputs, ResidentValueMut::Snapshot(&mut output)),
            Err(ResidentKernelError::InvalidInput)
        );
        assert!(output[0].is_none());
    }
}
