//! First-change callbacks must preserve current SSA and the ordinary failure boundary.
use super::*;

const DELAYED: &str = r#"walk(a<[u32]>, b<[u32]>, i<u64>, end<u64>, total<u64>, checksum<u64>, flip<bool>) => (out<u64>) :=
  old-a := a[1u64]
  old-b := b[1u64]
  wide-a<u64> := old-a
  wide-b<u64> := old-b
  next-total := total + 13u64 + i
  next-checksum := checksum + wide-a + 2u64 * wide-b
  next-flip := flip == false
  value-a := ((i < 3u64)? | true => old-a | false => old-a + 1u32)
  value-b := ((i < 5u64)? | true => old-b | false => old-b + 2u32)
  ~next-a := a
  next-a[1u64] = value-a
  ~next-b := b
  next-b[1u64] = value-b
  current-a<u64> := next-a[1u64]
  current-b<u64> := next-b[1u64]
  flag := (next-flip? | true => 7u64 | false => 3u64)
  answer := next-total * 100000u64 + next-checksum * 100u64 + current-a * 10u64 + current-b + flag
  out := ((i + 1u64 < end)? | true => walk(next-a, next-b, i + 1u64, end, next-total, next-checksum, next-flip) | false => answer).
walk(first<[u32]>, second<[u32]>, 0u64, 8u64, seed<u64>, checksum<u64>, flip<bool>)
"#;

