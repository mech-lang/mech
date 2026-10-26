//! Verified tail transitions use bounded invocation frames, with ordinary artifacts.
#![cfg(feature = "full_compiler")]

use mech_core::{ReactiveInstanceId, ResidentKernelError, ResidentValueRef, Value, ValueData};
use mech_engine::resident::{
    ActivationFacts, CapturedSignalInput, ReactiveInstance, ResidentActivationOptions,
    ResidentExecutionError, activate, activate_with_options,
};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact, SourceDocumentOutputKind};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

fn compile(source: &str) -> ([ProgramArtifact; 2], usize) {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x5441494c), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(
        parsed.is_strictly_clean(),
        "{source}: {:?}",
        parsed.diagnostics
    );
    let program = CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            mech_stdlib::source_catalog(),
        )
        .unwrap_or_else(|error| panic!("{source}: {error:?}"));
    let output = program
        .document_outputs()
        .iter()
        .rev()
        .find(|output| output.kind == SourceDocumentOutputKind::Program)
        .unwrap()
        .output as usize;
    let original = program.compile_artifact().unwrap();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    assert_eq!(original.revision(), decoded.revision());
    assert_eq!(
        bytes,
        mech_engine::encode_program_artifact_bytecode_v1(&decoded).unwrap()
    );
    ([original, decoded], output)
}

fn instance(artifact: &ProgramArtifact, ordinal: usize) -> ReactiveInstance {
    activate(
        ReactiveInstanceId::new(0x5441494c, ordinal as u32),
        artifact,
        &mech_stdlib::source_catalog(),
        &ActivationFacts::default(),
    )
    .unwrap()
}

fn turn(
    instance: &mut ReactiveInstance,
    value: ResidentValueRef<'_>,
) -> Result<(), ResidentExecutionError> {
    assert_eq!(instance.plan.inputs.len(), 1);
    let slot = instance.plan.inputs[0].slot;
    instance
        .turn(&[CapturedSignalInput { slot, value }])
        .map(|_| ())
}

fn u64_input(artifact: &ProgramArtifact, n: u64) -> Value {
    let declared = artifact
        .inputs()
        .iter()
        .find(|input| {
            matches!(
                artifact.schemas().get(input.schema).unwrap().body(),
                mech_core::SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64)
            )
        })
        .unwrap();
    mech_core::ValueDraft {
        schema: declared.schema,
        shape_values: Box::new([]),
        data: mech_core::ValueDataDraft::U64(n),
    }
    .finalize(&mech_core::snapshot::SnapshotValidationContext::new(
        artifact.schemas(),
    ))
    .unwrap()
}

fn u64_output(instance: &ReactiveInstance, output: usize, expected: u64) {
    let value = instance.copied_output(output).unwrap();
    assert!(
        matches!(value.data(), ValueData::U64(actual) if *actual == expected),
        "{value:?}"
    );
}

fn f64_output(instance: &ReactiveInstance, output: usize, expected: f64) {
    let value = instance.copied_output(output).unwrap();
    assert!(
        matches!(value.data(), ValueData::F64(actual) if actual.to_f64() == expected),
        "{value:?}"
    );
}

const COUNTDOWN: &str = "#Count(count<u64>) => <u64>\n  | :Step(left<u64>, total<u64>)\n  | :Done(total<u64>).\n#Count(count) -> :Step(count, 0u64)\n  :Step(left, total)\n    | left == 0u64 -> :Done(total)\n    | * -> :Step(left - 1u64, total + 1u64)\n  :Done(total) => total.\n#Count(signal<u64>)\n";

#[test]
fn synchronous_state_transitions_exceed_the_non_tail_frame_ceiling_and_refresh() {
    let (artifacts, output) = compile(COUNTDOWN);
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = instance(artifact, ordinal);
        for n in [0, 1, 24, 32, 64, 256, 1024, 3] {
            let value = u64_input(artifact, n);
            turn(&mut instance, ResidentValueRef::Snapshot(&[Some(value)]))
                .unwrap_or_else(|error| panic!("artifact {ordinal}, n={n}: {error:?}"));
            assert!(!instance.has_ready_continuation());
            u64_output(&instance, output, n);
        }
    }
}

#[test]
fn terminal_nested_matches_forward_to_the_exact_outer_recursive_target() {
    // This retained source form already has an ordinary validated Recur node.
    // The extra match is terminal; its scrutinee is not the recursive target.
    let source = "walk(n<f64>) => <f64>\n  | 0 => 0\n  | n => ((n > 0) ? | true => walk(n - 1) | * => 0).\nwalk(signal<f64>)\n";
    let (artifacts, output) = compile(source);
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = instance(artifact, ordinal);
        for n in [128.0, 0.0, 65.0, 1.0] {
            turn(&mut instance, ResidentValueRef::F64(&[n])).unwrap();
            f64_output(&instance, output, 0.0);
        }
    }
}

