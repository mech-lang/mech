//! Current keys, account membership and words remain checked on composed frames.
use super::*;
use mech_core::{SchemaBody, SchemaDraft, SchemaTableBuilder};
use mech_engine::native_artifact::{
    NativeBufferFrameLane as Lane, NativeBufferFrameOperation as Operation, NativeLibraryWorkBudget,
};

const SOURCE: &str = "+> sequence\n<entry> := <{x<u32>}>\n<child> := <{a<[u32]> n<u64>}>\n<parent> := <{child<child> rows<[entry]>}>\nwrap(a<[u32]>, rows<[entry]>, n<u64>) => (p<parent>) :=\n  c := {a: a, n: n}\n  p := {child: c, rows: rows}.\ninspect(a<[u32]>, rows<[entry]>, n<u64>) => (out<u64>) :=\n  p := wrap(a, rows, n)\n  out := sequence/length(p.child.a) + p.child.n + sequence/length(p.rows).\ninspect(a<[u32]>, rows<[entry]>, n<u64>)\n";

fn admitted(raw: Value, memory: &ManagedMemoryBudget) -> Value {
    let mut reserve = memory
        .reserve_capacity(raw.memory_budget_admission_bytes(memory).unwrap())
        .unwrap();
    raw.into_memory_budget(&mut reserve).unwrap()
}

#[test]
fn composition_uses_current_schema_key_account_and_bits_on_every_visit() {
    let mut expected = None;
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let metadata = jit.descriptor().metadata();
        let layout = &metadata.buffer_frames;
        let (site, operands, child, rows_at) = metadata
            .plan
            .blocks()
            .iter()
            .enumerate()
            .find_map(|(site, block)| {
                let Operation::Pack { output } = layout.action(site)?.operation() else {
                    return None;
                };
                let NativeIterationAction::LibraryCall { operands, .. } = block.action() else {
                    return None;
                };
                let fields = layout.fields(*output)?;
                let child = fields.iter().find_map(|f| layout.children(*f))?;
                let rows = operands.iter().position(|v| {
                    matches!(metadata.plan.values()[v.get() as usize].definition().body(),
                        SchemaBody::Matrix { element, .. } if matches!(element.as_ref(), SchemaBody::Record(_)))
                })?;
                Some((site, operands, child, rows))
            })
            .expect("a real composed pack with an opaque aggregate matrix");
        let call = metadata.calls[site].as_ref().unwrap();
        let array = child
            .iter()
            .find(|f| matches!(f.lane(), Lane::Managed { .. }))
            .unwrap();
        let rows_schema = metadata.plan.values()[operands[rows_at].get() as usize].schema();
        let raw = |schema, data| {
            let shape = mech_core::shape_for_resolved_extents(
                artifact.schemas().get(schema).unwrap(),
                &[1, 1],
            )
            .unwrap();
            ValueDraft {
                schema,
                shape_values: shape.parameter_values().into(),
                data,
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap()
        };
        let memory = mode.memory_budget();
        let array_value = admitted(
            raw(array.schema(), Data::Matrix(vec![Data::U32(3)].into())),
            &memory,
        );
        let rows_raw = raw(
            rows_schema,
            Data::Matrix(
                vec![Data::Record(
                    vec![mech_core::snapshot::NamedValueDraft {
                        name: "x".into(),
                        value: Data::U32(4),
                    }]
                    .into(),
                )]
                .into(),
            ),
        );
        let rows = admitted(rows_raw.clone(), &memory);
        // Build a valid foreign Bool at the SAME ID. Only its semantic key and
        // value differ; an ID-only check must not accept this current owner.
        let mut builder = SchemaTableBuilder::new();
        for count in 1..=rows_schema.get() {
            builder
                .insert(
                    SchemaDraft {
                        dimension_parameters: Box::new([]),
                        body: SchemaBody::Tuple(vec![SchemaBody::Bool; count as usize].into()),
                    }
                    .finalize()
                    .unwrap(),
                )
                .unwrap();
        }
        let prefix = builder.finish().unwrap().table;
        let mut boolean = SchemaTableBuilder::new();
        boolean
            .insert(
                SchemaDraft {
                    dimension_parameters: Box::new([]),
                    body: SchemaBody::Bool,
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let foreign = prefix
            .extend_preserving_ids(&boolean.finish().unwrap().table)
            .unwrap();
        let wrong_key = admitted(
            ValueDraft {
                schema: rows_schema,
                shape_values: Box::new([]),
                data: Data::Bool(true),
            }
            .finalize(&SnapshotValidationContext::new(&foreign))
            .unwrap(),
            &memory,
        );
        assert_eq!(wrong_key.schema(), rows.schema());
        assert_ne!(wrong_key.schema_key(), rows.schema_key());
        let aliases = [array_value.clone(), rows.clone(), wrong_key.clone()];
        let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
        let mut results = Vec::new();
        for (name, owner, bits, limit, succeeds) in [
            ("valid", &rows, 0, u64::MAX, true),
            ("key", &wrong_key, 0, u64::MAX, false),
            ("bits", &rows, 1, u64::MAX, false),
            ("unadmitted", &rows_raw, 0, u64::MAX, false),
            ("funding-before-key", &wrong_key, 0, 0, false),
            ("retry", &rows, 0, u64::MAX, true),
        ] {
            let mut inputs = vec![None; operands.len()];
            inputs[rows_at] = Some(owner.clone());
            let mut words = vec![0; operands.len()];
            words[rows_at] = bits;
            let work = NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &memory).unwrap();
            let mut limits = mode.execution().library;
            limits.max_compute_work = Some(limit);
            let result = call.admit_buffer_frame_composition(
                &inputs,
                &words,
                |field| {
                    Some(match field.lane() {
                        Lane::Managed { .. } => (Some(&array_value), 0),
                        Lane::Scalar { .. } => (None, 7),
                        Lane::Composite { .. } => panic!("only current leaves are requested"),
                    })
                },
                None,
                limits,
                &work,
                mech_core::ResourceDemand::default(),
            );
            assert_eq!(result.is_ok(), succeeds, "{name}: {result:?}");
            if matches!(name, "key" | "bits" | "unadmitted") {
                assert_eq!(format!("{result:?}"), "Err(InvalidInput)");
            }
            if name == "funding-before-key" {
                assert!(format!("{result:?}").contains("ComputeWork"));
                assert_eq!(work.used(), (0, 0));
            }
            results.push((name, format!("{result:?}"), work.used()));
            drop((inputs, work));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                retained
            );
            assert!(array_value.shares_frozen_storage(&aliases[0]));
            assert!(rows.shares_frozen_storage(&aliases[1]));
        }
        println!("composition-key-visits: {results:?}");
        assert_eq!(&results, expected.get_or_insert_with(|| results.clone()));
        drop((rows, rows_raw, wrong_key, array_value, aliases));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}
