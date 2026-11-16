use crate::native_artifact::{
    JitProgramArtifact, LinkedProgramArtifact, NativeLlvmToolchain, NativeProgramDescriptor,
    NativeProgramLimitMode, NativeProgramModule, emit_program_llvm_object_with_limits,
};
use mech_core::{
    ManagedMemoryBudget, Value, ValueDataDraft as Data,
    snapshot::{F64Bits, SnapshotValidationContext, ValueDraft},
};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};
use std::{
    ffi::{CString, c_char, c_void},
    path::Path,
    process::Command,
};
unsafe extern "C" {
    fn dlopen(path: *const c_char, flags: i32) -> *mut c_void;
    fn dlsym(handle: *mut c_void, name: *const c_char) -> *mut c_void;
    fn dlerror() -> *const c_char;
}

fn compile(source: &str) -> ProgramArtifact {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x4c4c564d), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
    CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            mech_stdlib::source_catalog(),
        )
        .unwrap()
        .compile_artifact()
        .unwrap()
}

fn compiler_thresholds(
    artifact: &ProgramArtifact,
    object: &crate::native_artifact::NativeProgramObject,
    toolchain: &NativeLlvmToolchain,
) {
    use crate::native_artifact::NativeCompileError;
    let catalog = mech_stdlib::source_catalog();
    let exact = crate::native_artifact::NativeCompileLimits {
        max_code_bytes: object.descriptor().code_bytes(),
        max_stack_bytes: object.descriptor().stack_bytes(),
        ..NativeProgramLimitMode::NoLimits.compilation()
    };
    assert!(exact.max_code_bytes > 0 && exact.max_stack_bytes > 0);
    let admitted =
        emit_program_llvm_object_with_limits(artifact, &catalog, exact, toolchain).unwrap();
    assert_eq!(admitted.bytes(), object.bytes());
    for (dimension, required, limits) in [
        (
            "Iteration code",
            exact.max_code_bytes,
            crate::native_artifact::NativeCompileLimits {
                max_code_bytes: exact.max_code_bytes - 1,
                ..exact
            },
        ),
        (
            "Iteration stack",
            exact.max_stack_bytes,
            crate::native_artifact::NativeCompileLimits {
                max_stack_bytes: exact.max_stack_bytes - 1,
                ..exact
            },
        ),
    ] {
        match emit_program_llvm_object_with_limits(artifact, &catalog, limits, toolchain) {
            Err(NativeCompileError::Limit {
                dimension: actual,
                required: actual_required,
                limit,
            }) => assert_eq!(
                (actual, actual_required, limit),
                (dimension, required, required - 1)
            ),
            _ => panic!("LLVM must enforce its actual {dimension} threshold"),
        }
    }
    for limits in [
        crate::native_artifact::NativeCompileLimits {
            max_definition_table_bytes: 0,
            ..exact
        },
        crate::native_artifact::NativeCompileLimits {
            max_nodes: 0,
            ..exact
        },
        crate::native_artifact::NativeCompileLimits {
            max_inputs: 0,
            ..exact
        },
        crate::native_artifact::NativeCompileLimits {
            max_outputs: 0,
            ..exact
        },
    ] {
        assert!(
            emit_program_llvm_object_with_limits(artifact, &catalog, limits, toolchain).is_err()
        );
    }
}
fn inputs(
    artifact: &ProgramArtifact,
    named: &[(&str, Data)],
    memory: &ManagedMemoryBudget,
) -> Vec<Value> {
    artifact
        .inputs()
        .iter()
        .map(|port| {
            let name = mech_engine::decode_source_input_name(&port.name).unwrap();
            let data = named
                .iter()
                .find(|(n, _)| *n == name)
                .unwrap_or_else(|| panic!("missing input {name}"))
                .1
                .clone();
            let shape_values = if let Data::Matrix(values) = &data {
                mech_core::shape_for_resolved_extents(
                    artifact.schemas().get(port.schema).unwrap(),
                    &[1, values.len() as u64],
                )
                .unwrap()
                .parameter_values()
                .to_vec()
                .into_boxed_slice()
            } else {
                Box::new([])
            };
            let value = ValueDraft {
                schema: port.schema,
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
fn load(object: &crate::native_artifact::NativeProgramObject, dir: &Path) -> LinkedProgramArtifact {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(dir.join("object.o"), object.bytes()).unwrap();
    std::fs::write(
        dir.join("descriptor.json"),
        object.descriptor().encode_build_owned_image().unwrap(),
    )
    .unwrap();
    let library = dir.join("program.dylib");
    let output = Command::new("/usr/bin/xcrun")
        .args(["clang", "-dynamiclib"])
        .arg(dir.join("object.o"))
        .arg("-o")
        .arg(&library)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let file = CString::new(library.to_str().unwrap()).unwrap();
    let handle = unsafe { dlopen(file.as_ptr(), 2) };
    assert!(
        !handle.is_null(),
        "dylib: {}",
        unsafe { std::ffi::CStr::from_ptr(dlerror()) }.to_string_lossy()
    );
    // The test deliberately retains the real library until process exit. Every
    // pointer is a genuine compiler-emitted export of this exact checked Object.
    let symbol = CString::new(object.compiled_module_symbol()).unwrap();
    let module = unsafe { dlsym(handle, symbol.as_ptr()) };
    assert!(!module.is_null());
    unsafe {
        LinkedProgramArtifact::from_compiled_module(
            &*(module as *const NativeProgramModule),
            &mech_stdlib::source_catalog(),
        )
    }
    .unwrap()
}
fn evaluate(
    artifact: &ProgramArtifact,
    named: &[(&str, Data)],
    limit: Option<u64>,
    library_limit: Option<u64>,
    run: impl FnOnce(
        &[Value],
        &mut [Option<Value>],
        crate::native_artifact::NativeProgramExecutionLimits,
        &ManagedMemoryBudget,
    ) -> Result<
        crate::native_artifact::NativeProgramExecutionReport,
        crate::native_artifact::NativeProgramExecutionError,
    >,
) -> (String, Vec<Data>) {
    let mode = NativeProgramLimitMode::NoLimits;
    let memory = mode.memory_budget();
    let args = inputs(artifact, named, &memory);
    let aliases = args.clone();
    let before = aliases
        .iter()
        .map(|v| v.canonical_data_draft().unwrap())
        .collect::<Vec<_>>();
    let mut out = vec![None; artifact.outputs().len()];
    let mut limits = mode.execution();
    if let Some(n) = limit {
        limits.execution.max_work = n;
    }
    if let Some(n) = library_limit {
        limits.library.max_compute_work = Some(n);
    }
    let result = run(&args, &mut out, limits, &memory);
    let text = format!("{result:?}");
    let values = out
        .iter()
        .map(|v| v.as_ref().map(|v| v.canonical_data_draft().unwrap()))
        .collect::<Vec<_>>();
    assert_eq!(
        aliases
            .iter()
            .map(|v| v.canonical_data_draft().unwrap())
            .collect::<Vec<_>>(),
        before
    );
    if result.is_err() {
        assert!(values.iter().all(|v| v.is_none()));
    }
    let values = values.into_iter().flatten().collect();
    drop((args, aliases, out));
    assert_eq!(
        (memory.used_bytes(), memory.retained_snapshot_nodes()),
        (0, 0)
    );
    (text, values)
}
#[test]
#[ignore = "requires MECH_LLVM_TEST_BIN and Apple AArch64"]
fn llvm_image_sources_errors_finite_limits_and_backend_forgery() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root).unwrap();
    let toolchain = NativeLlvmToolchain::new(
        std::env::var("MECH_LLVM_TEST_BIN").expect("explicit LLVM compiler directory"),
    )
    .with_diagnostics(root.join("compiler-work"));
    let mode = NativeProgramLimitMode::NoLimits;
    let f = |n: f64| Data::F64(F64Bits::from_f64(n));
    let cases: Vec<(&str, &str, Vec<Vec<(&str, Data)>>)> = vec![
        (
            "u64-add",
            "value<u64> + 1u64\n",
            vec![
                vec![("value", Data::U64(7))],
                vec![("value", Data::U64(u64::MAX))],
            ],
        ),
        (
            "u64-div",
            "value<u64> / divisor<u64>\n",
            vec![
                vec![("value", Data::U64(17)), ("divisor", Data::U64(3))],
                vec![("value", Data::U64(17)), ("divisor", Data::U64(0))],
            ],
        ),
        (
            "i64-div",
            "value<i64> / divisor<i64>\n",
            vec![
                vec![("value", Data::I64(-17)), ("divisor", Data::I64(3))],
                vec![("value", Data::I64(i64::MIN)), ("divisor", Data::I64(-1))],
            ],
        ),
        (
            "bool-branch",
            "(flag<bool>? | true => value<u64> + 1u64 | false => value<u64>)\n",
            vec![
                vec![("flag", Data::Bool(true)), ("value", Data::U64(5))],
                vec![("flag", Data::Bool(false)), ("value", Data::U64(u64::MAX))],
            ],
        ),
        (
            "f64-add",
            "value<f64> + 1.25\n",
            vec![
                vec![("value", f(1.5))],
                vec![("value", f(f64::NAN))],
                vec![("value", f(-0.0))],
            ],
        ),
        (
            "f64-to-u64",
            "result<u64> := value<f64>\nresult\n",
            vec![
                vec![("value", f(42.9))],
                vec![("value", f(-1.0))],
                vec![("value", f(f64::INFINITY))],
                vec![("value", f(f64::NAN))],
            ],
        ),
    ];
    let mut comparisons = 0;
    let mut loader_checks = 0;
    for (name, source, inputs) in cases {
        let original = compile(source);
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(
            &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
        )
        .unwrap();
        let mut original_bytes = None;
        for (kind, artifact) in [("original", original), ("decoded", decoded)] {
            let dir = root.join(name).join(kind);
            std::fs::create_dir_all(&dir).unwrap();
            let object = emit_program_llvm_object_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                mode.compilation(),
                &toolchain,
            )
            .unwrap();
            if name == "u64-add" && kind == "original" {
                compiler_thresholds(&artifact, &object, &toolchain);
            }
            if let Some(prior) = &original_bytes {
                assert_eq!(object.bytes(), prior);
            } else {
                original_bytes = Some(object.bytes().to_vec());
            }
            let image = object.descriptor().encode_build_owned_image().unwrap();
            let independent = unsafe {
                NativeProgramDescriptor::decode_build_owned_image(
                    &image,
                    &mech_stdlib::source_catalog(),
                )
            }
            .unwrap();
            assert_eq!(independent.encode_build_owned_image().unwrap(), image);
            loader_checks += 1;
            let linked = load(&object, &dir);
            let jit = JitProgramArtifact::compile_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                mode.compilation(),
            )
            .unwrap();
            for named in &inputs {
                for (work, library) in [
                    (None, None),
                    (Some(0), None),
                    (Some(1), None),
                    (Some(2), None),
                    (Some(7), None),
                    (Some(64), None),
                    (None, Some(0)),
                    (None, Some(1)),
                ] {
                    let native = evaluate(&artifact, named, work, library, |a, o, l, m| {
                        linked.evaluate_into(a, o, l, m)
                    });
                    let reference = evaluate(&artifact, named, work, library, |a, o, l, m| {
                        jit.evaluate_into(a, o, l, m)
                    });
                    assert_eq!(
                        native, reference,
                        "{name} {kind} work{work:?} library{library:?}"
                    );
                    comparisons += 1;
                }
            }
            let value: serde_json::Value = serde_json::from_slice(&image).unwrap();
            for mutation in 0..9 {
                let mut changed = value.clone();
                match mutation {
                    0 => {
                        changed["descriptor"]
                            .as_object_mut()
                            .unwrap()
                            .remove("machine_backend");
                    }
                    1 => changed["descriptor"]["machine_backend"] = serde_json::Value::Null,
                    2 => changed["version"] = 9.into(),
                    3 => changed["descriptor"]["machine_backend"]["kind"] = 2.into(),
                    4 => changed["descriptor"]["machine_backend"]["revision"] = 2.into(),
                    5 => {
                        changed["descriptor"]["machine_backend"]["options"][0] =
                            (value["descriptor"]["machine_backend"]["options"][0]
                                .as_u64()
                                .unwrap()
                                ^ 1)
                            .into()
                    }
                    6 => {
                        changed["descriptor"]["machine_backend"]["toolchain"][0] =
                            (value["descriptor"]["machine_backend"]["toolchain"][0]
                                .as_u64()
                                .unwrap()
                                ^ 1)
                            .into()
                    }
                    7 => {
                        changed["descriptor"]["machine_backend"]["machine_object"][0] =
                            (value["descriptor"]["machine_backend"]["machine_object"][0]
                                .as_u64()
                                .unwrap()
                                ^ 1)
                            .into()
                    }
                    _ => {
                        changed["descriptor"]["machine_backend"]["source_ir"][0] =
                            (value["descriptor"]["machine_backend"]["source_ir"][0]
                                .as_u64()
                                .unwrap()
                                ^ 1)
                            .into()
                    }
                }
                let altered = serde_json::to_vec(&changed).unwrap();
                assert!(
                    unsafe {
                        NativeProgramDescriptor::decode_build_owned_image(
                            &altered,
                            &mech_stdlib::source_catalog(),
                        )
                    }
                    .is_err(),
                    "{name} {mutation}"
                );
                loader_checks += 1;
            }
            // A self-consistent machine identity is not a source proof. This
            // reaches independent source rederivation rather than merely
            // tripping the stamp/identity consistency check above.
            let mut changed = value.clone();
            changed["descriptor"]["machine_backend"]["source_ir"][0] =
                (value["descriptor"]["machine_backend"]["source_ir"][0]
                    .as_u64()
                    .unwrap()
                    ^ 1)
                .into();
            let stamp: crate::native_artifact::program::image::machine_backend::MachineBackend =
                serde_json::from_value(changed["descriptor"]["machine_backend"].clone()).unwrap();
            changed["descriptor"]["identity"] = serde_json::to_value(stamp.identity()).unwrap();
            let error = unsafe {
                NativeProgramDescriptor::decode_build_owned_image(
                    &serde_json::to_vec(&changed).unwrap(),
                    &mech_stdlib::source_catalog(),
                )
            }
            .err()
            .expect("a recomputed identity must not authorize forged source facts");
            assert!(
                error.contains("machine backend source identity differs"),
                "{error}"
            );
            loader_checks += 1;
            println!(
                "{}",
                serde_json::json!({"case":name,"artifact":kind,"comparisons_so_far":comparisons,"loader_checks_so_far":loader_checks,"LLVM_executed":true,"timing_claim":false})
            );
        }
    }
    std::fs::write(root.join("summary.json"),serde_json::to_vec_pretty(&serde_json::json!({"comparisons":comparisons,"loader_checks":loader_checks,"original_decoded_Objects_identical":true,"LLVM_executed":true,"source_errors_finite_limits_aliases_cleanup_exact":true,"timing_claim":false})).unwrap()).unwrap();
}
