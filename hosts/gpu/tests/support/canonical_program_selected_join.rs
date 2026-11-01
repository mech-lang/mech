//! Actual selected factories/owners compared with ordinary maintained leaves.
use super::{artifacts, equal, limits, reference};
use mech_core::{
    ManagedMemoryBudget, MemoryBudgetLimits, ResourceDemand, Value, ValueData,
    snapshot::{SnapshotValidationContext, ValueDataDraft as Data, ValueDraft},
};
use mech_engine::{
    ProgramArtifact,
    native_artifact::{
        NativeCollectionInvocationError as Error, NativeIterationAction, NativeLibraryWorkBudget,
        NativePreparedLibraryCall, lower_native_program_artifact,
        prepare_native_program_library_call,
    },
};
use mech_gpu::native_artifact::{JitProgramArtifact, emit_program_object};

fn prepared(artifact: &ProgramArtifact) -> [NativePreparedLibraryCall; 2] {
    let catalog = mech_stdlib::source_catalog();
    let plan = lower_native_program_artifact(artifact, &catalog).unwrap();
    let regions = plan.selected_text_joins();
    assert_eq!(regions.len(), 1);
    [regions[0].selection(), regions[0].consumer()].map(|site| {
        let NativeIterationAction::LibraryCall { template, .. } =
            plan.blocks()[site as usize].action()
        else {
            panic!("actual selected library call");
        };
        prepare_native_program_library_call(artifact, template, &catalog).unwrap()
    })
}

fn owners(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    dimensions: [u64; 2],
    cells: &[&str],
    indices: &[u64],
    separator: &str,
) -> ([Option<Value>; 2], Value) {
    let mut result = [None, None, None];
    for input in artifact.inputs() {
        let name = mech_engine::decode_source_input_name(&input.name).unwrap();
        let (slot, data, extents) = match name.as_str() {
            "input" => (
                0,
                Data::Matrix(cells.iter().map(|x| Data::String((*x).into())).collect()),
                Some(dimensions),
            ),
            "indices" => (
                1,
                Data::Matrix(indices.iter().copied().map(Data::U64).collect()),
                Some([1, indices.len() as u64]),
            ),
            "separator" => (2, Data::String(separator.into()), None),
            _ => panic!("unexpected input {name}"),
        };
        let shape_values = extents.map_or_else(
            || Box::new([]) as Box<[u64]>,
            |extents| {
                mech_core::shape_for_resolved_extents(
                    artifact.schemas().get(input.schema).unwrap(),
                    &extents,
                )
                .unwrap()
                .parameter_values()
                .to_vec()
                .into_boxed_slice()
            },
        );
        let value = ValueDraft {
            schema: input.schema,
            shape_values,
            data,
        }
        .finalize(&SnapshotValidationContext::new(artifact.schemas()))
        .unwrap();
        let mut claim = memory
            .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
            .unwrap();
        result[slot] = Some(value.into_memory_budget(&mut claim).unwrap());
    }
    let [source, selector, separator] = result;
    ([source, selector], separator.unwrap())
}

fn ledger(memory: &ManagedMemoryBudget) -> NativeLibraryWorkBudget {
    NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, memory).unwrap()
}

fn ordinary(
    selected: &[NativePreparedLibraryCall; 2],
    inputs: &[Option<Value>],
    separator: &Value,
    memory: &ManagedMemoryBudget,
) -> Result<String, Error> {
    let work = ledger(memory);
    let mut matrix = None;
    selected[0].invoke_current(inputs, &mut matrix, limits().library, &work)?;
    let mut joined = None;
    selected[1].invoke_current(
        &[matrix, Some(separator.clone())],
        &mut joined,
        limits().library,
        &work,
    )?;
    let ValueData::String(text) = joined.as_ref().unwrap().data() else {
        panic!("ordinary canonical String");
    };
    Ok(text.to_string())
}

