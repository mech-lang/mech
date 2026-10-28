//! Scalar fields use the selected record selector without transient Values.
use super::{allocation_probe, artifacts, inputs, limits, reference_result};
use mech_core::{
    ManagedMemoryBudget, ResourceDemand, ValueDataDraft as Data,
    snapshot::{F64Bits, NamedValueDraft},
};
use mech_engine::native_artifact::{
    NativeCollectionPrimitive as Primitive, NativeIterationAction as Action,
    NativeLibraryWorkBudget, lower_native_program_artifact, prepare_native_program_library_call,
};
use mech_gpu::native_artifact::JitProgramArtifact;

fn record(nodes: usize, cache: usize, word: Data) -> Data {
    Data::Record(
        vec![
            NamedValueDraft {
                name: "nodes".into(),
                value: Data::Matrix(vec![Data::U16(0); nodes].into()),
            },
            NamedValueDraft {
                name: "cache".into(),
                value: Data::Matrix(vec![Data::U32(0); cache].into()),
            },
            NamedValueDraft {
                name: "word".into(),
                value: word,
            },
        ]
        .into(),
    )
}

#[test]
fn selected_record_words_keep_exact_types_limits_admission_and_no_allocation() {
    use mech_core::MemoryBudgetDimension as Dimension;
    for (kind, word, expected, width, nodes, cache) in [
        ("u64", Data::U64(u64::MAX), u64::MAX, 8, 262144, 1500513),
        ("bool", Data::Bool(true), 1, 1, 4, 4),
        ("u8", Data::U8(255), 255, 1, 4, 4),
        ("u16", Data::U16(65535), 65535, 2, 4, 4),
        ("u32", Data::U32(u32::MAX), u64::from(u32::MAX), 4, 4, 4),
        ("i64", Data::I64(i64::MIN), i64::MIN as u64, 8, 4, 4),
        (
            "f64",
            Data::F64(F64Bits::from_bits(1u64 << 63)),
            1u64 << 63,
            8,
            4,
            4,
        ),
    ] {
        let source = format!(
            "<frame> := <{{nodes<[u16]:1,{nodes}> cache<[u32]:1,{cache}> word<{kind}>}}>\nread-word(frame<frame>) => (out<{kind}>) :=\n  out := frame.word.\nread-word(frame<frame>)\n"
        );
        for artifact in artifacts(&source).0 {
            let catalog = mech_stdlib::source_catalog();
            let plan = lower_native_program_artifact(&artifact, &catalog).unwrap();
            let (template, selector_value) = plan
                .blocks()
                .iter()
                .find_map(|block| match block.action() {
                    Action::LibraryCall {
                        template, operands, ..
                    } if template.primitive() == Primitive::RecordFieldAccess => {
                        Some((template, operands[1]))
                    }
                    _ => None,
                })
                .unwrap();
            let call = prepare_native_program_library_call(&artifact, template, &catalog).unwrap();
            assert!(call.record_scalar_field_kind().is_some());
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let args = inputs(
                &artifact,
                &[("frame", record(nodes, cache, word.clone()))],
                &memory,
            );
            let constant = plan
                .constants()
                .iter()
                .find(|constant| constant.value() == selector_value)
                .unwrap()
                .constant()
                .clone();
            let mut reserve = memory
                .reserve_capacity(constant.memory_budget_admission_bytes(&memory).unwrap())
                .unwrap();
            let selector = constant.into_memory_budget(&mut reserve).unwrap();
            drop(reserve);
            let work = NativeLibraryWorkBudget::new(1 << 40, 1 << 40, &memory).unwrap();
            let mut policy = limits().library;
            policy.max_compute_work = Some(4096);
            policy.max_comparison_work = Some(4096);
            policy.max_cloned_bytes = Some(0);
            policy.max_output_bytes = Some(width);
            policy.max_output_elements = Some(1);
            policy.max_temporary_bytes = Some(
                args[0].memory_budget_retained_bytes(&memory).unwrap()
                    + selector.memory_budget_retained_bytes(&memory).unwrap()
                    + width,
            );
            policy.max_retained_nodes = Some(
                args[0].memory_budget_retained_nodes(&memory).unwrap()
                    + selector.memory_budget_retained_nodes(&memory).unwrap()
                    + 1,
            );
            if kind == "u64" {
                let mut output = None;
                let ordinary = call.invoke_current(
                    &[Some(args[0].clone()), Some(selector.clone())],
                    &mut output,
                    policy,
                    &work,
                );
                println!("minimal record field through general binding: {ordinary:?}");
                assert!(ordinary.is_err());
                assert!(output.is_none());
            }
            let before = memory.used_bytes();
            let start = work.used();
            let (result, allocations, bytes) = allocation_probe::measured_with_bytes(|| {
                call.record_scalar_field(
                    &args[0],
                    &selector,
                    policy,
                    &work,
                    ResourceDemand::default(),
                )
            });
            assert_eq!(result.unwrap(), expected);
            assert_eq!((allocations, bytes), (0, 0));
            let cost = (work.used().0 - start.0, work.used().1 - start.1);
            policy.max_compute_work = Some(cost.0);
            policy.max_comparison_work = Some(cost.1);
            for dimension in [
                Dimension::ComputeWork,
                Dimension::ComparisonWork,
                Dimension::OutputBytes,
                Dimension::OutputElements,
                Dimension::TemporaryBytes,
                Dimension::RetainedNodes,
            ] {
                let mut denied = policy;
                match dimension {
                    Dimension::ComputeWork => denied.max_compute_work = Some(cost.0 - 1),
                    Dimension::ComparisonWork => denied.max_comparison_work = Some(cost.1 - 1),
                    Dimension::OutputBytes => denied.max_output_bytes = Some(width - 1),
                    Dimension::OutputElements => denied.max_output_elements = Some(0),
                    Dimension::TemporaryBytes => {
                        denied.max_temporary_bytes = Some(policy.max_temporary_bytes.unwrap() - 1)
                    }
                    Dimension::RetainedNodes => {
                        denied.max_retained_nodes = Some(policy.max_retained_nodes.unwrap() - 1)
                    }
                    _ => unreachable!(),
                }
                let error = call
                    .record_scalar_field(
                        &args[0],
                        &selector,
                        denied,
                        &work,
                        ResourceDemand::default(),
                    )
                    .unwrap_err();
                let actual = match error {
                    mech_engine::native_artifact::NativeCollectionInvocationError::Kernel(
                        mech_core::ResidentKernelError::ResourceLimit { dimension, .. },
                    ) => dimension,
                    mech_engine::native_artifact::NativeCollectionInvocationError::Plan(
                        mech_core::MemoryPlanError::TargetLimitExceeded { violation },
                    ) => violation.dimension,
                    error => panic!("{error:?}"),
                };
                assert_eq!(actual, dimension);
                assert_eq!(memory.used_bytes(), before);
                assert_eq!(
                    call.record_scalar_field(
                        &args[0],
                        &selector,
                        policy,
                        &work,
                        ResourceDemand::default()
                    )
                    .unwrap(),
                    expected
                );
            }
            let foreign = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let foreign_work = NativeLibraryWorkBudget::new(1 << 40, 1 << 40, &foreign).unwrap();
            assert!(matches!(
                call.record_scalar_field(
                    &args[0],
                    &selector,
                    limits().library,
                    &foreign_work,
                    ResourceDemand::default()
                ),
                Err(mech_engine::native_artifact::NativeCollectionInvocationError::UnadmittedValue)
            ));
            drop((args, selector, work));
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
fn record_scalar_fields_keep_fresh_visits_assertion_order_and_recovery() {
    let source = "<state> := <{nodes<[u16]:1,4> cache<[u32]:1,4> count<u64> allowed<bool>}>\nchecked(frame<state>, divisor<u64>) => (out<u64>) :=\n  enabled! := frame.allowed\n  out := frame.count + 12u64 / divisor.\nchecked(frame<state>, divisor<u64>)\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(32 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for (count, allowed, divisor) in [(7, true, 3), (91, false, 0), (6, true, 0), (0, true, 2)]
        {
            let Data::Record(mut fields) = record(4, 4, Data::U64(count)) else {
                unreachable!()
            };
            fields
                .iter_mut()
                .find(|field| field.name == "word")
                .unwrap()
                .name = "count".into();
            let mut fields = fields.into_vec();
            fields.push(NamedValueDraft {
                name: "allowed".into(),
                value: Data::Bool(allowed),
            });
            let args = inputs(
                &artifact,
                &[
                    ("frame", Data::Record(fields.into())),
                    ("divisor", Data::U64(divisor)),
                ],
                &memory,
            );
            let aliases = args.clone();
            let prior = output.clone();
            let retained = memory.used_bytes();
            let result = jit.evaluate_into(&args, &mut output, limits(), &memory);
            if !allowed || divisor == 0 {
                assert!(matches!((allowed,&result), (false,Err(mech_gpu::native_artifact::NativeProgramExecutionError::Control(mech_gpu::native_artifact::NativeIterationExecutionError::Integrity{..}))) | (true,Err(mech_gpu::native_artifact::NativeProgramExecutionError::Control(mech_gpu::native_artifact::NativeIterationExecutionError::Arithmetic{..})))),"{result:?}");
                assert!(reference_result(&artifact, &args).is_err());
                assert_eq!(memory.used_bytes(), retained);
                assert!(output.iter().zip(&prior).all(|(value, old)| {
                    value
                        .as_ref()
                        .unwrap()
                        .shares_frozen_storage(old.as_ref().unwrap())
                }));
            } else {
                result.unwrap();
                super::equal(
                    &artifact,
                    &output,
                    &reference_result(&artifact, &args).unwrap(),
                );
            }
            assert!(
                args.iter()
                    .zip(&aliases)
                    .all(|(value, alias)| value.shares_frozen_storage(alias))
            );
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn record_scalar_field_requires_the_maintained_selector_factory() {
    let mut builder = mech_core::FunctionCatalogBuilder::new();
    for (key, entry) in mech_stdlib::source_catalog().resident_factories() {
        if key.module_path.as_ref() != ["access"] || key.operation_name != "column" {
            builder
                .insert_resident_factory_entry(entry.clone())
                .unwrap();
        }
    }
    builder
        .insert_resident_factory(
            ["access"],
            "column",
            mech_core::ImplementationMemoryClass::CanonicalFinalize,
            |_| panic!("replacement must be rejected before binding"),
        )
        .unwrap();
    let replacement = builder.build().unwrap();
    for artifact in artifacts("<state> := <{count<u64>}>\nread-count(frame<state>) => (out<u64>) :=\n  out := frame.count.\nread-count(frame<state>)\n").0 {
        assert!(JitProgramArtifact::compile(&artifact, &replacement).is_err());
    }
}
