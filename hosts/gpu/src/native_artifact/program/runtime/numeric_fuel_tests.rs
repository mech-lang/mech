//! Observe a genuine finite-fuel exit before ordinary execution rejects its
//! exhausted budget. No test fuel override or production accounting change.
use super::*;

#[test]
fn numeric_fuel_exit_synchronizes_current_carrier_then_cleans_up_and_retries() {
    let source = "walk(words<[u32]>, left<u64>, total<u64>, expected<u64>) => (out<bool>) :=\n  more := left > 0u64\n  value<u64> := (more? | true => words[1u64] | false => 0u32)\n  out := (more? | true => walk(words, left - 1u64, total + value, expected) | false => total == expected).\nwalk(words<[u32]>, steps<u64>, 0u64, expected<u64>)\n";
    let original = artifact_source(source);
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    for artifact in [original, decoded] {
        let catalog = mech_stdlib::source_catalog();
        let jit = crate::native_artifact::JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let region = jit
            .descriptor()
            .metadata()
            .closed_tails
            .regions
            .iter()
            .find(|region| region.numeric.is_some())
            .unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let args = artifact
            .inputs()
            .iter()
            .map(|input| {
                let name = mech_engine::decode_source_input_name(&input.name).unwrap();
                let (data, shape_values): (_, Box<[u64]>) = match name.as_str() {
                    "words" => (
                        ValueDataDraft::Matrix(vec![ValueDataDraft::U32(3)].into()),
                        mech_core::shape_for_resolved_extents(
                            artifact.schemas().get(input.schema).unwrap(),
                            &[1, 1],
                        )
                        .unwrap()
                        .parameter_values()
                        .to_vec()
                        .into_boxed_slice(),
                    ),
                    "steps" => (ValueDataDraft::U64(64), Box::new([])),
                    "expected" => (ValueDataDraft::U64(192), Box::new([])),
                    _ => panic!("unexpected input"),
                };
                let value = ValueDraft {
                    schema: input.schema,
                    shape_values,
                    data,
                }
                .finalize(&SnapshotValidationContext::new(artifact.schemas()))
                .unwrap();
                let mut reservation = memory
                    .reserve_capacity(value.memory_budget_admission_bytes(&memory).unwrap())
                    .unwrap();
                value.into_memory_budget(&mut reservation).unwrap()
            })
            .collect::<Vec<_>>();
        let aliases = args.clone();
        let mut outputs = vec![None];
        jit.evaluate_into(&args, &mut outputs, limits(), &memory)
            .unwrap();
        let full = jit
            .evaluate_into(&args, &mut outputs, limits(), &memory)
            .unwrap();
        let prior = outputs.clone();
        let before = (memory.used_bytes(), memory.retained_snapshot_nodes());
        let mut observed = 0;
        for withheld_bodies in [1, 2, 4, 8, 16, 32, 48, 64] {
            let mut denied = limits();
            denied.execution.max_work = full.work - withheld_bodies * region.compute;
            let (result, carriers) =
                execute_observed_numeric_fuel(&jit, &args, &mut outputs, denied, &memory);
            let error = format!("{:?}", result.unwrap_err());
            assert!(error.contains("WorkLimit"), "{error}");
            for row in &carriers {
                assert_eq!(row.len(), 6);
                assert_eq!(row[0] % row[1], 0);
                let traversals = row[0] / row[1];
                assert!((1..=64).contains(&traversals));
                assert_eq!(
                    &row[2..],
                    &[1, 64 - traversals, 3 * traversals, 192],
                    "current scalar fields must match actual prepaid native traversals, not entry state"
                );
                observed += 1;
            }
            assert_eq!(
                before,
                (memory.used_bytes(), memory.retained_snapshot_nodes())
            );
            assert!(
                outputs[0]
                    .as_ref()
                    .unwrap()
                    .shares_frozen_storage(prior[0].as_ref().unwrap())
            );
            assert!(
                args.iter()
                    .zip(&aliases)
                    .all(|(a, b)| a.shares_frozen_storage(b))
            );
            println!(
                "{}",
                serde_json::json!({"witness":"actual-numeric-fuel-exit", "full_work":full.work, "body_cost":region.compute, "denied_limit":denied.execution.max_work, "carriers":carriers, "error":error})
            );
        }
        assert!(
            observed > 0,
            "must reach real finite-fuel exits after a nonempty native prefix"
        );
        jit.evaluate_into(&args, &mut outputs, limits(), &memory)
            .unwrap();
        assert_eq!(
            outputs[0].as_ref().unwrap().canonical_data_draft().unwrap(),
            ValueDataDraft::Bool(true)
        );
        drop((args, aliases, outputs, prior));
        assert_eq!(memory.used_bytes(), 0);
        assert_eq!(memory.retained_snapshot_nodes(), 0);
    }
}