fn append(
    consumer: &NativePreparedLibraryCall,
    view: &mech_engine::native_artifact::NativeSelectedStringMatrix,
    separator: &Value,
    work: &NativeLibraryWorkBudget,
) -> String {
    let ValueData::String(sep) = separator.data() else {
        panic!("String separator")
    };
    let bytes = view.payload_bytes() + sep.len() as u64 * view.count().saturating_sub(1) as u64;
    // This isolated leaf fixture pays the full append/selector scan and reserves
    // its actual buffer. It is not a native publication or runtime-region test.
    work.charge(bytes * 16 + view.count() as u64 * 128 + 1024, 0)
        .unwrap();
    let memory = work.memory_budget();
    let claim = memory.reserve_capacity(bytes).unwrap();
    let mut text = String::new();
    text.try_reserve_exact(bytes as usize).unwrap();
    assert_eq!(text.capacity() as u64, bytes);
    let pointer = text.as_ptr();
    consumer
        .selected_matrix_join_append(view, separator, &mut text, &memory)
        .unwrap();
    assert_eq!(text.as_ptr(), pointer, "append never reallocates");
    let result = text.clone(); // Test oracle, not a published Mech-owned result.
    drop(text);
    drop(claim);
    result
}

fn source(kind: &str) -> String {
    format!(
        "+> text\nwords := input<{kind}>\nselect := indices<[u64]>\ntext/join(words[select], separator<string>)\n"
    )
}

fn argument_order(
    artifact: &ProgramArtifact,
    inputs: &[Option<Value>; 2],
    separator: &Value,
) -> Vec<Value> {
    artifact
        .inputs()
        .iter()
        .map(|input| {
            match mech_engine::decode_source_input_name(&input.name)
                .unwrap()
                .as_str()
            {
                "input" => inputs[0].clone().unwrap(),
                "indices" => inputs[1].clone().unwrap(),
                "separator" => separator.clone(),
                name => panic!("unexpected {name}"),
            }
        })
        .collect()
}

#[test]
fn generated_selected_join_keeps_original_and_decoded_root_and_lexical_results() {
    let lexical = "+> text\njoin(words<[string]>, select<[u64]>, sep<string>) => (out<string>) := out := text/join(words[select], sep).\njoin(input<[string]>, indices<[u64]>, separator<string>)\n";
    for source in [source("[string]"), lexical.into()] {
        for artifact in artifacts(&source).0 {
            let catalog = mech_stdlib::source_catalog();
            let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
            assert_eq!(jit.descriptor().plan().selected_text_joins().len(), 1);
            let object = emit_program_object(&artifact, &catalog).unwrap();
            assert_eq!(
                object.descriptor().typed_ir_sha256(),
                jit.descriptor().typed_ir_sha256()
            );
            for indices in [vec![], vec![1], vec![4, 1, 4, 2, 3]] {
                let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
                let (inputs, separator) = owners(
                    &artifact,
                    &memory,
                    [1, 4],
                    &["", "é\0", "\r\n", "🧑‍🚀"],
                    &indices,
                    "|",
                );
                let args = argument_order(&artifact, &inputs, &separator);
                let expected = reference(&artifact, &args);
                let mut outputs = vec![None; artifact.outputs().len()];
                jit.evaluate_into(&args, &mut outputs, limits(), &memory)
                    .unwrap();
                equal(&artifact, &outputs, &expected);
                drop((outputs, args, inputs, separator));
                assert_eq!(memory.used_bytes(), 0);
            }
        }
    }
}

