//! General checked U16 scalar arithmetic, image restoration and finite work.
use super::*;
use crate::native_artifact::compact_runtime::NativeStatus;
use crate::native_artifact::program::prepare::PreparedNativeProgram;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{FuncOrDataId, Module, default_libcall_names};

const OPERATORS: [&str; 5] = ["+", "-", "*", "/", "%"];
const VALUES: [u16; 10] = [0, 1, 2, 255, 256, 32767, 32768, 65533, 65534, 65535];
fn expected(op: &str, a: u16, b: u16) -> Option<u16> {
    match op {
        "+" => a.checked_add(b),
        "-" => a.checked_sub(b),
        "*" => a.checked_mul(b),
        "/" => a.checked_div(b),
        "%" => a.checked_rem(b),
        _ => unreachable!(),
    }
}
fn arguments(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    a: u16,
    b: u16,
) -> Vec<Value> {
    inputs(
        artifact,
        &[("left", Data::U16(a)), ("right", Data::U16(b))],
        memory,
    )
}
#[test]
fn u16_all_checked_operations_keep_extremes_original_decoded_restored_and_atomic_errors() {
    let mode = NativeProgramLimitMode::NoLimits;
    for op in OPERATORS {
        let text = format!("left<u16> {op} right<u16>\n");
        for artifact in artifacts(&text) {
            let native = JitProgramArtifact::compile_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                mode.compilation(),
            )
            .unwrap();
            let memory = mode.memory_budget();
            for restored in [false, true] {
                let mut output = vec![None; artifact.outputs().len()];
                for a in VALUES {
                    for b in VALUES {
                        let args = arguments(&artifact, &memory, a, b);
                        let aliases = args.clone();
                        let before = contents(&args);
                        let prior = output.clone();
                        let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                        let run = if restored {
                            observe::run_restored(
                                &native,
                                &args,
                                &mut output,
                                mode.execution(),
                                &memory,
                            )
                            .0
                        } else {
                            observe::run(&native, &args, &mut output, mode.execution(), &memory).0
                        };
                        match expected(op, a, b) {
                            Some(value) => {
                                assert!(run.is_ok(), "{op} {a} {b}: {run:?}");
                                assert_eq!(
                                    output
                                        .last()
                                        .unwrap()
                                        .as_ref()
                                        .unwrap()
                                        .canonical_data_draft()
                                        .unwrap(),
                                    Data::U16(value)
                                );
                            }
                            None => {
                                assert!(run.is_err(), "{op} {a} {b}");
                                assert!(output.iter().zip(&prior).all(|(a, b)| match (a, b) {
                                    (Some(a), Some(b)) => a.shares_frozen_storage(b),
                                    (None, None) => true,
                                    _ => false,
                                }));
                                assert_eq!(
                                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                                    retained
                                );
                            }
                        }
                        assert_eq!(contents(&args), before);
                        assert_eq!(contents(&aliases), before);
                    }
                }
            }
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}
#[test]
fn u16_compact_native_operations_keep_values_first_failure_origins_and_retry() {
    for op in OPERATORS {
        let mut expected_origin = None;
        for artifact in artifacts(&format!("left<u16> {op} right<u16>\n")) {
            let catalog = mech_stdlib::source_catalog();
            let plan =
                mech_engine::native_artifact::lower_native_program_artifact(&artifact, &catalog)
                    .unwrap();
            let prepared = PreparedNativeProgram::prepare(&artifact, &catalog, plan).unwrap();
            let builder = JITBuilder::with_flags(
                &[
                    ("opt_level", "speed"),
                    ("is_pic", "true"),
                    ("preserve_frame_pointers", "true"),
                    ("unwind_info", "true"),
                    (
                        "log2_min_function_alignment",
                        if cfg!(target_arch = "aarch64") {
                            "7"
                        } else {
                            "0"
                        },
                    ),
                ],
                default_libcall_names(),
            )
            .unwrap();
            let mut module = JITModule::new(builder);
            let metadata =
                crate::native_artifact::program::compact::define(&mut module, &prepared, false)
                    .unwrap();
            let (_, object_metadata) =
                crate::native_artifact::emit_compact_closed_object_for_diagnostic(
                    &artifact, &catalog,
                )
                .unwrap();
            assert_eq!(metadata["output_words"], 1);
            assert_eq!(metadata["concrete_heap_context"], false);
            assert_eq!(object_metadata["output_types"], metadata["output_types"]);
            let Some(FuncOrDataId::Func(id)) =
                module.get_name(metadata["symbol"].as_str().unwrap())
            else {
                panic!("compact entry")
            };
            module.finalize_definitions().unwrap();
            // SAFETY: this module owns the exact typed compact entry for the whole call set.
            let run: unsafe extern "C" fn(*const u64, *mut u64) -> NativeStatus =
                unsafe { core::mem::transmute(module.get_finalized_function(id)) };
            let mut arguments = vec![0; metadata["input_words"].as_u64().unwrap() as usize];
            for a in VALUES {
                for b in VALUES {
                    for port in metadata["inputs"].as_array().unwrap() {
                        let name =
                            mech_engine::decode_source_input_name(port["name"].as_str().unwrap())
                                .unwrap();
                        arguments[port["input_offset"].as_u64().unwrap() as usize] =
                            u64::from(if name == "left" {
                                a
                            } else {
                                assert_eq!(name, "right");
                                b
                            });
                    }
                    let before = arguments.clone();
                    let mut output = [0xcafe];
                    let status = unsafe { run(arguments.as_ptr(), output.as_mut_ptr()) };
                    match expected(op, a, b) {
                        Some(value) => {
                            assert_eq!(status.status, 0, "{op} {a} {b}");
                            assert_eq!(output, [u64::from(value)]);
                        }
                        None => {
                            assert_ne!(status.status, 0);
                            assert_eq!(output, [0xcafe]);
                            assert!(status.origin < metadata["source_blocks"].as_u64().unwrap());
                            assert_eq!(
                                status.origin,
                                *expected_origin.get_or_insert(status.origin)
                            );
                        }
                    }
                    assert_eq!(arguments, before);
                }
            }
        }
    }
}
#[test]
fn u16_checked_operations_keep_finite_work_and_atomic_publication_boundaries() {
    for op in OPERATORS {
        resources::finite_boundaries(
            &format!("left<u16> {op} right<u16>\n"),
            |artifact, memory| arguments(artifact, memory, 9, 3),
        );
    }
}
