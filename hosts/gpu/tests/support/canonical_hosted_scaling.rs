//! Scale gates for the complete source, not a replacement by leaf timings.
use super::*;

#[test]
#[ignore = "explicit current-source selected-site metadata diagnostic; no program execution or timings"]
fn current_complete_source_selected_library_inventory() {
    let (variants, catalog) = artifacts(FULL);
    for (variant, artifact) in variants.into_iter().enumerate() {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let plan = jit.descriptor().plan();
        eprintln!(
            "SELECTED_SOURCE variant={variant} typed_ir={:?}",
            jit.descriptor().typed_ir_sha256()
        );
        for (site, block) in plan.blocks().iter().enumerate() {
            if let mech_engine::native_artifact::NativeIterationAction::LibraryCall {
                template,
                operands,
                output,
            } = block.action()
            {
                eprintln!(
                    "SELECTED_LIBRARY variant={variant} site={site} origin={:?} primitive={:?} operands={operands:?} output={output:?} iteration={:?}",
                    template.origin(),
                    template.primitive(),
                    block.iteration()
                );
            }
        }
        for (id, collection) in plan.collections().iter().enumerate() {
            let strategy = match collection.result() {
                mech_engine::native_artifact::NativeCollectionResult::Materialized => {
                    "materialized"
                }
                mech_engine::native_artifact::NativeCollectionResult::TextJoin(_) => "text-join",
                mech_engine::native_artifact::NativeCollectionResult::Histogram(_) => "histogram",
                mech_engine::native_artifact::NativeCollectionResult::GuardedHistogramQuery(_) => {
                    "guarded-histogram-query"
                }
            };
            eprintln!(
                "SELECTED_COLLECTION variant={variant} collection={id} strategy={strategy} output={:?} begin={} push={} finish={}",
                collection.output(),
                collection.begin(),
                collection.push(),
                collection.finish()
            );
        }
    }
}

#[test]
fn complete_secured_knucleotide_handles_hundred_and_four_hundred_bases_without_lifting_limits() {
    let (variants, catalog) = artifacts(FULL);
    for artifact in variants {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let joined = jit
            .descriptor()
            .plan()
            .collections()
            .iter()
            .filter(|collection| {
                matches!(
                    collection.result(),
                    mech_engine::native_artifact::NativeCollectionResult::TextJoin(_)
                )
            })
            .count();
        assert!(
            joined >= 4,
            "the complete original/decoded source must actually fuse uppercase, formatting and outer joins, got {joined}"
        );
        assert_eq!(
            jit.descriptor()
                .plan()
                .collections()
                .iter()
                .filter(|collection| matches!(
                    collection.result(),
                    mech_engine::native_artifact::NativeCollectionResult::GuardedHistogramQuery(_)
                ))
                .count(),
            1,
            "the original five-query body, including18mer, must actually select demand fusion"
        );
        assert_eq!(
            jit.descriptor()
                .plan()
                .collections()
                .iter()
                .filter(|collection| matches!(
                    collection.result(),
                    mech_engine::native_artifact::NativeCollectionResult::Histogram(_)
                ))
                .count(),
            1,
            "frequency enumeration still needs its genuine full Map"
        );
        for repeats in [25usize, 100, 1] {
            let input = format!(
                ">ONE\nAAAA\n>THREE timing\n{}\n>FOUR\nCC\n",
                "GGTA".repeat(repeats)
            );
            let windows = 4 * repeats - 1;
            let common = 100.0 * repeats as f64 / windows as f64;
            let other = if repeats > 1 {
                format!("AG {:.3}\n", 100.0 * (repeats - 1) as f64 / windows as f64)
            } else {
                String::new()
            };
            let expected = format!(
                "G 50.000\nA 25.000\nT 25.000\n\nGG {common:.3}\nGT {common:.3}\nTA {common:.3}\n{other}\n{repeats}\tGGT\n{repeats}\tGGTA\n0\tGGTATT\n0\tGGTATTTTAATT\n0\tGGTATTTTAATTTATAGT\n"
            );
            let (mut runtime, trace) = runtime(input.as_bytes(), true);
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let report = execute(&mut runtime, &artifact, &jit, limits(), &memory)
                .unwrap_or_else(|error| panic!("{} bases: {error:?}", repeats * 4));
            assert!(report.work <= limits().execution.max_work);
            no_resident(&runtime);
            let snapshot = trace.lock().unwrap().clone();
            assert_eq!(snapshot.output, expected);
            assert_eq!(
                (
                    snapshot.reads,
                    snapshot.acquisitions,
                    snapshot.prepares,
                    snapshot.deliveries,
                    snapshot.flushes
                ),
                (1, 1, 1, 1, 1)
            );
            assert_eq!(memory.used_bytes(), 0);
            runtime.shutdown().unwrap();
        }
    }
}