#[test]
fn non_tail_recursive_postprocessing_keeps_the_existing_depth_limit_and_recovers() {
    let source = "factorial(n<f64>) => <f64>\n  | 0 => 1\n  | n => n * factorial(n - 1).\nfactorial(signal<f64>)\n";
    let (artifacts, output) = compile(source);
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = instance(artifact, ordinal);
        turn(&mut instance, ResidentValueRef::F64(&[5.0])).unwrap();
        f64_output(&instance, output, 120.0);
        let epoch = instance.published_epoch();
        let before = instance.copied_output(output).unwrap();
        let error = turn(&mut instance, ResidentValueRef::F64(&[32.0])).unwrap_err();
        assert!(
            matches!(
                error,
                ResidentExecutionError::Kernel {
                    error: ResidentKernelError::InvalidShape,
                    ..
                }
            ),
            "{error:?}"
        );
        assert_eq!(instance.published_epoch(), epoch);
        assert_eq!(
            instance
                .copied_output(output)
                .unwrap()
                .canonical_data_draft()
                .unwrap(),
            before.canonical_data_draft().unwrap()
        );
        turn(&mut instance, ResidentValueRef::F64(&[4.0])).unwrap();
        f64_output(&instance, output, 24.0);
    }
}

const LATE_ERROR: &str = "#Checked(count<u64>) => <u64>\n  | :Step(left<u64>, total<u64>)\n  | :Done(total<u64>).\n#Checked(count) -> :Step(count, 0u64)\n  :Step(left, total)\n    | left == 0u64 -> :Done(total)\n    | * -> denominator := ((count >= 64u64)? | true => (left? | 1u64 => 0u64 | * => 1u64) | false => 1u64)\n      -> :Step(left - 1u64, total + 1u64 / denominator)\n  :Done(total) => total.\n#Checked(signal<u64>)\n";

#[test]
fn a_late_checked_error_after_many_backedges_preserves_publication_and_recovers() {
    let (artifacts, output) = compile(LATE_ERROR);
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = instance(artifact, ordinal);
        let run = |instance: &mut ReactiveInstance, n| {
            let value = u64_input(artifact, n);
            turn(instance, ResidentValueRef::Snapshot(&[Some(value)]))
        };
        run(&mut instance, 3u64).unwrap();
        u64_output(&instance, output, 3);
        let epoch = instance.published_epoch();
        let previous = instance.copied_output(output).unwrap();
        let error = run(&mut instance, 64u64).unwrap_err();
        assert!(
            matches!(
                error,
                ResidentExecutionError::Kernel {
                    error: ResidentKernelError::Arithmetic,
                    ..
                }
            ),
            "{error:?}"
        );
        assert_eq!(instance.published_epoch(), epoch);
        assert_eq!(
            instance
                .copied_output(output)
                .unwrap()
                .canonical_data_draft()
                .unwrap(),
            previous.canonical_data_draft().unwrap()
        );
        run(&mut instance, 4u64).unwrap();
        u64_output(&instance, output, 4);
    }
}

#[test]
fn actual_record_bounds_machine_scans_long_fasta_prefix_and_body_in_source_order() {
    const FIXTURE: &str =
        include_str!("../../../hosts/gpu/tests/fixtures/native/knucleotide-idiomatic.mec");
    let (prefix, _) = FIXTURE.split_once("uppercase :=").unwrap();
    // Preserve the fixture's actual functions and machine. Replace only its
    // unavailable host imports with an ordinary live String input for this test.
    let mut source = prefix
        .lines()
        .filter(|line| !line.starts_with("+> @"))
        .collect::<Vec<_>>()
        .join("\n");
    source.push_str(
        "\nlines := text/lines(signal<string>)\n#RecordBounds(lines, sequence/length(lines))\n",
    );
    let (artifacts, output) = compile(&source);
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = instance(artifact, ordinal);
        for (prefix, body) in [(0usize, 0usize), (24, 32), (64, 64), (128, 96), (1, 2)] {
            let text = format!(
                "{}>THREE chosen\n{}>FOUR stop\nignored",
                "ignored\n".repeat(prefix),
                "acgt\n".repeat(body)
            );
            turn(&mut instance, ResidentValueRef::String(&[text])).unwrap_or_else(|error| {
                panic!("artifact {ordinal}, prefix={prefix}, body={body}: {error:?}")
            });
            let value = instance.copied_output(output).unwrap();
            let ValueData::Tuple(parts) = value.data() else {
                panic!("{value:?}")
            };
            assert!(matches!(&parts[0], ValueData::U64(first) if *first == (prefix + 2) as u64));
            assert!(matches!(&parts[1], ValueData::U64(end) if *end == (prefix + body + 2) as u64));
        }
    }
}