#[test]
fn numeric_write_fuel_exit_freezes_current_cells_without_renewing_admission() {
    let source = "put(xs<[u32]>, at<u64>, word<u32>) => (out<[u32]>) :=\n  ~next := xs\n  next[at] = word\n  out := next.\nwalk(xs<[u32]>, i<u64>, end<u64>) => (out<[u32]>) :=\n  more := i < end\n  out := (more? | true => walk(put(xs, i + 1u64, xs[i + 1u64] + 1u32), i + 1u64, end) | false => xs).\nwalk(words<[u32]>, 0u64, 64u64)\n";
    let original = artifact_source(source);
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    for artifact in [original, decoded] {
        let jit = crate::native_artifact::JitProgramArtifact::compile(
            &artifact,
            &mech_stdlib::source_catalog(),
        )
        .unwrap();
        let region = jit
            .descriptor()
            .metadata()
            .closed_tails
            .regions
            .iter()
            .find(|r| r.numeric.as_ref().is_some_and(|p| p.writes()))
            .unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        assert_eq!(artifact.inputs().len(), 1);
        let schema = artifact.inputs()[0].schema;
        let shape = mech_core::shape_for_resolved_extents(
            artifact.schemas().get(schema).unwrap(),
            &[1, 64],
        )
        .unwrap();
        let value = ValueDraft {
            schema,
            shape_values: shape.parameter_values().to_vec().into_boxed_slice(),
            data: ValueDataDraft::Matrix(vec![ValueDataDraft::U32(0); 64].into()),
        }
        .finalize(&SnapshotValidationContext::new(artifact.schemas()))
        .unwrap();
        let mut admission = memory
            .reserve_capacity(value.memory_budget_admission_bytes(&memory).unwrap())
            .unwrap();
        let args = vec![value.into_memory_budget(&mut admission).unwrap()];
        drop(admission);
        let mut outputs = vec![None];
        let full = jit
            .evaluate_into(&args, &mut outputs, limits(), &memory)
            .unwrap();
        let prior = outputs.clone();
        let before = (memory.used_bytes(), memory.retained_snapshot_nodes());
        let mut observed = 0;
        for withheld in [1, 2, 4, 8, 16, 32, 48, 64] {
            let mut denied = limits();
            denied.execution.max_work = full.work - withheld * region.compute;
            let (run, rows) =
                execute_observed_numeric_fuel(&jit, &args, &mut outputs, denied, &memory);
            assert!(format!("{:?}", run.unwrap_err()).contains("WorkLimit"));
            for row in &rows {
                assert_eq!(row[0] % row[1], 0);
                assert_eq!(row[1], region.compute);
                assert_eq!(row[2], 1);
                assert_eq!(row.len(), 70);
                let completed = row[3] as usize;
                assert!((1..=64).contains(&completed));
                assert_eq!(
                    completed as u64,
                    1 + row[0] / row[1],
                    "first changed write settled the first paid traversal"
                );
                assert_eq!(&row[4..6], &[64, 64]);
                assert_eq!(&row[6..6 + completed], &vec![1; completed]);
                assert_eq!(&row[6 + completed..], &vec![0; 64 - completed]);
                observed += 1;
            }
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                before
            );
            assert!(
                outputs[0]
                    .as_ref()
                    .unwrap()
                    .shares_frozen_storage(prior[0].as_ref().unwrap())
            );
            assert_eq!(
                args[0].canonical_data_draft().unwrap(),
                ValueDataDraft::Matrix(vec![ValueDataDraft::U32(0); 64].into())
            );
            println!(
                "{}",
                serde_json::json!({"witness":"actual-write-fuel-exit","full_work":full.work,"body_cost":region.compute,"denied_limit":denied.execution.max_work,"carriers_and_cells":rows})
            );
        }
        assert!(observed > 0);
        jit.evaluate_into(&args, &mut outputs, limits(), &memory)
            .unwrap();
        assert_eq!(
            outputs[0].as_ref().unwrap().canonical_data_draft().unwrap(),
            ValueDataDraft::Matrix(vec![ValueDataDraft::U32(1); 64].into())
        );
        drop((args, outputs, prior));
        assert_eq!(memory.used_bytes(), 0);
        assert_eq!(memory.retained_snapshot_nodes(), 0);
    }
}
