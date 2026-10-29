//! A private child is packed, returned, wrapped twice and projected repeatedly.
//! This is the costs/saved transport required by the compression controller.
use super::{artifacts, inputs, limits, reference_result};
use mech_core::{ManagedMemoryBudget, ValueData, ValueDataDraft as Data};
use mech_engine::native_artifact::{
    NativeCollectionPrimitive as Primitive, NativeIterationAction as Action,
    lower_native_program_artifact,
};
use mech_gpu::native_artifact::JitProgramArtifact;

const NESTED: &str = "+> sequence\n<costs> := <{literal<[u32]> length<[u32]> offset<[u32]>}>\n<selected> := <{nodes<[u64]> costs<costs> marker<u64>}>\n<planned> := <{selected<selected> saved<costs>}>\nwrap(nodes<[u64]>, values<[u32]>, marker<u64>) => (out<planned>) :=\n  costs := {literal: values, length: values, offset: values}\n  selected := {nodes: nodes, costs: costs, marker: marker}\n  out := {selected: selected, saved: costs}.\nchecked(capacity<u64>, marker<u64>, allowed<bool>, divisor<u64>) => (out<[u64]>) :=\n  returned := wrap(sequence/with-capacity(0u64, capacity), [7u32], marker)\n  current! := returned.selected.marker == marker\n  same! := returned.selected.costs.literal[1u64] == returned.saved.length[1u64]\n  other! := returned.saved.offset[1u64] == 7u32\n  enabled! := allowed\n  word := marker + 12u64 / divisor\n  out := sequence/append(returned.selected.nodes, word).\nchecked(capacity<u64>, marker<u64>, allowed<bool>, divisor<u64>)\n";

#[test]
fn private_nested_pack_return_and_projection_select_the_complete_component() {
    for artifact in artifacts(NESTED).0 {
        let plan =
            lower_native_program_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let frames = plan.facts().buffer_frames();
        let mut packs = 0;
        let mut projections = 0;
        for (site, block) in plan.blocks().iter().enumerate() {
            let Action::LibraryCall {
                template,
                operands,
                output,
            } = block.action()
            else {
                continue;
            };
            if matches!(template.primitive(), Primitive::CompositePack(_))
                && matches!(template.output().body(), mech_core::SchemaBody::Record(_))
            {
                assert!(frames.slot(*output).is_some(), "pack site {site}");
                assert!(frames.action(site).is_some());
                packs += 1;
            } else if template.primitive() == Primitive::RecordFieldAccess {
                assert!(frames.slot(operands[0]).is_some(), "read site {site}");
                assert!(frames.action(site).is_some());
                if matches!(template.output().body(), mech_core::SchemaBody::Record(_)) {
                    assert!(frames.slot(*output).is_some(), "child site {site}");
                    projections += 1;
                }
            }
        }
        assert_eq!(packs, 3);
        assert!(projections >= 5);
    }
}

