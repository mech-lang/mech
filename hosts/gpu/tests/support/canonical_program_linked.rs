//! Actual managed-value linked processes; no timing or resource-host claim.
use super::{LEXICAL, ROOT, artifacts_mode, equal, inputs, limits, reference_result};
use mech_build::{CargoInvocation, parse_cargo_build_messages};
use mech_core::{
    ManagedMemoryBudget, ValueDataDraft,
    snapshot::{SnapshotValidationContext, ValueDraft},
};
use mech_gpu::native_artifact::{
    JitProgramArtifact, NativeExecutionError, NativeProgramExecutionError, NativeValueError,
    generate_linked_program_application,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    io::Cursor,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};

#[path = "linked_process.rs"]
mod linked_process;
#[path = "canonical_program_pins.rs"]
mod provenance;
use linked_process::{
    assert_no_ambient_cargo_config, bounded, clean, closure, hash, pin, workspace,
};

fn exported(artifact: &mech_engine::ProgramArtifact, values: &[mech_core::Value]) -> Vec<u8> {
    let mut result = String::new();
    for value in values {
        let bytes = value.canonical_snapshot_bytes(artifact.schemas()).unwrap();
        result.extend(bytes.iter().map(|byte| format!("{byte:02x}")));
        result.push('\n');
    }
    result.into_bytes()
}
fn policy(work: u64, memory: u64, compute: u64, temporary: u64) -> Vec<String> {
    vec![
        work.to_string(),
        memory.to_string(),
        compute.to_string(),
        "1000000".into(),
        temporary.to_string(),
        "--".into(),
    ]
}
fn input_file(directory: &Path, label: &str, values: &[mech_core::Value]) -> PathBuf {
    let drafts = values
        .iter()
        .map(|value| ValueDraft {
            schema: value.schema(),
            shape_values: value.shape().parameter_values().to_vec().into_boxed_slice(),
            data: value.canonical_data_draft().unwrap(),
        })
        .collect::<Vec<_>>();
    let path = directory.join(format!("{label}-input.json"));
    assert!(!path.exists());
    fs::write(&path, serde_json::to_vec(&drafts).unwrap()).unwrap();
    path
}

// Match the generated shell's public draft import and shared schema ownership,
// so the limit below admits precisely the same input bytes in both processes.
fn import_inputs(
    artifact: &mech_engine::ProgramArtifact,
    input: &Path,
    memory: &ManagedMemoryBudget,
) -> Vec<mech_core::Value> {
    let drafts: Vec<ValueDraft> = serde_json::from_slice(&fs::read(input).unwrap()).unwrap();
    let schemas = Arc::new(artifact.schemas().clone());
    drafts
        .into_iter()
        .map(|draft| {
            let value = draft
                .finalize(&SnapshotValidationContext::with_shared_schemas(&schemas))
                .unwrap();
            let mut reservation = memory
                .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
                .unwrap();
            value.into_memory_budget(&mut reservation).unwrap()
        })
        .collect()
}

#[test]
fn linked_assertion_rejection_preserves_publication_and_recovers() {
    let source = "checked(n<u64>) => (out<u64>) :=\n  positive! := n > 0u64\n  out := 12u64 / n.\nchecked(input<u64>)\n";
    linked_assertion_retry(source, ValueDataDraft::U64);
}

#[test]
fn linked_narrow_comparison_assertion_preserves_publication_and_recovers() {
    let source = "checked(n<u32>) => (out<u64>) :=\n  positive! := n > 0u32\n  wide<u64> := n\n  out := 12u64 / wide.\nchecked(input<u32>)\n";
    linked_assertion_retry(source, |n| ValueDataDraft::U32(n.try_into().unwrap()));
}