fn arguments(
    artifact: &mech_engine::ProgramArtifact,
    a: u32,
    b: u32,
    seed: u64,
    checksum: u64,
    flip: bool,
    memory: &ManagedMemoryBudget,
) -> Vec<Value> {
    artifact
        .inputs()
        .iter()
        .map(|input| {
            let name = mech_engine::decode_source_input_name(&input.name).unwrap();
            let (data, shape_values): (ValueDataDraft, Box<[u64]>) = match name.as_str() {
                "first" | "second" => (
                    ValueDataDraft::Matrix(
                        vec![ValueDataDraft::U32(if name == "first" { a } else { b })].into(),
                    ),
                    mech_core::shape_for_resolved_extents(
                        artifact.schemas().get(input.schema).unwrap(),
                        &[1, 1],
                    )
                    .unwrap()
                    .parameter_values()
                    .to_vec()
                    .into_boxed_slice(),
                ),
                "seed" => (ValueDataDraft::U64(seed), Box::new([]) as Box<[u64]>),
                "checksum" => (ValueDataDraft::U64(checksum), Box::new([])),
                "flip" => (ValueDataDraft::Bool(flip), Box::new([])),
                _ => panic!("unexpected input {name}"),
            };
            let value = ValueDraft {
                schema: input.schema,
                shape_values,
                data,
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap();
            let mut reservation = memory
                .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
                .unwrap();
            value.into_memory_budget(&mut reservation).unwrap()
        })
        .collect()
}

fn model(
    mut a: u32,
    mut b: u32,
    mut total: u64,
    mut checksum: u64,
    mut flip: bool,
) -> (u64, Vec<Vec<u64>>) {
    let mut transitions = Vec::new();
    for i in 0..8u64 {
        if i == 3 || i == 5 {
            transitions.push(vec![i, 8, total, checksum, u64::from(flip)]);
        }
        total += 13 + i;
        checksum += u64::from(a) + 2 * u64::from(b);
        flip = !flip;
        if i >= 3 {
            a += 1;
        }
        if i >= 5 {
            b += 2;
        }
    }
    (
        total * 100000
            + checksum * 100
            + u64::from(a) * 10
            + u64::from(b)
            + if flip { 7 } else { 3 },
        transitions,
    )
}

#[test]
fn delayed_numeric_first_changes_preserve_current_scalars_two_arrays_and_retry() {
    let original = artifact_source(DELAYED);
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    let mut work = Vec::new();
    for artifact in [original, decoded] {
        let jit = crate::native_artifact::JitProgramArtifact::compile(
            &artifact,
            &mech_stdlib::source_catalog(),
        )
        .unwrap();
        assert!(
            jit.descriptor()
                .metadata()
                .closed_tails
                .regions
                .iter()
                .any(|region| region.numeric.as_ref().is_some_and(|proof| proof
                    .arrays
                    .iter()
                    .filter(|array| array.writable)
                    .count()
                    == 2))
        );
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut outputs = vec![None; artifact.outputs().len()];
        let mut current_work = Vec::new();
        // Failure kinds: 0 succeeds, 1 denies the real helper's private lane,
        // 2 denies the original first copy while keeping actual input aliases.
        for (a, b, seed, checksum, flip, alias, failure) in [
            (5, 11, 17, 23, false, false, 0),
            (5, 11, 17, 23, false, false, 1),
            (19, 19, 31, 7, true, true, 0),
            (41, 9, 3, 5, false, false, 2),
            (7, 29, 11, 37, true, false, 0),
        ] {
            let mut args = arguments(&artifact, a, b, seed, checksum, flip, &memory);
            if alias {
                let port = |name: &str| {
                    artifact
                        .inputs()
                        .iter()
                        .position(|p| {
                            mech_engine::decode_source_input_name(&p.name).unwrap() == name
                        })
                        .unwrap()
                };
                let first = port("first");
                let second = port("second");
                assert_eq!(args[first].schema(), args[second].schema());
                args[second] = args[first].clone();
            }
            let aliases = args.clone();
            let values = args
                .iter()
                .map(|v| v.canonical_data_draft().unwrap())
                .collect::<Vec<_>>();
            let prior = outputs.clone();
            let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
            let mut caps = limits();
            if failure == 2 {
                caps.library.max_cloned_bytes = Some(0);
            }
            let (run, rows) = execute_observed_numeric_first_change(
                &jit,
                &args,
                &mut outputs,
                caps,
                &memory,
                failure == 1,
            );
            let (answer, expected_rows) = model(a, b, seed, checksum, flip);
            assert_eq!(rows[0].scalars, expected_rows[0], "{rows:?}");
            assert_eq!(rows[0].incoming, u64::from(a) + 1);
            if failure == 0 {
                current_work.push(run.unwrap().work);
                assert_eq!(
                    rows.len(),
                    2,
                    "each array changes once after distinct no-op prefixes"
                );
                assert_eq!(rows[1].scalars, expected_rows[1], "{rows:?}");
                assert_eq!(rows[1].incoming, u64::from(b) + 2);
                assert!(rows.iter().all(|row| row.status == STATUS_OK));
                assert_eq!(
                    outputs
                        .last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    ValueDataDraft::U64(answer)
                );
            } else {
                let error = format!("{:?}", run.unwrap_err());
                if failure == 1 {
                    assert!(
                        error.contains("native reconciliation word outside admitted lane"),
                        "{error}"
                    );
                } else {
                    assert!(error.contains("ClonedBytes"), "{error}");
                }
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0].status, STATUS_ADMISSION);
                assert_eq!(
                    retained,
                    (memory.used_bytes(), memory.retained_snapshot_nodes())
                );
                assert!(outputs.iter().zip(&prior).all(|(a, b)| {
                    a.as_ref()
                        .unwrap()
                        .shares_frozen_storage(b.as_ref().unwrap())
                }));
            }
            assert_eq!(
                args.iter()
                    .map(|v| v.canonical_data_draft().unwrap())
                    .collect::<Vec<_>>(),
                values
            );
            assert!(
                args.iter()
                    .zip(&aliases)
                    .all(|(a, b)| a.shares_frozen_storage(b))
            );
        }
        work.push(current_work);
        drop(outputs);
        assert_eq!(memory.used_bytes(), 0);
        assert_eq!(memory.retained_snapshot_nodes(), 0);
    }
    assert_eq!(work[0], work[1], "original and decoded work agreement");
}
