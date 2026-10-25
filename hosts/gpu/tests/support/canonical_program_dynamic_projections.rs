//! Live buffer dimensions survive destructuring, tuple reads, and state updates.
use super::{artifacts, equal, inputs, limits, reference};
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_gpu::native_artifact::JitProgramArtifact;

#[test]
fn dynamic_array_tuple_destructuring_preserves_independent_parameter_arenas() {
    let source = "(left, right, count) := (a<[u16]>, b<[u32]>, n<u64>)\n(right, left, count)\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for ((ar, ac), (br, bc)) in [
            ((1, 0), (2, 3)),
            ((2, 3), (3, 2)),
            ((0, 7), (1, 3)),
            ((1, 2), (2, 1)),
            ((0, 0), (1, 0)),
        ] {
            let a = Data::Matrix(vec![Data::U16(65535); (ar * ac) as usize].into());
            let b = Data::Matrix(vec![Data::U32(u32::MAX); (br * bc) as usize].into());
            let mut args = inputs(
                &artifact,
                &[("a", a.clone()), ("b", b.clone()), ("n", Data::U64(19))],
                &memory,
            );
            for (name, rows, columns) in [("a", ar, ac), ("b", br, bc)] {
                let index = artifact
                    .inputs()
                    .iter()
                    .position(|input| {
                        mech_engine::decode_source_input_name(&input.name).unwrap() == name
                    })
                    .unwrap();
                let schema = artifact
                    .schemas()
                    .get(artifact.inputs()[index].schema)
                    .unwrap();
                let shape =
                    mech_core::shape_for_resolved_extents(schema, &[rows, columns]).unwrap();
                let value = mech_core::ValueDraft {
                    schema: artifact.inputs()[index].schema,
                    shape_values: shape.parameter_values().to_vec().into(),
                    data: args[index].canonical_data_draft().unwrap(),
                }
                .finalize(&mech_core::snapshot::SnapshotValidationContext::new(
                    artifact.schemas(),
                ))
                .unwrap();
                let mut claim = memory
                    .reserve_capacity(value.memory_budget_admission_bytes(&memory).unwrap())
                    .unwrap();
                args[index] = value.into_memory_budget(&mut claim).unwrap();
            }
            let ordinary = reference(&artifact, &args);
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            equal(&artifact, &output, &ordinary);
            assert_eq!(
                output
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::Tuple(vec![b, a, Data::U64(19)].into())
            );
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn dynamic_array_constant_tuple_reads_preserve_nested_shapes() {
    let source = "values := (a<[u8]>, (b<[u16]>, n<u64>))\nselected := values.2\n(selected.1, values.1, selected.2)\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for (a, b) in [(0, 7), (4, 2), (1, 0), (0, 0)] {
            let a = Data::Matrix(vec![Data::U8(255); a].into());
            let b = Data::Matrix(vec![Data::U16(65535); b].into());
            let args = inputs(
                &artifact,
                &[("a", a.clone()), ("b", b.clone()), ("n", Data::U64(23))],
                &memory,
            );
            let ordinary = reference(&artifact, &args);
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            equal(&artifact, &output, &ordinary);
            assert_eq!(
                output
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::Tuple(vec![b, a, Data::U64(23)].into())
            );
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[path = "canonical_program_ordinary_recursion.rs"]
mod recursion;

#[path = "canonical_program_dynamic_projection_admission.rs"]
mod admission;
