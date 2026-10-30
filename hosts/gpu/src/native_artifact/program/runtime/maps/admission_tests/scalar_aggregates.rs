use super::*;

#[test]
fn ordinary_aggregate_groups_keep_current_words_origins_fallback_and_retry() {
    let source =
        include_str!("../../../../../../tests/fixtures/native/scalar-aggregate-admission.mec");
    for artifact in artifacts(source) {
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let image = jit.descriptor().encode_build_owned_image().unwrap();
        // SAFETY: this exact checked image and generated entry share the live JIT owner.
        let restored =
            unsafe { NativeProgramDescriptor::decode_build_owned_image(&image, &catalog) }.unwrap();
        assert_eq!(restored.encode_build_owned_image().unwrap(), image);
        for descriptor in [jit.descriptor(), &restored] {
            assert!(
                descriptor.metadata().map_admission.groups.iter().any(|g| {
                    g.owner.is_none()
                        && g.sites.iter().any(|&s| {
                            descriptor
                                .metadata()
                                .scalar_aggregates
                                .action(s as usize)
                                .is_some()
                        })
                }),
                "private scalar record instructions must share ordinary native admission"
            );
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let mut outputs = vec![None; artifact.outputs().len()];
            for (word, flag, permit, divisor, signed, real, mode) in [
                (7, true, true, 1, -3, -2.5, Mode::Native),
                (11, false, true, 2, 4, 3.5, Mode::Native),
                (7, true, true, 0, -3, -2.5, Mode::Native),
                (7, false, true, 0, 4, 3.5, Mode::ScalarLimited),
                (u64::MAX, false, false, 0, -3, -2.5, Mode::Native),
                (19, false, true, 3, -8, f64::NAN, Mode::Native),
                (23, true, true, 2, 8, f64::NEG_INFINITY, Mode::Native),
            ] {
                let args = artifact
                    .inputs()
                    .iter()
                    .map(|port| {
                        let name = mech_engine::decode_source_input_name(&port.name).unwrap();
                        let data = match name.as_str() {
                            "word" => ValueDataDraft::U64(word),
                            "flag" => ValueDataDraft::Bool(flag),
                            "permit" => ValueDataDraft::Bool(permit),
                            "divisor" => ValueDataDraft::U64(divisor),
                            "signed" => ValueDataDraft::I64(signed),
                            "real" => ValueDataDraft::F64(real),
                            _ => panic!("{name}"),
                        };
                        let value = ValueDraft {
                            schema: port.schema,
                            shape_values: Box::new([]),
                            data,
                        }
                        .finalize(&SnapshotValidationContext::new(artifact.schemas()))
                        .unwrap();
                        let mut claim = memory
                            .reserve_capacity(value.memory_budget_admission_bytes(&memory).unwrap())
                            .unwrap();
                        value.into_memory_budget(&mut claim).unwrap()
                    })
                    .collect::<Vec<_>>();
                let aliases = args.clone();
                let prior = outputs.clone();
                let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let (result, observed) = run(&jit, descriptor, mode, &args, &mut outputs, &memory);
                assert!(observed.errors.is_empty(), "{observed:?}");
                assert_eq!(observed.scalar_interiors, 0, "{observed:?}");
                if permit && divisor != 0 {
                    result.unwrap();
                    let selected = if flag {
                        (word + 1) * 3 + 5
                    } else {
                        (word + 1) * 7 + 11
                    };
                    let expected = (selected
                        + if flag { 17 } else { 13 }
                        + if signed < 0 { 2 } else { 3 }
                        + if real < 0.0 { 5 } else { 7 })
                        / divisor
                        + 17;
                    assert!(
                        matches!(outputs[0].as_ref().unwrap().data(), ValueData::U64(n) if *n == expected)
                    );
                    assert!(observed.aggregate_groups > 0, "{observed:?}");
                } else {
                    let error = result.unwrap_err();
                    if !permit {
                        assert!(
                            matches!(
                                error,
                                NativeProgramExecutionError::Control(
                                    NativeIterationExecutionError::Integrity { .. }
                                )
                            ),
                            "{error:?}"
                        );
                    } else {
                        let NativeProgramExecutionError::Control(
                            NativeIterationExecutionError::Arithmetic { origin },
                        ) = error
                        else {
                            panic!("expected original arithmetic: {error:?} {observed:?}");
                        };
                        let (failed, previous) = observed.failure.unwrap();
                        assert_eq!(origin, descriptor.metadata().visits[failed as usize].origin);
                        if mode == Mode::ScalarLimited {
                            assert!(
                                observed.scalar_limited && observed.scalar_fallbacks > 0,
                                "{observed:?}"
                            );
                            assert_eq!(previous, Some(failed as u32));
                        } else {
                            let group = descriptor
                                .metadata()
                                .map_admission
                                .group(previous.unwrap() as usize)
                                .unwrap();
                            assert_ne!(previous, Some(failed as u32));
                            assert!(group.sites.contains(&(failed as u32)));
                        }
                    }
                    assert!(outputs.iter().zip(&prior).all(|(a, b)| {
                        a.as_ref()
                            .unwrap()
                            .shares_frozen_storage(b.as_ref().unwrap())
                    }));
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        retained
                    );
                }
                assert!(
                    args.iter()
                        .zip(&aliases)
                        .all(|(a, b)| a.shares_frozen_storage(b))
                );
            }
            drop(outputs);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}