#[test]
fn nested_current_fields_preserve_errors_publication_aliases_and_retry() {
    for source in [
        NESTED.to_owned(),
        NESTED.replace(
            "  enabled! := allowed\n  word := marker + 12u64 / divisor",
            "  word := marker + 12u64 / divisor\n  enabled! := allowed",
        ),
    ] {
        let assertion_first = source == NESTED;
        for artifact in artifacts(&source).0 {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            super::record_buffer_qualification::assert_build_owned_image_round_trip(&jit);
            let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
            let mut output = vec![None; artifact.outputs().len()];
            for (capacity, marker, allowed, divisor) in [
                (16, 7u64, true, 3u64),
                (16, 91, true, 2),
                (16, 13, false, 0),
                (16, 37, true, 0),
                (16, u64::MAX, true, 1),
                (16, 19, true, 4),
            ] {
                let args = inputs(
                    &artifact,
                    &[
                        ("capacity", Data::U64(capacity)),
                        ("marker", Data::U64(marker)),
                        ("allowed", Data::Bool(allowed)),
                        ("divisor", Data::U64(divisor)),
                    ],
                    &memory,
                );
                let aliases = args.clone();
                let prior = output.clone();
                let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let result = jit.evaluate_into(&args, &mut output, limits(), &memory);
                let resident = reference_result(&artifact, &args);
                if allowed && divisor != 0 && marker != u64::MAX {
                    result.unwrap();
                    super::equal(&artifact, &output, &resident.unwrap());
                    let value = output.last().unwrap().as_ref().unwrap();
                    assert_eq!(
                        value.canonical_data_draft().unwrap(),
                        Data::Matrix(vec![Data::U64(marker + 12 / divisor)].into())
                    );
                    let ValueData::Matrix(matrix) = value.data() else {
                        panic!()
                    };
                    assert_eq!(matrix.unsigned_buffer_capacity(), Some(capacity as usize));
                } else {
                    let native = format!("{:?}", result.unwrap_err());
                    let resident = resident.unwrap_err();
                    let assertion = !allowed && assertion_first;
                    assert_eq!(native.contains("Integrity"), assertion, "{native}");
                    assert_eq!(
                        resident.contains("LexicalIntegrity"),
                        assertion,
                        "{resident}"
                    );
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        retained
                    );
                    assert!(output.iter().zip(&prior).all(|(v, p)| {
                        v.as_ref()
                            .unwrap()
                            .shares_frozen_storage(p.as_ref().unwrap())
                    }));
                }
                assert!(
                    args.iter()
                        .zip(&aliases)
                        .all(|(v, a)| v.shares_frozen_storage(a))
                );
            }
            drop(output);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn nested_private_path_and_output_buffers_keep_the_real_reservation() {
    let source = "+> sequence\nwrap(nodes<[u64]>, words<[u32]>) => (out<{selected<{nodes<[u64]> costs<{literal<[u32]>}>}>}>) :=\n  costs := {literal: words}\n  selected := {nodes: nodes, costs: costs}\n  out := {selected: selected}.\nreturned := wrap(sequence/with-capacity(0u64, capacity<u64>), [7u32])\nsequence/append(returned.selected.nodes, 17u64)\n";
    for (width, capacity) in [(64, 305257), (8, 6633832)] {
        let source = if width == 64 {
            source.to_owned()
        } else {
            source
                .replace("[u64]", "[u8]")
                .replace("0u64, capacity", "0u8, capacity")
                .replace("17u64", "17u8")
        };
        for artifact in artifacts(&source).0 {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let memory = ManagedMemoryBudget::new(128 * 1024 * 1024);
            let args = inputs(&artifact, &[("capacity", Data::U64(capacity))], &memory);
            let mut output = vec![None; artifact.outputs().len()];
            let mut policy = limits();
            policy.library.max_cloned_bytes = Some(0);
            jit.evaluate_into(&args, &mut output, policy, &memory)
                .unwrap();
            let value = output.last().unwrap().as_ref().unwrap();
            assert_eq!(
                value.canonical_data_draft().unwrap(),
                Data::Matrix(
                    vec![if width == 64 {
                        Data::U64(17)
                    } else {
                        Data::U8(17)
                    }]
                    .into()
                )
            );
            let ValueData::Matrix(matrix) = value.data() else {
                panic!()
            };
            assert_eq!(matrix.unsigned_buffer_capacity(), Some(capacity as usize));
            drop((args, output));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn composed_children_keep_aliases_and_all_current_scalar_bits_across_parents() {
    let source = "+> sequence\nchild := {nodes: values<[u64]>, bytes: bytes<[u8]>, signed: signed<i64>, real: real<f64>, allowed: allowed<bool>}\nfirst := {child: child, marker: marker<u64>}\nsecond := {child: child, marker: marker + 1u64}\nchanged := sequence/append(sequence/clear(first.child.nodes), first.marker)\n(changed, second.child.nodes, first.child.bytes, second.child.signed, first.child.real, second.child.allowed, second.marker)\n";
    for artifact in artifacts(source).0 {
        let plan =
            lower_native_program_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let frames = plan.facts().buffer_frames();
        let selected = plan
            .blocks()
            .iter()
            .filter(|b| {
                matches!(b.action(),
            Action::LibraryCall { template, output, .. }
            if matches!(template.primitive(), Primitive::CompositePack(_))
                && matches!(template.output().body(), mech_core::SchemaBody::Record(_))
                && frames.slot(*output).is_some())
            })
            .count();
        assert_eq!(selected, 3);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(32 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for (count, marker, signed, real, allowed) in [
            (0, 0, i64::MIN, 0x8000000000000000, true),
            (3, 0xfedcba9876543210, i64::MAX, 0x7ff8000000000042, false),
            (1, 71, -19, 0x3ff0000000000000, true),
        ] {
            let values = Data::Matrix((0..count).map(|i| Data::U64(i + 257)).collect());
            let bytes = Data::Matrix((0..count + 1).map(|i| Data::U8(i as u8)).collect());
            let real = Data::F64(mech_core::snapshot::F64Bits::from_bits(real));
            let args = inputs(
                &artifact,
                &[
                    ("values", values.clone()),
                    ("bytes", bytes.clone()),
                    ("marker", Data::U64(marker)),
                    ("signed", Data::I64(signed)),
                    ("real", real.clone()),
                    ("allowed", Data::Bool(allowed)),
                ],
                &memory,
            );
            let aliases = args.clone();
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            super::equal(
                &artifact,
                &output,
                &reference_result(&artifact, &args).unwrap(),
            );
            assert_eq!(
                output
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::Tuple(
                    vec![
                        Data::Matrix(vec![Data::U64(marker)].into()),
                        values,
                        bytes,
                        Data::I64(signed),
                        real,
                        Data::Bool(allowed),
                        Data::U64(marker + 1)
                    ]
                    .into()
                )
            );
            assert!(
                args.iter()
                    .zip(&aliases)
                    .all(|(a, b)| a.shares_frozen_storage(b))
            );
        }
        drop(output);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn nested_public_escape_keeps_the_complete_component_canonical() {
    let source = format!(
        "{}wrap(values<[u64]>, words<[u32]>, marker<u64>)\n",
        NESTED.split("checked(capacity").next().unwrap()
    );
    for artifact in artifacts(&source).0 {
        let plan =
            lower_native_program_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let frames = plan.facts().buffer_frames();
        let mut declined = 0;
        for block in plan.blocks() {
            if let Action::LibraryCall {
                template, output, ..
            } = block.action()
                && matches!(template.primitive(), Primitive::CompositePack(_))
                && matches!(template.output().body(), mech_core::SchemaBody::Record(_))
            {
                assert!(frames.slot(*output).is_none());
                declined += 1;
            }
        }
        assert_eq!(declined, 3);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(16 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for count in [0, 3, 1] {
            let args = inputs(
                &artifact,
                &[
                    (
                        "values",
                        Data::Matrix((0..count).map(|i| Data::U64(17 + i)).collect()),
                    ),
                    (
                        "words",
                        Data::Matrix((0..count + 1).map(|i| Data::U32(29 + i as u32)).collect()),
                    ),
                    ("marker", Data::U64(count + 91)),
                ],
                &memory,
            );
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            let record = |fields: Vec<(&str, Data)>| {
                Data::Record(
                    fields
                        .into_iter()
                        .map(|(name, value)| mech_core::snapshot::NamedValueDraft {
                            name: name.into(),
                            value,
                        })
                        .collect(),
                )
            };
            let values = Data::Matrix((0..count).map(|i| Data::U64(17 + i)).collect());
            let words = Data::Matrix((0..count + 1).map(|i| Data::U32(29 + i as u32)).collect());
            let costs = record(vec![
                ("literal", words.clone()),
                ("length", words.clone()),
                ("offset", words),
            ]);
            let expected = record(vec![
                (
                    "selected",
                    record(vec![
                        ("nodes", values),
                        ("costs", costs.clone()),
                        ("marker", Data::U64(count + 91)),
                    ]),
                ),
                ("saved", costs),
            ]);
            assert_eq!(
                output
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                expected
            );
        }
        drop(output);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn recursive_composed_results_keep_current_visits_and_recover_after_rejection() {
    let source = "+> sequence\n<leaf> := <{nodes<[u64]> marker<u64> allowed<bool>}>\n<outer> := <{selected<leaf>}>\nwalk(nodes<[u64]>, marker<u64>, left<u64>, allowed<bool>) => (out<outer>) :=\n  leaf := {nodes: nodes, marker: marker, allowed: allowed}\n  outer := {selected: leaf}\n  enabled! := outer.selected.allowed || left == 0u64\n  out := ((left > 0u64)? | true => walk(sequence/append(outer.selected.nodes, outer.selected.marker), marker + 1u64, left - 1u64, allowed == false) | false => outer).\nreturned := walk(sequence/with-capacity(0u64, 16u64), marker<u64>, steps<u64>, allowed<bool>)\n(returned.selected.nodes, returned.selected.marker, returned.selected.allowed)\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let frames = jit.descriptor().plan().facts().buffer_frames();
        assert!(
            jit.descriptor()
                .plan()
                .blocks()
                .iter()
                .any(|b| matches!(b.action(),
            Action::LibraryCall { template, output, .. }
            if matches!(template.output().body(), mech_core::SchemaBody::Record(_))
                && frames.slot(*output).is_some()))
        );
        let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for (steps, marker, allowed) in
            [(0, 9, false), (1, 37, true), (2, 71, true), (1, 113, true)]
        {
            let args = inputs(
                &artifact,
                &[
                    ("steps", Data::U64(steps)),
                    ("marker", Data::U64(marker)),
                    ("allowed", Data::Bool(allowed)),
                ],
                &memory,
            );
            let prior = output.clone();
            let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
            let result = jit.evaluate_into(&args, &mut output, limits(), &memory);
            let resident = reference_result(&artifact, &args);
            if steps == 2 {
                assert!(format!("{:?}", result.unwrap_err()).contains("Integrity"));
                assert!(resident.unwrap_err().contains("LexicalIntegrity"));
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    retained
                );
                assert!(output.iter().zip(&prior).all(|(a, b)| {
                    a.as_ref()
                        .unwrap()
                        .shares_frozen_storage(b.as_ref().unwrap())
                }));
            } else {
                result.unwrap();
                super::equal(&artifact, &output, &resident.unwrap());
                assert_eq!(
                    output
                        .last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    Data::Tuple(
                        vec![
                            Data::Matrix(
                                if steps == 0 {
                                    vec![]
                                } else {
                                    vec![Data::U64(marker)]
                                }
                                .into()
                            ),
                            Data::U64(marker + steps),
                            Data::Bool(if steps == 0 { allowed } else { !allowed })
                        ]
                        .into()
                    )
                );
            }
        }
        drop(output);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn composed_projection_keeps_exact_logical_output_bounds_and_current_shapes() {
    use mech_core::snapshot::{NamedValueDraft, SnapshotValidationContext, ValueDraft};
    use mech_engine::native_artifact::{
        NativeBufferFrameLane as Lane, NativeLibraryWorkBudget, prepare_native_program_library_call,
    };
    let source = "+> sequence\nchild := {literal: values<[u32]>, length: values, offset: values}\nparent := {selected: child}\nsequence/length(parent.selected.literal)\n";
    for artifact in artifacts(source).0 {
        let catalog = mech_stdlib::source_catalog();
        let plan = lower_native_program_artifact(&artifact, &catalog).unwrap();
        let frames = plan.facts().buffer_frames();
        let (site, template, operands, output) = plan.blocks().iter().enumerate().find_map(|(site,b)| {
            let Action::LibraryCall { template, operands, output } = b.action() else { return None };
            let mech_core::SchemaBody::Record(fields) = template.output().body() else { return None };
            (template.primitive()==Primitive::RecordFieldAccess && fields.len()==3
                && fields.iter().all(|f|matches!(&f.schema,mech_core::SchemaBody::Matrix { element, .. }
                    if **element==mech_core::SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W32))))
                .then_some((site,template,operands,*output))
        }).unwrap();
        let mut call = prepare_native_program_library_call(&artifact, template, &catalog).unwrap();
        call.prepare_buffer_frame_composition(
            &frames,
            frames.action(site).unwrap().operation(),
            operands,
        )
        .unwrap();
        let memory = ManagedMemoryBudget::new(16 * 1024 * 1024);
        let admit = |value: mech_core::Value| {
            let mut reserve = memory
                .reserve_capacity(value.memory_budget_admission_bytes(&memory).unwrap())
                .unwrap();
            value.into_memory_budget(&mut reserve).unwrap()
        };
        let selector = admit(
            plan.constants()
                .iter()
                .find(|c| c.value() == operands[1])
                .unwrap()
                .constant()
                .clone(),
        );
        let mech_engine::native_artifact::NativeBufferFrameOperation::Project {
            source,
            ordinal,
            ..
        } = frames.action(site).unwrap().operation()
        else {
            panic!()
        };
        let field = frames.fields(*source).unwrap()[*ordinal];
        let leaves = frames.children(field).unwrap();
        let work = NativeLibraryWorkBudget::new(1 << 40, 1 << 40, &memory).unwrap();
        for count in [0, 3, 1] {
            let data = Data::Matrix((0..count).map(|i| Data::U32(19 + i)).collect());
            let values = leaves
                .iter()
                .map(|field| {
                    let definition = artifact.schemas().get(field.schema()).unwrap();
                    (
                        field.lane(),
                        admit(
                            ValueDraft {
                                schema: field.schema(),
                                shape_values: mech_core::shape_for_resolved_extents(
                                    definition,
                                    &[1, u64::from(count)],
                                )
                                .unwrap()
                                .parameter_values()
                                .to_vec()
                                .into(),
                                data: data.clone(),
                            }
                            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
                            .unwrap(),
                        ),
                    )
                })
                .collect::<Vec<_>>();
            assert!(
                values
                    .iter()
                    .all(|(lane, _)| matches!(lane, Lane::Managed { .. }))
            );
            let ty = &plan.values()[output.get() as usize];
            let mech_core::SchemaBody::Record(fields) = ty.definition().body() else {
                panic!()
            };
            let canonical = ValueDraft {
                schema: ty.schema(),
                shape_values:
                    mech_core::snapshot::CompositeSnapshotConstructor::shape_for_children(
                        ty.schema(),
                        &values
                            .iter()
                            .map(|(_, v)| (v.schema(), v.shape().clone()))
                            .collect::<Vec<_>>(),
                        artifact.schemas(),
                    )
                    .unwrap()
                    .parameter_values()
                    .to_vec()
                    .into(),
                data: Data::Record(
                    fields
                        .iter()
                        .map(|f| NamedValueDraft {
                            name: f.name.clone(),
                            value: data.clone(),
                        })
                        .collect(),
                ),
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap();
            let exact = canonical
                .retained_footprint(artifact.schemas())
                .unwrap()
                .retained_bytes;
            let read = |field: mech_engine::native_artifact::NativeBufferFrameField| {
                values
                    .iter()
                    .find(|(lane, _)| *lane == field.lane())
                    .map(|(_, v)| (Some(v), 0))
            };
            let mut policy = limits().library;
            policy.max_output_bytes = Some(exact);
            policy.max_cloned_bytes = Some(0);
            call.admit_buffer_frame_composition(
                &[None, None],
                &[0, 0],
                read,
                Some((&selector, *ordinal)),
                policy,
                &work,
                Default::default(),
            )
            .unwrap();
            let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
            let mut short = policy;
            short.max_output_bytes = Some(exact - 1);
            let error = call
                .admit_buffer_frame_composition(
                    &[None, None],
                    &[0, 0],
                    read,
                    Some((&selector, *ordinal)),
                    short,
                    &work,
                    Default::default(),
                )
                .unwrap_err();
            assert!(format!("{error:?}").contains("OutputBytes"), "{error:?}");
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                retained
            );
            call.admit_buffer_frame_composition(
                &[None, None],
                &[0, 0],
                read,
                Some((&selector, *ordinal)),
                policy,
                &work,
                Default::default(),
            )
            .unwrap();
        }
        drop((selector, work));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}
