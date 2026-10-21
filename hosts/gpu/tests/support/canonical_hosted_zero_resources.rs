//! Empty resource plans still use the SAME generated shell and phased session.
//! Generation is not linked-process execution; that qualification is separate.
use super::*;
use mech_core::{Value, ValueData};
use mech_gpu::native_artifact::{
    NativeApplicationError, generate_linked_hosted_program_application,
};
use sha2::{Digest, Sha256};
use std::{cell::Cell, fs, path::PathBuf};

const SOURCE: &str = "answer := 7u64\nanswer + 5u64\n";

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}
fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
fn zero_ports() -> CanonicalNativeResourceLimits {
    CanonicalNativeResourceLimits {
        max_ports: 0,
        max_input_text_bytes: 0,
        max_output_text_bytes: 0,
    }
}

#[test]
fn original_and_decoded_closed_program_generate_same_hosted_object_and_pins() {
    let (variants, catalog) = artifacts(SOURCE);
    let temporary = tempfile::tempdir().unwrap();
    let mut pins = None;
    for (index, artifact) in variants.iter().enumerate() {
        assert!(artifact.inputs().is_empty());
        assert!(artifact.requirements().is_empty());
        let selected = CanonicalNativeResourcePlan::select(artifact, 0).unwrap();
        assert!(selected.requirements().is_empty());
        assert!(selected.observations().is_empty());
        assert!(selected.effects().is_empty());
        let project = generate_linked_hosted_program_application(
            &workspace(),
            &temporary.path().join(format!("project-{index}")),
            "canonical-closed-program",
            artifact,
            &catalog,
            SOURCE.as_bytes(),
        )
        .unwrap();
        for (path, expected) in project.pins() {
            assert_eq!(
                &hash(&fs::read(project.root().join(path)).unwrap()),
                expected
            );
        }
        if let Some(expected) = &pins {
            assert_eq!(project.pins(), expected);
        } else {
            pins = Some(project.pins().clone());
        }
        assert_eq!(
            fs::read(project.root().join("application.mec")).unwrap(),
            SOURCE.as_bytes()
        );
        assert!(
            !fs::read(project.root().join("program.o"))
                .unwrap()
                .is_empty()
        );
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(
            &fs::read(project.root().join("program.mecb")).unwrap(),
        )
        .unwrap();
        assert_eq!(decoded.revision(), artifact.revision());
        assert!(decoded.requirements().is_empty());
        let main = fs::read_to_string(project.root().join("src/main.rs")).unwrap();
        let manifest = fs::read_to_string(project.manifest_path())
            .unwrap()
            .parse::<toml_edit::DocumentMut>()
            .unwrap();
        assert_eq!(
            manifest["profile"]["release"]["strip"].as_str(),
            Some("symbols")
        );
        assert_eq!(
            manifest["profile"]["release"]["opt-level"].as_integer(),
            Some(3)
        );
        assert_eq!(manifest["profile"]["dev"]["debug"].as_integer(), Some(0));
        assert!(main.contains("with_canonical_native_resources_phased"));
        assert!(main.contains("linked.prepare_hosted"));
        assert!(main.contains("resume_hosted_into"));
        let linkage =
            mech_engine::linked_factories::NativeFactoryLinkageProfile::derive(artifact, &catalog);
        for factory in linkage
            .factories()
            .expect("closed scalar operations have maintained linkage")
        {
            assert!(main.contains(&format!("{}.install(&mut builder)", factory.rust_path())));
        }
        assert!(!main.contains("mech_engine::install_intrinsic_resident"));
        assert!(!main.contains("mech_stdlib::install_native_plan"));
        assert_eq!(project.metadata()["source_is_provenance_only"], true);
        assert_eq!(project.metadata()["linked"], false);
        assert_eq!(project.metadata()["executed"], false);
        assert_eq!(project.metadata()["timing_samples"], 0);
    }
    // A resource-free ordinary INPUT is still unbound, not silently converted
    // to a provider read or an ambient command-line argument.
    let (variants, catalog) = artifacts("value<u64>\n");
    for (index, artifact) in variants.iter().enumerate() {
        assert!(!artifact.inputs().is_empty());
        let directory = temporary.path().join(format!("unbound-{index}"));
        assert!(matches!(
            generate_linked_hosted_program_application(
                &workspace(),
                &directory,
                "canonical-closed-program",
                artifact,
                &catalog,
                b"source-unpaired input negative",
            ),
            Err(NativeApplicationError::Invalid(
                "hosted application must bind its application inputs through resources"
            ))
        ));
        assert!(!directory.exists());
    }
}