#[test]
fn suspension_stays_a_turn_boundary_and_retains_frozen_arguments_beside_live_values() {
    let source = "+> text\n+> sequence\n#Capture(items<[string]>) => <(string,u64,u64)>\n  | :Step(left<u64>)\n  | :Wait\n  | :Later.\n#Capture(items) -> :Step(64u64)\n  :Step(left)\n    | left == 0u64 -> :Wait\n    | * -> :Step(left - 1u64)\n  :Wait ~> :Later\n  :Later => (items[2u64], sequence/length(items), sequence/length(text/lines(signal<string>))).\n#Capture(text/lines(signal<string>))\n";
    let (artifacts, output) = compile(source);
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = instance(artifact, ordinal);
        for (old, fresh, expected, old_len, new_len) in [
            ("a\né", "new", "é", 2, 1),
            ("a\nkept\nx", "z\ny\nx\nw", "kept", 3, 4),
        ] {
            turn(&mut instance, ResidentValueRef::String(&[old.to_owned()])).unwrap();
            assert!(instance.has_ready_continuation());
            turn(&mut instance, ResidentValueRef::String(&[fresh.to_owned()])).unwrap();
            assert!(!instance.has_ready_continuation());
            let value = instance.copied_output(output).unwrap();
            let ValueData::Tuple(parts) = value.data() else {
                panic!("{value:?}")
            };
            assert!(matches!(&parts[0], ValueData::String(text) if text.as_ref() == expected));
            assert!(matches!(&parts[1], ValueData::U64(n) if *n == old_len));
            assert!(matches!(&parts[2], ValueData::U64(n) if *n == new_len));
        }
    }
}

#[test]
fn configured_managed_budget_denial_preserves_a_long_walk_publication_and_recovers() {
    let (artifacts, output) = compile(COUNTDOWN);
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let budget = mech_core::ManagedMemoryBudget::new(8 * 1024 * 1024);
        let mut instance = activate_with_options(
            ReactiveInstanceId::new(0x5441494d, ordinal as u32),
            artifact,
            &mech_stdlib::source_catalog(),
            &ActivationFacts::default(),
            ResidentActivationOptions {
                memory_budget: Some(budget.clone()),
                ..Default::default()
            },
        )
        .unwrap();
        let run = |instance: &mut ReactiveInstance, n: u64| {
            let value = u64_input(artifact, n);
            turn(instance, ResidentValueRef::Snapshot(&[Some(value)]))
        };
        run(&mut instance, 64).unwrap();
        u64_output(&instance, output, 64);
        let epoch = instance.published_epoch();
        let previous = instance.copied_output(output).unwrap();
        // Hold the remaining configured account capacity. The next admitted
        // input/publication must fail without changing the completed result.
        // This tests the real managed path, not a claim that the temporary
        // tail carrier owns a separate physical reservation.
        let used = budget.used_bytes();
        let held = budget
            .reserve_capacity(budget.limit_bytes() - used)
            .unwrap();
        let error = run(&mut instance, 65).unwrap_err();
        assert!(
            matches!(
                error,
                ResidentExecutionError::MemoryRuntime {
                    error: mech_core::MemoryRuntimeError::BudgetExceeded { .. },
                }
            ),
            "{error:?}"
        );
        assert_eq!(instance.published_epoch(), epoch);
        assert_eq!(
            instance
                .copied_output(output)
                .unwrap()
                .canonical_data_draft()
                .unwrap(),
            previous.canonical_data_draft().unwrap()
        );
        drop(held);
        assert_eq!(
            budget.used_bytes(),
            used,
            "failed admission releases candidate backing"
        );
        run(&mut instance, 66).unwrap();
        u64_output(&instance, output, 66);
    }
}