fn linked_assertion_retry(source: &str, input_word: fn(u64) -> ValueDataDraft) {
    let temporary = tempfile::tempdir().unwrap();
    let directory = temporary.path();
    let artifact = super::artifacts(source).0.into_iter().next().unwrap();
    let generated = generate_linked_program_application(
        &workspace(),
        &directory.join("project"),
        "checked-assertion",
        &artifact,
        &mech_stdlib::source_catalog(),
        source.as_bytes(),
    )
    .unwrap();
    // Keep every build-pinned generated input unchanged. This additional binary
    // reuses the maintained shell and the SAME pinned Object/descriptor, with
    // only the test driver's invocation sequence replaced. It contains no JIT.
    let shell = fs::read_to_string(generated.root().join("src/main.rs")).unwrap();
    // The auxiliary binary lives one directory deeper than the pinned shell.
    let descriptor = "include_bytes!(\"../native-descriptor.bin\")";
    assert_eq!(shell.matches(descriptor).count(), 1);
    let shell = shell.replace(
        descriptor,
        "include_bytes!(\"../../native-descriptor.bin\")",
    );
    let invocation = "    linked.evaluate_into(&inputs, &mut outputs, limits, &memory).map_err(|error| error.to_string())?;";
    assert_eq!(shell.matches(invocation).count(), 1);
    let driver = shell.replace(invocation, r#"
    for (number, expected) in [(3, Some(4)), (0, None), (2, Some(6))] {
        let data = match inputs[0].canonical_data_draft().unwrap() {
            mech_core::ValueDataDraft::U32(_) => mech_core::ValueDataDraft::U32(number.try_into().unwrap()),
            mech_core::ValueDataDraft::U64(_) => mech_core::ValueDataDraft::U64(number),
            _ => panic!("unexpected assertion input kind"),
        };
        let value = ValueDraft {
            schema: inputs[0].schema(), shape_values: Box::new([]),
            data,
        }.finalize(&SnapshotValidationContext::with_shared_schemas(&schemas)).unwrap();
        let mut reservation = memory.reserve_capacity(value.memory_budget_admission_bytes(&memory).unwrap()).unwrap();
        let arguments = [value.into_memory_budget(&mut reservation).unwrap()];
        let prior = outputs.clone(); // Retain published aliases across rejection.
        let before = memory.used_bytes();
        let result = linked.evaluate_into(&arguments, &mut outputs, limits, &memory);
        if let Some(expected) = expected {
            assert!(result.unwrap().visits > 0);
            assert_eq!(outputs[0].as_ref().unwrap().canonical_data_draft().unwrap(), mech_core::ValueDataDraft::U64(expected));
        } else {
            let mech_gpu::native_artifact::NativeProgramExecutionError::Control(
                mech_gpu::native_artifact::NativeIterationExecutionError::Integrity { origin }
            ) = result.unwrap_err() else { panic!("expected lexical integrity before division") };
            assert!(origin.block().is_some() && origin.local().is_some());
            for (actual, old) in outputs.iter().zip(&prior) {
                assert!(actual.as_ref().unwrap().snapshot_eq(&schemas, old.as_ref().unwrap(), &schemas).unwrap());
            }
            assert_eq!(memory.used_bytes(), before);
        }
    }
"#);
    fs::create_dir(generated.root().join("src/bin")).unwrap();
    fs::write(generated.root().join("src/bin/lexical-retry.rs"), driver).unwrap();
    fs::copy(
        workspace().join("Cargo.lock"),
        generated.root().join("Cargo.lock"),
    )
    .unwrap();
    let isolated = generated.root().ancestors().last().unwrap();
    let resolved = bounded(
        &mut CargoInvocation::resolve_lockfile(generated.manifest_path(), true)
            .with_current_dir(isolated)
            .command(),
        directory,
        "metadata",
        120,
    );
    assert!(
        resolved.status.success(),
        "{}",
        String::from_utf8_lossy(&resolved.stderr)
    );
    let mut command = CargoInvocation::build(
        generated.manifest_path(),
        "lexical-retry",
        None,
        directory.join("cargo-target"),
        false,
        true,
    )
    .unwrap()
    .with_current_dir(isolated)
    .command();
    command
        .env("CARGO_INCREMENTAL", "0")
        .env("CARGO_PROFILE_DEV_DEBUG", "0")
        .env("CARGO_BUILD_JOBS", "1");
    let built = bounded(&mut command, directory, "build", 15 * 60);
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let metadata: Value = serde_json::from_slice(&resolved.stdout).unwrap();
    let units = closure(&metadata, &built.stdout);
    assert!(units.iter().any(|unit| unit["root"] == true));
    let messages = parse_cargo_build_messages(Cursor::new(&built.stdout)).unwrap();
    assert_eq!(messages.build_finished, Some(true));
    let binary = messages.select_binary("lexical-retry").unwrap().executable;
    let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
    let args = inputs(&artifact, &[("input", input_word(3))], &memory);
    let input = input_file(directory, "checked", &args);
    let result = bounded(
        Command::new(binary)
            .args(policy(
                1_000_000_000,
                256 * 1024 * 1024,
                134_217_728,
                128 * 1024 * 1024,
            ))
            .arg(input),
        directory,
        "retry",
        30,
    );
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(result.stderr.is_empty());
    let recovery = inputs(&artifact, &[("input", input_word(2))], &memory);
    assert_eq!(
        result.stdout,
        exported(&artifact, &reference_result(&artifact, &recovery).unwrap())
    );
    for (relative, expected) in generated.pins() {
        assert_eq!(&pin(&generated.root().join(relative)), expected);
    }
}

fn collection_source() -> String {
    format!(
        "{ROOT}projection := text/join([number/format(sequence/length(word) / (sequence/length(word) - divisor<u64>), 0u64) | word <- words], \",\")\n"
    )
}

fn windows_source() -> String {
    format!(
        "{}windowed := windows(input<string>, width<u64>)\nclosed-range := 1u64..2u64..=5u64\nexclusive := 1u64..stop<u64>\ninclusive := 1u64..=stop\nstepped := 1u64..increment<u64>..stop\nstepped-inclusive := 1u64..increment..=stop\nconverted<[f64]> := stepped\ninput-length := sequence/length(input)\nfirst := (input-length? | 0u64 => \"\" | * => input[1u64])\ncopy := input[:]\nquotient := 1u64 / divisor<u64>\n",
        super::window_qualification::WINDOWS_FUNCTION
    )
}

fn access_source() -> String {
    format!(
        r#"{ROOT}unique<{{string}}> := keys
present := "a" ∈ unique
absent := "never" ∉ unique
count := (present? | true => counts{{"a"}} | false => 0u64)
first := (size? | 0u64 => 0u64 | * => values[1u64])
constructed := {{"live": more, "next": size + 2u64, "last": size + 3u64, "wider": size + 4u64}}
constructed-value := constructed{{"next"}}
packed := (count, first, present, absent)
quotient := more / divisor<u64>
"#
    )
}

fn assert_access_oracle(
    artifact: &mech_engine::ProgramArtifact,
    expected: &[mech_core::Value],
    case: &str,
) {
    let named = |name| {
        let ordinal = artifact
            .outputs()
            .iter()
            .position(|o| {
                o.interactive_binding
                    .as_ref()
                    .is_some_and(|b| b.lexical_name == name)
            })
            .unwrap();
        expected[ordinal].canonical_data_draft().unwrap()
    };
    let (keys, count, first, constructed) = match case {
        "ordinary" => (&["a", "bb"][..], 2, 2, 8),
        "empty" => (&[][..], 0, 0, 2),
        "unicode" => (&["é", "👋"][..], 0, 2, 7),
        _ => panic!("missing independent access oracle"),
    };
    assert_eq!(
        named("unique"),
        ValueDataDraft::Set(
            keys.iter()
                .map(|key| ValueDataDraft::String((*key).into()))
                .collect()
        )
    );
    assert_eq!(named("count"), ValueDataDraft::U64(count));
    assert_eq!(named("first"), ValueDataDraft::U64(first));
    assert_eq!(named("constructed-value"), ValueDataDraft::U64(constructed));
    assert_eq!(
        named("packed"),
        ValueDataDraft::Tuple(
            vec![
                ValueDataDraft::U64(count),
                ValueDataDraft::U64(first),
                ValueDataDraft::Bool(case == "ordinary"),
                ValueDataDraft::Bool(true)
            ]
            .into()
        )
    );
}

#[test]
fn linked_access_fixture_checks_set_conversion_membership_lookup_and_general_construction() {
    let source = access_source();
    for (case, text) in [
        ("ordinary", "a\nbb\na"),
        ("empty", ""),
        ("unicode", "é\n👋\né"),
    ] {
        let named = [
            ("input", ValueDataDraft::String(text.into())),
            ("divisor", ValueDataDraft::U64(1)),
        ];
        for artifact in artifacts_mode(&source, true).0 {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let args = inputs(&artifact, &named, &memory);
            let expected = reference_result(&artifact, &args).unwrap();
            assert_access_oracle(&artifact, &expected, case);
        }
        super::check(&source, &named);
    }
}

fn window_oracle(case: &str) -> mech_core::ValueDataDraft {
    let words: &[&str] = match case {
        "ordinary" => &["AB", "BB", "BA"],
        "unicode" => &["é🧑‍🚀", "🧑‍🚀é"],
        "empty" | "zero-width" | "oversized-width" => &[],
        _ => panic!("missing independent window oracle"),
    };
    ValueDataDraft::Matrix(
        words
            .iter()
            .map(|word| ValueDataDraft::String((*word).into()))
            .collect(),
    )
}

fn assert_window_oracle(
    artifact: &mech_engine::ProgramArtifact,
    expected: &[mech_core::Value],
    case: &str,
) {
    // Interactive publication includes both document results and named
    // bindings; their ordinal is not the source declaration's position.
    let named = |name| {
        let index = artifact
            .outputs()
            .iter()
            .position(|output| {
                output
                    .interactive_binding
                    .as_ref()
                    .is_some_and(|binding| binding.lexical_name == name)
            })
            .unwrap_or_else(|| panic!("missing named output {name}"));
        expected[index].canonical_data_draft().unwrap()
    };
    assert_eq!(named("windowed"), window_oracle(case));
    assert_eq!(
        named("closed-range"),
        ValueDataDraft::Matrix([1, 3, 5].map(ValueDataDraft::U64).into())
    );
}

#[test]
fn linked_window_fixture_executes_general_ranges_selection_and_conversion() {
    let source = windows_source();
    for (case, text, width) in [
        ("ordinary", "ABBA", 2),
        ("unicode", "é🧑‍🚀é", 2),
        ("empty", "", 2),
        ("zero-width", "ABBA", 0),
        ("oversized-width", "ABBA", 5),
    ] {
        let named = [
            ("input", ValueDataDraft::String(text.into())),
            ("width", ValueDataDraft::U64(width)),
            ("stop", ValueDataDraft::U64(6)),
            ("increment", ValueDataDraft::U64(2)),
            ("divisor", ValueDataDraft::U64(1)),
        ];
        let (variants, _) = artifacts_mode(&source, true);
        for artifact in &variants {
            let account = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let args = inputs(artifact, &named, &account);
            let expected = reference_result(artifact, &args).unwrap();
            assert_window_oracle(artifact, &expected, case);
        }
        super::check(&source, &named);
    }
}

#[test]
fn linked_collection_fixture_checks_each_body_and_discards_a_partial_result() {
    let (variants, _) = artifacts_mode(&collection_source(), true);
    for artifact in variants {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let mut output = vec![None; artifact.outputs().len()];
        for (text, divisor) in [("a\nbb\na", 0), ("", 0), ("é\n👋\né", 0)] {
            let args = inputs(
                &artifact,
                &[
                    ("input", ValueDataDraft::String(text.into())),
                    ("divisor", ValueDataDraft::U64(divisor)),
                ],
                &memory,
            );
            let expected = reference_result(&artifact, &args).unwrap();
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            equal(&artifact, &output, &expected);
        }
        // The first word yields 2; the second divides by zero. Neither this
        // partial collection nor any earlier root may replace prior outputs.
        let args = inputs(
            &artifact,
            &[
                ("input", ValueDataDraft::String("bb\na".into())),
                ("divisor", ValueDataDraft::U64(1)),
            ],
            &memory,
        );
        assert!(reference_result(&artifact, &args).is_err());
        let prior = exported(
            &artifact,
            &output
                .iter()
                .map(|v| v.clone().unwrap())
                .collect::<Vec<_>>(),
        );
        let before = memory.used_bytes();
        let error = jit
            .evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap_err();
        assert!(matches!(
            error,
            NativeProgramExecutionError::Control(
                mech_gpu::native_artifact::NativeIterationExecutionError::Arithmetic { .. }
            )
        ));
        assert_eq!(memory.used_bytes(), before);
        assert_eq!(
            exported(
                &artifact,
                &output
                    .iter()
                    .map(|v| v.clone().unwrap())
                    .collect::<Vec<_>>()
            ),
            prior
        );
    }
}

#[test]
#[ignore = "explicit clean-source linked managed-program qualification; no timings"]
fn linked_managed_programs_preserve_canonical_outputs_errors_and_denials() {
    let commit = std::env::var("MECH_CANONICAL_PROGRAM_SOURCE_COMMIT")
        .expect("exact source commit required");
    let evidence = PathBuf::from(
        std::env::var_os("MECH_CANONICAL_PROGRAM_EVIDENCE_DIR")
            .expect("fresh absolute evidence directory required"),
    );
    assert!(evidence.is_absolute() && !evidence.exists());
    fs::create_dir_all(&evidence).unwrap();
    clean(&evidence, &commit, "before");
    assert_no_ambient_cargo_config();
    let root = format!("{ROOT}quotient := more / divisor<u64>\n");
    let lexical = LEXICAL
        .replacen(
            "step(x<string>)",
            "choose := flag<bool>\nstep(x<string>)",
            1,
        )
        .replace(
            "next := text/join(keys, formatted).",
            "next := (choose? | true => text/join(keys, formatted) | false => x).",
        );
    let catalog = mech_stdlib::source_catalog();
    let standard = policy(
        1_000_000_000,
        256 * 1024 * 1024,
        134_217_728,
        128 * 1024 * 1024,
    );
    let mut receipts = Vec::new();
    for (name, source, input_name) in [
        ("root", root, "input"),
        ("lexical", lexical, "seed"),
        ("collection", collection_source(), "input"),
        ("windows", windows_source(), "input"),
        ("access", access_source(), "input"),
    ] {
        let directory = evidence.join(name);
        fs::create_dir(&directory).unwrap();
        let (variants, _) = artifacts_mode(&source, true);
        let binary_name = format!("canonical-program-{name}");
        let generated = generate_linked_program_application(
            &workspace(),
            &directory.join("project"),
            &binary_name,
            &variants[0],
            &catalog,
            source.as_bytes(),
        )
        .unwrap();
        let decoded = generate_linked_program_application(
            &workspace(),
            &directory.join("decoded-project"),
            &binary_name,
            &variants[1],
            &catalog,
            source.as_bytes(),
        )
        .unwrap();
        assert_eq!(generated.pins(), decoded.pins());
        assert_eq!(
            generated.metadata()["typed_ir_sha256"],
            decoded.metadata()["typed_ir_sha256"]
        );
        let mut pins = BTreeMap::new();
        for project in [&generated, &decoded] {
            for (relative, expected) in project.pins() {
                let path = project.root().join(relative);
                assert_eq!(&pin(&path), expected);
                pins.insert(path, expected.clone());
            }
        }
        fs::copy(
            workspace().join("Cargo.lock"),
            generated.root().join("Cargo.lock"),
        )
        .unwrap();
        let isolated = generated.root().ancestors().last().unwrap();
        let resolved = bounded(
            &mut CargoInvocation::resolve_lockfile(generated.manifest_path(), true)
                .with_current_dir(isolated)
                .command(),
            &directory,
            "metadata",
            120,
        );
        assert!(
            resolved.status.success(),
            "{}",
            String::from_utf8_lossy(&resolved.stderr)
        );
        let metadata: Value = serde_json::from_slice(&resolved.stdout).unwrap();
        let mut command = CargoInvocation::build(
            generated.manifest_path(),
            &binary_name,
            None,
            evidence.join("cargo-target"),
            false,
            true,
        )
        .unwrap()
        .with_current_dir(isolated)
        .command();
        command
            .env("CARGO_INCREMENTAL", "0")
            .env("CARGO_PROFILE_DEV_DEBUG", "0")
            .env("CARGO_BUILD_JOBS", "1");
        let built = bounded(&mut command, &directory, "build", 15 * 60);
        assert!(
            built.status.success(),
            "{}",
            String::from_utf8_lossy(&built.stderr)
        );
        let messages = parse_cargo_build_messages(Cursor::new(&built.stdout)).unwrap();
        assert_eq!(messages.build_finished, Some(true));
        let units = closure(&metadata, &built.stdout);
        let binary = messages.select_binary(&binary_name).unwrap().executable;
        let binary_pin = pin(&binary);
        let jit = JitProgramArtifact::compile(&variants[0], &catalog).unwrap();
        let mut cases = Vec::new();
        let mut normal_input = None;
        let normal = if name == "windows" {
            vec![
                ("ordinary", "ABBA", 2),
                ("empty", "", 2),
                ("unicode", "é🧑‍🚀é", 2),
                ("zero-width", "ABBA", 0),
                ("oversized-width", "ABBA", 5),
            ]
        } else {
            vec![
                ("ordinary", "a\nbb\na", 2),
                ("empty", "", 2),
                ("unicode", "é\n👋\né", 2),
            ]
        };
        for (case, text, width) in normal {
            for choose in [false, true] {
                if name != "lexical" && !choose {
                    continue;
                }
                let label = format!("{case}-{choose}");
                let named = [
                    (input_name, ValueDataDraft::String(text.into())),
                    (
                        "divisor",
                        ValueDataDraft::U64(u64::from(name != "collection")),
                    ),
                    ("flag", ValueDataDraft::Bool(choose)),
                    ("width", ValueDataDraft::U64(width)),
                    ("stop", ValueDataDraft::U64(6)),
                    ("increment", ValueDataDraft::U64(2)),
                ];
                let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
                let args = inputs(&variants[0], &named, &memory);
                let expected = reference_result(&variants[0], &args).unwrap();
                if name == "windows" {
                    assert_window_oracle(&variants[0], &expected, case);
                }
                if name == "access" {
                    assert_access_oracle(&variants[0], &expected, case);
                }
                let mut output = vec![None; expected.len()];
                let report = jit
                    .evaluate_into(&args, &mut output, limits(), &memory)
                    .unwrap();
                assert!(report.visits > 0);
                equal(&variants[0], &output, &expected);
                let decoded_memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
                let decoded_args = inputs(&variants[1], &named, &decoded_memory);
                assert_eq!(
                    exported(&variants[0], &expected),
                    exported(
                        &variants[1],
                        &reference_result(&variants[1], &decoded_args).unwrap()
                    )
                );
                let paired = JitProgramArtifact::compile(&variants[1], &catalog).unwrap();
                let mut paired_output = vec![None; expected.len()];
                paired
                    .evaluate_into(&decoded_args, &mut paired_output, limits(), &decoded_memory)
                    .unwrap();
                equal(&variants[1], &paired_output, &expected);
                let input = input_file(&directory, &label, &args);
                let result = bounded(
                    Command::new(&binary).args(&standard).arg(&input),
                    &directory,
                    &label,
                    30,
                );
                assert!(
                    result.status.success(),
                    "{name}/{label}: {}",
                    String::from_utf8_lossy(&result.stderr)
                );
                assert_eq!(result.stdout, exported(&variants[0], &expected));
                assert!(result.stderr.is_empty());
                if case == "ordinary" && choose {
                    normal_input = Some(input.clone());
                }
                cases.push(json!({"name":label,"policy":standard,"input":input,"input_sha256":pin(&input),"stdout_hex":String::from_utf8(result.stdout).unwrap(),"exit":0,"jit_work":report.work}));
            }
        }
        let input = normal_input.unwrap();
        let account = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let imported = import_inputs(&variants[0], &input, &account);
        let input_bytes = account.used_bytes();
        assert!(input_bytes > 0);
        let plan = jit.descriptor().plan();
        // Admit the input AND the native word transport; deny the following
        // managed Value arena inside evaluate_into, not the shell's importer.
        let words = plan.inputs().len()
            + plan.values().len()
            + 3 * plan.iterations().len()
            + plan.outputs().len()
            + 1;
        let mut native_memory_limit = input_bytes + words as u64 * 8;
        drop(imported);
        assert_eq!(account.used_bytes(), 0);
        let mut native_denial = None;
        let mut transport_admission = Vec::new();
        // Native transport includes managed plan metadata, not just payload.
        // Follow its structured required bytes until transport is admitted;
        // never guess a platform constant or accept a successful evaluation.
        for _ in 0..16 {
            let account = ManagedMemoryBudget::new(native_memory_limit);
            let imported = import_inputs(&variants[0], &input, &account);
            assert_eq!(account.used_bytes(), input_bytes);
            let mut denied_outputs = vec![None; plan.outputs().len()];
            let denial = jit
                .evaluate_into(&imported, &mut denied_outputs, limits(), &account)
                .unwrap_err();
            assert!(denied_outputs.iter().all(Option::is_none));
            assert_eq!(account.used_bytes(), input_bytes);
            match &denial {
                NativeProgramExecutionError::Base(NativeExecutionError::Memory {
                    error: mech_core::MemoryRuntimeError::BudgetExceeded { requested, .. },
                }) => {
                    assert!(*requested > native_memory_limit);
                    transport_admission
                        .push(json!({"limit":native_memory_limit,"required":requested}));
                    native_memory_limit = *requested;
                }
                NativeProgramExecutionError::Value(NativeValueError::Base(
                    NativeExecutionError::Memory {
                        error: mech_core::MemoryRuntimeError::BudgetExceeded { .. },
                    },
                )) => {
                    native_denial = Some(denial.to_string());
                    break;
                }
                _ => panic!("expected native transport/arena memory admission: {denial}"),
            }
        }
        let denial = native_denial.expect("native Value arena denial was not exercised");
        let selected = policy(
            1_000_000_000,
            native_memory_limit,
            134_217_728,
            128 * 1024 * 1024,
        );
        let result = bounded(
            Command::new(&binary).args(&selected).arg(&input),
            &directory,
            "native-arena-memory-denial",
            30,
        );
        assert!(!result.status.success());
        assert!(result.stdout.is_empty());
        assert_eq!(result.stderr, format!("{denial}\n").as_bytes());
        cases.push(json!({"name":"native-arena-memory-denial","phase":"linked.evaluate_into managed Value arena admission","policy":selected,"input":input,"input_admitted_bytes":input_bytes,"word_transport_payload_bytes":words * 8,"transport_admission":transport_admission,"memory_limit":native_memory_limit,"stdout":"","stderr":String::from_utf8(result.stderr).unwrap(),"exit":result.status.code()}));
        for (label, selected, diagnostic) in [
            (
                "work-denial",
                policy(0, 256 * 1024 * 1024, 134_217_728, 128 * 1024 * 1024),
                "Work",
            ),
            (
                "input-import-memory-denial",
                policy(1_000_000_000, 0, 134_217_728, 128 * 1024 * 1024),
                "BudgetExceeded",
            ),
            (
                "call-work-denial",
                policy(1_000_000_000, 256 * 1024 * 1024, 0, 128 * 1024 * 1024),
                "Work",
            ),
            (
                "call-temporary-denial",
                policy(1_000_000_000, 256 * 1024 * 1024, 134_217_728, 0),
                "Temporary",
            ),
        ] {
            let result = bounded(
                Command::new(&binary).args(&selected).arg(&input),
                &directory,
                label,
                30,
            );
            assert!(!result.status.success());
            assert!(result.stdout.is_empty());
            assert!(
                String::from_utf8_lossy(&result.stderr).contains(diagnostic),
                "{label}: {}",
                String::from_utf8_lossy(&result.stderr)
            );
            cases.push(json!({"name":label,"policy":selected,"input":input,"stdout":"","stderr":String::from_utf8(result.stderr).unwrap(),"exit":result.status.code()}));
        }
        for (label, bytes, diagnostic) in [
            ("invalid-transport", b"not JSON".as_slice(), "expected"),
            ("missing-input", b"[]".as_slice(), "input count"),
        ] {
            let malformed = directory.join(format!("{label}-input.json"));
            fs::write(&malformed, bytes).unwrap();
            let result = bounded(
                Command::new(&binary).args(&standard).arg(&malformed),
                &directory,
                label,
                30,
            );
            assert!(!result.status.success());
            assert!(result.stdout.is_empty());
            assert!(String::from_utf8_lossy(&result.stderr).contains(diagnostic));
            cases.push(json!({"name":label,"policy":standard,"input":malformed,"stdout":"","stderr":String::from_utf8(result.stderr).unwrap(),"exit":result.status.code()}));
        }
        let mut wrong_kind: Value = serde_json::from_slice(&fs::read(&input).unwrap()).unwrap();
        let ordinal = variants[0]
            .inputs()
            .iter()
            .position(|input| {
                mech_engine::decode_source_input_name(&input.name).unwrap() == input_name
            })
            .unwrap();
        wrong_kind[ordinal]["data"] = json!({"U64":7});
        let malformed = directory.join("wrong-kind-input.json");
        fs::write(&malformed, serde_json::to_vec(&wrong_kind).unwrap()).unwrap();
        let result = bounded(
            Command::new(&binary).args(&standard).arg(&malformed),
            &directory,
            "wrong-input-kind",
            30,
        );
        assert!(!result.status.success());
        assert!(result.stdout.is_empty());
        assert!(!result.stderr.is_empty());
        cases.push(json!({"name":"wrong-input-kind","policy":standard,"input":malformed,"stdout":"","stderr":String::from_utf8(result.stderr).unwrap(),"exit":result.status.code()}));
        if name != "lexical" {
            let named = [
                (
                    input_name,
                    ValueDataDraft::String(if name == "collection" {
                        "bb\na".into()
                    } else {
                        "a\nbb\na".into()
                    }),
                ),
                (
                    "divisor",
                    ValueDataDraft::U64(u64::from(name == "collection")),
                ),
                ("width", ValueDataDraft::U64(2)),
                ("stop", ValueDataDraft::U64(6)),
                ("increment", ValueDataDraft::U64(2)),
            ];
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let args = inputs(&variants[0], &named, &memory);
            for artifact in &variants {
                let account = ManagedMemoryBudget::new(256 * 1024 * 1024);
                let values = inputs(artifact, &named, &account);
                assert!(reference_result(artifact, &values).is_err());
            }
            let mut output = vec![None; variants[0].outputs().len()];
            let expected = jit
                .evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap_err()
                .to_string()
                + "\n";
            assert!(output.iter().all(Option::is_none));
            let input = input_file(&directory, "late-arithmetic", &args);
            let result = bounded(
                Command::new(&binary).args(&standard).arg(&input),
                &directory,
                "late-arithmetic",
                30,
            );
            assert!(!result.status.success());
            assert!(result.stdout.is_empty());
            assert_eq!(result.stderr, expected.as_bytes());
            cases.push(json!({"name":"late-arithmetic","policy":standard,"input":input,"stdout":"","stderr":expected,"exit":result.status.code()}));
        }
        if name == "windows" {
            let named = [
                (input_name, ValueDataDraft::String("ABBA".into())),
                ("width", ValueDataDraft::U64(2)),
                ("stop", ValueDataDraft::U64(6)),
                ("increment", ValueDataDraft::U64(0)),
                ("divisor", ValueDataDraft::U64(1)),
            ];
            let account = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let args = inputs(&variants[0], &named, &account);
            for artifact in &variants {
                let account = ManagedMemoryBudget::new(256 * 1024 * 1024);
                let values = inputs(artifact, &named, &account);
                assert!(
                    reference_result(artifact, &values)
                        .unwrap_err()
                        .contains("InvalidInput")
                );
            }
            let mut output = vec![None; variants[0].outputs().len()];
            let expected = jit
                .evaluate_into(&args, &mut output, limits(), &account)
                .unwrap_err()
                .to_string()
                + "\n";
            assert!(expected.contains("InvalidInput"));
            assert!(output.iter().all(Option::is_none));
            let input = input_file(&directory, "invalid-range-step", &args);
            let result = bounded(
                Command::new(&binary).args(&standard).arg(&input),
                &directory,
                "invalid-range-step",
                30,
            );
            assert!(!result.status.success());
            assert!(result.stdout.is_empty());
            assert_eq!(result.stderr, expected.as_bytes());
            cases.push(json!({"name":"invalid-range-step","policy":standard,"input":input,"stdout":"","stderr":expected,"exit":result.status.code()}));
        }
        let provenance_build = (name == "collection").then(|| {
            provenance::qualify(
                &generated,
                &binary_name,
                &binary,
                &evidence.join("cargo-target"),
                &directory,
                false,
            )
        });
        let result = bounded(
            Command::new(&binary).args(&standard).arg(&input),
            &directory,
            "recovery",
            30,
        );
        assert!(result.status.success());
        assert!(result.stderr.is_empty());
        let ordinary = cases
            .iter()
            .find(|case| case["name"] == "ordinary-true")
            .unwrap();
        assert_eq!(
            result.stdout,
            ordinary["stdout_hex"].as_str().unwrap().as_bytes()
        );
        cases.push(json!({"name":"fresh-process-recovery","stdout_hex":String::from_utf8(result.stdout).unwrap(),"exit":0}));
        for (path, expected) in &pins {
            assert_eq!(&pin(path), expected);
        }
        assert_eq!(pin(&binary), binary_pin);
        receipts.push(json!({"name":name,"source":source,"source_sha256":hash(source.as_bytes()),"generator":generated.metadata(),"generated_pins":pins,"binary":binary,"binary_sha256":binary_pin,"lock_sha256":pin(&generated.root().join("Cargo.lock")),"compiled_units":units,"cases":cases,"provenance_build":provenance_build,"linked":true,"executed":true,"timing_samples":0}));
    }
    clean(&evidence, &commit, "after");
    fs::write(evidence.join("qualification.json"), serde_json::to_vec_pretty(&json!({"schema":"mech.canonical-program-linked-qualification.v1","source_commit":commit,"programs":receipts,"scope":"managed root and lexical calls, scalar control and native String matrix comprehensions in real linked Objects; no resource-host/full-workload/performance claim","benchmark_measurements":0})).unwrap()).unwrap();
}