#[test]
fn generated_selected_join_failed_indices_and_finite_denials_preserve_output_and_recover() {
    for artifact in artifacts(&source("[string]")).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
        let (inputs, separator) = owners(
            &artifact,
            &memory,
            [1, 3],
            &["a", "é", "\r\n"],
            &[3, 1, 3],
            "|",
        );
        let args = argument_order(&artifact, &inputs, &separator);
        let mut outputs = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut outputs, limits(), &memory)
            .unwrap();
        let prior = outputs.clone();
        for dimension in 0..6 {
            let mut caps = limits();
            match dimension {
                0 => caps.library.max_compute_work = Some(0),
                1 => caps.library.max_temporary_bytes = Some(0),
                2 => caps.library.max_retained_nodes = Some(0),
                3 => caps.library.max_output_elements = Some(0),
                4 => caps.library.max_output_bytes = Some(0),
                _ => caps.library.max_cloned_bytes = Some(0),
            }
            let before = memory.used_bytes();
            assert!(
                jit.evaluate_into(&args, &mut outputs, caps, &memory)
                    .is_err()
            );
            assert_eq!(memory.used_bytes(), before);
            for (actual, old) in outputs.iter().zip(&prior) {
                assert!(
                    actual
                        .as_ref()
                        .unwrap()
                        .shares_frozen_storage(old.as_ref().unwrap())
                );
            }
        }
        let (invalid, invalid_separator) = owners(
            &artifact,
            &memory,
            [1, 3],
            &["a", "é", "\r\n"],
            &[1, 2, 4],
            "|",
        );
        let invalid_args = argument_order(&artifact, &invalid, &invalid_separator);
        let before = memory.used_bytes();
        let error = jit
            .evaluate_into(&invalid_args, &mut outputs, limits(), &memory)
            .unwrap_err();
        assert!(
            matches!(
                error,
                mech_gpu::native_artifact::NativeProgramExecutionError::Library {
                    error: Error::Kernel(mech_core::ResidentKernelError::InvalidInput),
                    ..
                }
            ),
            "{error:?}"
        );
        assert_eq!(memory.used_bytes(), before);
        for (actual, old) in outputs.iter().zip(&prior) {
            assert!(
                actual
                    .as_ref()
                    .unwrap()
                    .shares_frozen_storage(old.as_ref().unwrap())
            );
        }
        jit.evaluate_into(&args, &mut outputs, limits(), &memory)
            .unwrap();
        equal(&artifact, &outputs, &reference(&artifact, &args));
        drop((
            outputs,
            prior,
            invalid_args,
            invalid,
            invalid_separator,
            args,
            inputs,
            separator,
        ));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn selected_join_projection_matches_ordinary_order_shapes_and_raw_text() {
    for (kind, dimensions, cells) in [
        ("[string]", [1, 0], vec![]),
        ("[string]:1,4", [1, 4], vec!["", "e\u{301}\0", "\r\n", "🧑‍🚀"]),
        ("[string]:4,1", [4, 1], vec!["", "e\u{301}\0", "\r\n", "🧑‍🚀"]),
        ("[string]:2,3", [2, 3], vec!["a", "b", "c", "d", "e", "f"]),
    ] {
        for artifact in artifacts(&source(kind)).0 {
            let selected = prepared(&artifact);
            let n = cells.len() as u64;
            let selections = if n == 0 {
                vec![vec![]]
            } else {
                vec![
                    vec![],
                    (1..=n).collect(),
                    (1..=n).rev().collect(),
                    vec![n, 1, n, 1],
                ]
            };
            for indices in selections {
                for sep in ["", "|é\0", "\r\n"] {
                    let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
                    let work = ledger(&memory);
                    let (inputs, separator) =
                        owners(&artifact, &memory, dimensions, &cells, &indices, sep);
                    let expected = ordinary(&selected, &inputs, &separator, &memory).unwrap();
                    let baseline = memory.used_bytes();
                    let view = selected[0]
                        .selected_string_matrix(
                            &inputs,
                            limits().library,
                            &work,
                            ResourceDemand::default(),
                        )
                        .unwrap();
                    assert_eq!(view.count(), indices.len());
                    assert_eq!(
                        memory.used_bytes(),
                        baseline,
                        "binding scratch fully retires"
                    );
                    drop(inputs); // Only the opaque projection now pins these owners.
                    assert_eq!(memory.used_bytes(), baseline);
                    assert_eq!(append(&selected[1], &view, &separator, &work), expected);
                    assert_eq!(memory.used_bytes(), baseline);
                    drop((view, separator, work));
                    assert_eq!(memory.used_bytes(), 0);
                }
            }
        }
    }
}