#[test]
fn fixed_empty_matrix_carriers_and_zero_lane_results_keep_logical_tail_identity() {
    let source = "#Empty(seed<[f64]:0,2>, count<u64>) => <[f64]:0,2>\n  | :Step(value<[f64]:0,2>, left<u64>)\n  | :Done(value<[f64]:0,2>).\n#Empty(seed, count) -> :Step(seed, count)\n  :Step(value, left)\n    | left == 0u64 -> :Done(value)\n    | * -> :Step(value, left - 1u64)\n  :Done(value) => value.\n#Empty(empty<[f64]:0,2>, signal<u64>)\n";
    let (artifacts, output) = compile(source);
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = instance(artifact, ordinal);
        assert_eq!(instance.plan.inputs.len(), 2);
        for n in [0u64, 64, 256, 3] {
            let value = u64_input(artifact, n);
            let count = [Some(value)];
            let inputs = instance
                .plan
                .inputs
                .iter()
                .map(|input| CapturedSignalInput {
                    slot: input.slot,
                    value: match artifact.schemas().get(input.schema).unwrap().body() {
                        mech_core::SchemaBody::Matrix { .. } => ResidentValueRef::F64(&[]),
                        mech_core::SchemaBody::UnsignedInteger(_) => {
                            ResidentValueRef::Snapshot(&count)
                        }
                        schema => panic!("unexpected input {schema:?}"),
                    },
                })
                .collect::<Vec<_>>();
            instance.turn(&inputs).unwrap();
            let value = instance.copied_output(output).unwrap();
            let owner = value.schemas().unwrap();
            let mech_core::SchemaBody::Matrix { dimensions, .. } =
                owner.get(value.schema()).unwrap().body()
            else {
                panic!("{value:?}")
            };
            assert_eq!(
                dimensions
                    .iter()
                    .map(|axis| value.shape().resolve_dimension(axis).unwrap())
                    .collect::<Vec<_>>(),
                [0, 2]
            );
            assert_eq!(
                value.canonical_data_draft().unwrap(),
                mech_core::ValueDataDraft::Matrix(Box::new([]))
            );
        }
    }
}

#[test]
fn actual_full_ordinary_knucleotide_output_survives_long_fasta_prefixes() {
    const FIXTURE: &str =
        include_str!("../../../hosts/gpu/tests/fixtures/native/knucleotide-idiomatic.mec");
    let mut delivered = 0;
    let mut source = FIXTURE
        .lines()
        .filter(|line| !line.starts_with("+> @"))
        .map(|line| {
            if let Some(expression) = line.strip_prefix("@out/text <- ") {
                delivered += 1;
                format!("delivery-{delivered} := {expression}")
            } else {
                line.replace("@in/text", "signal<string>")
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(delivered, 1);
    source.push_str("\ndelivery-1\n");
    let expected = concat!(
        "G 50.000\nA 25.000\nT 25.000\n\n",
        "GG 33.333\nGT 33.333\nTA 33.333\n\n",
        "1\tGGT\n1\tGGTA\n0\tGGTATT\n0\tGGTATTTTAATT\n0\tGGTATTTTAATTTATAGT\n",
    );
    let (artifacts, output) = compile(&source);
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = instance(artifact, ordinal);
        for prefix in [24usize, 32, 64, 128, 0] {
            let text = format!(
                "{}>THREE description\ngG\nta\n>FOUR\nCC\n",
                "ignored\n".repeat(prefix)
            );
            turn(&mut instance, ResidentValueRef::String(&[text])).unwrap();
            let value = instance.copied_output(output).unwrap();
            let ValueData::String(actual) = value.data() else {
                panic!("{value:?}")
            };
            assert_eq!(actual.as_ref(), expected);
        }
    }
}

#[test]
fn nonempty_matrix_state_is_carried_before_locals_are_reused_on_each_tail_transition() {
    let source = "#Rotate(seed<[f64]:1,2>, count<u64>) => <[f64]:1,2>\n  | :Step(value<[f64]:1,2>, left<u64>)\n  | :Done(value<[f64]:1,2>).\n#Rotate(seed, count) -> :Step(seed, count)\n  :Step(value, left)\n    | left == 0u64 -> :Done(value)\n    | * -> :Step([value[2u64] value[1u64]], left - 1u64)\n  :Done(value) => value.\n#Rotate(values<[f64]:1,2>, signal<u64>)\n";
    let (artifacts, output) = compile(source);
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = instance(artifact, ordinal);
        for (n, pair, expected) in [
            (64u64, [10.0, 20.0], [10.0, 20.0]),
            (65, [30.0, 40.0], [40.0, 30.0]),
            (0, [50.0, 60.0], [50.0, 60.0]),
        ] {
            let count = [Some(u64_input(artifact, n))];
            let inputs = instance
                .plan
                .inputs
                .iter()
                .map(|input| CapturedSignalInput {
                    slot: input.slot,
                    value: match artifact.schemas().get(input.schema).unwrap().body() {
                        mech_core::SchemaBody::Matrix { .. } => ResidentValueRef::F64(&pair),
                        mech_core::SchemaBody::UnsignedInteger(_) => {
                            ResidentValueRef::Snapshot(&count)
                        }
                        schema => panic!("unexpected input {schema:?}"),
                    },
                })
                .collect::<Vec<_>>();
            instance.turn(&inputs).unwrap();
            assert_eq!(
                instance
                    .copied_output(output)
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                mech_core::ValueDataDraft::Matrix(
                    expected
                        .into_iter()
                        .map(|value| mech_core::ValueDataDraft::F64(
                            mech_core::snapshot::F64Bits::from_f64(value)
                        ))
                        .collect()
                )
            );
        }
    }
}
