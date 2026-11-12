//! Compare the compiled cache with the previous runtime scan, including exact
//! narrow failure prefixes, finite required/limit behavior and restored images.
use super::*;
use crate::native_artifact::program::prepare::utf8_reads::{
    derive_replay_facts, reference_replay_facts,
};
use crate::native_artifact::program::runtime::collections::utf8_read::with_reference_replay_for_test;

fn assert_exact_cached_facts(descriptor: &crate::native_artifact::NativeProgramDescriptor) {
    let metadata = descriptor.metadata();
    assert!(!metadata.utf8_reads.regions.is_empty());
    for region in &metadata.utf8_reads.regions {
        if region.conversion_site.is_some() {
            let expected = reference_replay_facts(metadata, region);
            assert!(
                expected.is_some(),
                "working selected fixture must have a replay bound"
            );
            assert_eq!(region.replay_facts(), expected);
        } else {
            assert_eq!(region.replay_facts(), None);
        }
    }
}

#[test]
fn typed_replay_facts_are_exact_on_original_decoded_and_both_image_paths() {
    use crate::native_artifact::{NativeProgramDescriptor, emit_program_object};
    for width in [1, 2, 4] {
        for classified in [false, true] {
            for record in [false, true] {
                for artifact in artifacts(&converted_source(width, classified, record)) {
                    let catalog = mech_stdlib::source_catalog();
                    let object = emit_program_object(&artifact, &catalog).unwrap();
                    assert_exact_cached_facts(object.descriptor());
                    let image = object.native_metadata();
                    let independent = unsafe {
                        NativeProgramDescriptor::decode_build_owned_image(image, &catalog)
                    }
                    .unwrap();
                    let bound = unsafe {
                        NativeProgramDescriptor::decode_object_bound_image(
                            image,
                            &catalog,
                            object.identity().bytes(),
                            object.native_metadata_binding().bytes(),
                        )
                    }
                    .unwrap();
                    for descriptor in [&independent, &bound] {
                        assert_exact_cached_facts(descriptor);
                        let left = &object.descriptor().metadata().utf8_reads.regions;
                        let right = &descriptor.metadata().utf8_reads.regions;
                        assert_eq!(left.len(), right.len());
                        for (a, b) in left.iter().zip(right) {
                            assert_eq!(a.replay_facts(), b.replay_facts());
                        }
                    }
                    let json = object.descriptor().encode_build_owned_image().unwrap();
                    assert!(!String::from_utf8(json).unwrap().contains("replay_facts"));
                }
            }
        }
    }
}

#[test]
fn typed_replay_facts_decline_unsupported_and_overflow_without_execution_authority() {
    let mode = NativeProgramLimitMode::NoLimits;
    for artifact in artifacts(&converted_source(1, true, false)) {
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        assert_exact_cached_facts(jit.descriptor());
        for corruption in 0..5 {
            // Metadata-only negative cases. No mutated descriptor executes.
            let mut descriptor =
                super::super::super::tests::restore_admitted_helper_descriptor(&jit);
            let region = descriptor.metadata.utf8_reads.regions[0].clone();
            match corruption {
                0 => descriptor.metadata.scalar_bytes = u64::MAX,
                1 => descriptor.metadata.observation_schema_bytes = u64::MAX,
                2 => descriptor.metadata.visits[region.header as usize].work = u64::MAX,
                3 => {
                    descriptor.metadata.calls[region.read_site as usize].take();
                }
                4 => descriptor.metadata.utf8_reads.regions[0].read_site = u32::MAX,
                _ => unreachable!(),
            }
            derive_replay_facts(&mut descriptor.metadata).unwrap();
            for current in &descriptor.metadata.utf8_reads.regions {
                assert_eq!(
                    current.replay_facts(),
                    reference_replay_facts(descriptor.metadata(), current)
                );
                assert_eq!(current.replay_facts(), None);
            }
        }
    }
}

#[test]
fn typed_replay_cache_and_old_scan_keep_exact_failed_attempt_work_and_retry() {
    let mode = NativeProgramLimitMode::NoLimits;
    for record in [false, true] {
        for artifact in artifacts(&converted_source(1, true, record)) {
            let jit = JitProgramArtifact::compile_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                mode.compilation(),
            )
            .unwrap();
            for restored in [false, true] {
                let memory = mode.memory_budget();
                let good = arguments(
                    &artifact,
                    &memory,
                    &[("ok", 1), ("bad", 255)],
                    &["ok"],
                    1,
                    1,
                );
                let mut prior = vec![None; artifact.outputs().len()];
                run(
                    &jit,
                    &good,
                    &mut prior,
                    mode.execution(),
                    &memory,
                    restored,
                    Flags::default(),
                )
                .0
                .unwrap();
                for bad_at in 0..4 {
                    let mut keys = vec!["ok"; 4];
                    keys[bad_at] = "bad";
                    let args =
                        arguments(&artifact, &memory, &[("ok", 1), ("bad", 256)], &keys, 1, 4);
                    let held = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let trial = || {
                        let mut output = prior.clone();
                        let (result, counts) = run(
                            &jit,
                            &args,
                            &mut output,
                            mode.execution(),
                            &memory,
                            restored,
                            Flags::default(),
                        );
                        let error = format!("{:?}", result.unwrap_err());
                        assert!(error.contains("Arithmetic"), "{error}");
                        assert_prior(&output, &prior);
                        drop(output);
                        assert_eq!(
                            (memory.used_bytes(), memory.retained_snapshot_nodes()),
                            held
                        );
                        let helper_work = counts
                            .helper_work
                            .iter()
                            .map(|entry| {
                                (
                                    entry.site,
                                    entry.op,
                                    entry.status,
                                    entry.before,
                                    entry.after,
                                )
                            })
                            .collect::<Vec<_>>();
                        (
                            error,
                            counts.selected,
                            counts.complete,
                            counts.attempt_progress,
                            helper_work,
                        )
                    };
                    let cached = trial();
                    let old_scan = with_reference_replay_for_test(trial);
                    assert_eq!(
                        cached, old_scan,
                        "exact failed attempt and error prefix at row{bad_at}"
                    );
                    assert_eq!((cached.1, cached.2), (1, 0));
                    drop(args);
                }
                with_reference_replay_for_test(|| {
                    run(
                        &jit,
                        &good,
                        &mut prior,
                        mode.execution(),
                        &memory,
                        restored,
                        Flags::default(),
                    )
                })
                .0
                .unwrap();
                run(
                    &jit,
                    &good,
                    &mut prior,
                    mode.execution(),
                    &memory,
                    restored,
                    Flags::default(),
                )
                .0
                .unwrap();
                drop((good, prior));
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
}

#[test]
fn old_scan_also_passes_every_existing_narrow_finite_error_boundary_and_retry() {
    // Reuse the maintained exhaustive contract, including every cast width,
    // failure position, original/decoded/restored paths and exact global caps.
    // No assertion or relative-call-offset rule in that contract is relaxed.
    with_reference_replay_for_test(|| {
        super::utf8_checked_yield_errors_keep_every_source_position_tight_required_limits_and_retry(
        );
    });
}