#[test]
fn selected_join_projection_preserves_bad_index_errors_and_never_keeps_failed_binding() {
    for artifact in artifacts(&source("[string]")).0 {
        let selected = prepared(&artifact);
        for indices in [vec![0], vec![4], vec![1, 2, 4], vec![u64::MAX]] {
            let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
            let work = ledger(&memory);
            let (inputs, separator) = owners(
                &artifact,
                &memory,
                [1, 3],
                &["a", "é", "\r\n"],
                &indices,
                "|",
            );
            let expected = ordinary(&selected, &inputs, &separator, &memory).unwrap_err();
            let baseline = memory.used_bytes();
            let error = selected[0]
                .selected_string_matrix(&inputs, limits().library, &work, ResourceDemand::default())
                .err()
                .expect("invalid selected index");
            assert_eq!(format!("{error:?}"), format!("{expected:?}"));
            assert_eq!(memory.used_bytes(), baseline);
            assert!(work.used().0 > 0, "failed selector work is not refunded");
            drop((inputs, separator, work));
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
fn selected_join_projection_checks_owners_current_limits_and_append_capacity() {
    for artifact in artifacts(&source("[string]")).0 {
        let selected = prepared(&artifact);
        let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
        let work = ledger(&memory);
        let (inputs, separator) = owners(
            &artifact,
            &memory,
            [1, 3],
            &["a", "é", "\r\n"],
            &[3, 1, 3],
            "|",
        );
        let baseline = memory.used_bytes();
        for dim in 0..4 {
            let mut caps: MemoryBudgetLimits = limits().library;
            match dim {
                0 => caps.max_compute_work = Some(0),
                1 => caps.max_temporary_bytes = Some(0),
                2 => caps.max_retained_nodes = Some(0),
                _ => caps.max_compute_work = None,
            }
            assert!(
                selected[0]
                    .selected_string_matrix(&inputs, caps, &work, ResourceDemand::default())
                    .is_err()
            );
            assert_eq!(memory.used_bytes(), baseline);
        }
        let foreign = ManagedMemoryBudget::new(64 * 1024 * 1024);
        let foreign_work = ledger(&foreign);
        assert!(matches!(
            selected[0].selected_string_matrix(
                &inputs,
                limits().library,
                &foreign_work,
                ResourceDemand::default()
            ),
            Err(Error::UnadmittedValue)
        ));
        drop(foreign_work);
        assert_eq!(foreign.used_bytes(), 0);
        let view = selected[0]
            .selected_string_matrix(&inputs, limits().library, &work, ResourceDemand::default())
            .unwrap();
        for capacity in [0, 1, 64] {
            let mut target = String::with_capacity(capacity);
            let before = (target.as_ptr(), target.capacity());
            assert!(
                selected[1]
                    .selected_matrix_join_append(&view, &separator, &mut target, &memory)
                    .is_err()
            );
            assert!(target.is_empty());
            assert_eq!((target.as_ptr(), target.capacity()), before);
        }
        let mut target = String::from("untouched");
        assert!(
            selected[1]
                .selected_matrix_join_append(&view, &separator, &mut target, &memory)
                .is_err()
        );
        assert_eq!(target, "untouched");
        assert_eq!(
            append(&selected[1], &view, &separator, &work),
            ordinary(&selected, &inputs, &separator, &memory).unwrap()
        );
        drop((view, inputs, separator, work));
        assert_eq!(memory.used_bytes(), 0);
    }
}