#[test]
fn canonical_only_catalog_preserves_original_and_decoded_native_objects() {
    let mut builder = mech_core::FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut builder).unwrap();
    let canonical = builder.build().unwrap();
    assert_eq!(canonical.runtime_factory_count(), 0);
    assert_eq!(canonical.specializer_count(), 0);
    assert!(canonical.resident_factory_count() > 0);

    // Source compilation still uses the ordinary complete catalog. The
    // linked shell only rebinds its resulting canonical artifact; it must not
    // keep the old bytecode factories (and all their callees) alive as well.
    for source in [SOURCE, FULL, ECHO, TWO, CLOSED_FAIL, CLOSED_LIVE, FAIL] {
        let (variants, complete) = artifacts(source);
        for artifact in variants {
            let expected =
                mech_gpu::native_artifact::emit_program_object(&artifact, &complete).unwrap();
            let selected =
                mech_gpu::native_artifact::emit_program_object(&artifact, &canonical).unwrap();
            assert_eq!(selected.bytes(), expected.bytes());
            assert_eq!(selected.identity().bytes(), expected.identity().bytes());
            assert_eq!(selected.metadata(), expected.metadata());
        }
    }

    let (variants, _) = artifacts(FULL);
    for artifact in variants {
        let jit = JitProgramArtifact::compile(&artifact, &canonical).unwrap();
        let (mut instance, trace) = runtime(GGTA_INPUT, true);
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        super::execute(&mut instance, &artifact, &jit, limits(), &memory).unwrap();
        assert_eq!(trace.lock().unwrap().output, GGTA_OUTPUT);
        assert_eq!(memory.used_bytes(), 0);
        no_resident(&instance);
        instance.shutdown().unwrap();
    }
}

fn execute(
    runtime: &mut MechRuntime,
    artifact: &mech_engine::ProgramArtifact,
    jit: &JitProgramArtifact,
    outputs: &mut [Option<Value>],
    policy: NativeProgramExecutionLimits,
    memory: &ManagedMemoryBudget,
    entered: &Cell<(usize, usize)>,
) -> Result<NativeProgramExecutionReport, CanonicalNativeResourceRunError> {
    runtime.with_canonical_native_resources_phased(
        artifact,
        zero_ports(),
        |admission| {
            assert!(admission.plan().requirements().is_empty());
            assert!(admission.plan().observations().is_empty());
            assert!(admission.plan().effects().is_empty());
            let (prepare, resume) = entered.get();
            entered.set((prepare + 1, resume));
            jit.prepare_hosted(admission, policy, memory)
                .map_err(|error| MechError::new(error, None))
        },
        |session, turn| {
            assert!(turn.observations().is_empty());
            assert!(turn.plan().effects().is_empty());
            let (prepare, resume) = entered.get();
            entered.set((prepare, resume + 1));
            let report = session
                .resume_hosted_into(turn, &[], outputs)
                .map_err(|error| MechError::new(error, None))?;
            // Ordinary candidates are NOT resource effects or automatic stdout.
            Ok(CanonicalNativeResourceCompletion {
                result: report,
                effects: Box::new([]),
            })
        },
    )
}

#[test]
fn zero_grant_closed_program_uses_real_phased_jit_session_without_io_authority() {
    let (variants, catalog) = artifacts(SOURCE);
    for artifact in variants {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let plan = jit.descriptor().plan();
        assert!(plan.resource_requirements().is_empty());
        assert!(plan.observations().is_empty());
        assert!(plan.effects().is_empty());
        assert_eq!(plan.outputs().len(), 1);
        for monitored_cli in [false, true] {
            let (mut instance, trace) = if monitored_cli {
                let (instance, trace) = runtime(b"must-not-read", false);
                (instance, Some(trace))
            } else {
                (RuntimeBuilder::new().build().unwrap(), None)
            };
            let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
            let mut output = vec![None; plan.transport_output_count()];
            let entered = Cell::new((0, 0));
            let report = execute(
                &mut instance,
                &artifact,
                &jit,
                &mut output,
                limits(),
                &memory,
                &entered,
            )
            .unwrap();
            assert_eq!(entered.get(), (1, 1));
            assert!(report.work > 0 && report.visits > 0);
            assert!(matches!(
                output[0].as_ref().unwrap().data(),
                ValueData::U64(12)
            ));
            let prior = output[0].as_ref().unwrap().clone();
            let baseline = memory.used_bytes();
            let mut denied = limits();
            denied.execution.max_work = 1;
            assert!(matches!(
                execute(
                    &mut instance,
                    &artifact,
                    &jit,
                    &mut output,
                    denied,
                    &memory,
                    &entered
                ),
                Err(CanonicalNativeResourceRunError::BeforeCommit {
                    phase: "native activation",
                    ..
                })
            ));
            assert_eq!(entered.get(), (2, 1), "failed preparation must not resume");
            assert!(output[0].as_ref().unwrap().shares_frozen_storage(&prior));
            assert_eq!(memory.used_bytes(), baseline);
            execute(
                &mut instance,
                &artifact,
                &jit,
                &mut output,
                limits(),
                &memory,
                &entered,
            )
            .unwrap();
            assert_eq!(entered.get(), (3, 2));
            assert!(matches!(
                output[0].as_ref().unwrap().data(),
                ValueData::U64(12)
            ));
            if let Some(trace) = trace {
                let trace = trace.lock().unwrap();
                assert_eq!(
                    (
                        trace.reads,
                        trace.acquisitions,
                        trace.prepares,
                        trace.deliveries,
                        trace.flushes
                    ),
                    (0, 0, 0, 0, 0)
                );
                assert!(trace.output.is_empty());
            }
            no_resident(&instance);
            instance.shutdown().unwrap();
            drop((prior, output));
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}
