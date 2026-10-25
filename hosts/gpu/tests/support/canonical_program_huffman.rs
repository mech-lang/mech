//! The DEFLATE Huffman builder is implemented by the included Mech program.
use super::{artifacts, inputs, limits};
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_gpu::native_artifact::JitProgramArtifact;
const KERNEL: &str = include_str!("../fixtures/native/deflate-huffman.mec");
#[derive(serde::Deserialize)]
struct Reference {
    cases: Vec<HuffmanCase>,
}
#[derive(serde::Deserialize)]
struct HuffmanCase {
    name: String,
    num_symbols: u64,
    max_len: u64,
    frequencies: Vec<u32>,
    lengths: Vec<u8>,
    codewords: Vec<u32>,
}
#[test]
fn deflate_huffman_kernel_matches_the_pinned_c_builder() {
    let reference: Reference = serde_json::from_str(include_str!(
        "../fixtures/native/deflate-huffman-reference.json"
    ))
    .unwrap();
    assert_eq!(reference.cases.len(), 19);
    let source = format!(
        "{KERNEL}\n#MakeHuffman(frequencies<words>, symbols<u64>, max-length<u64>, scratch-counts<counters>, scratch-lengths<lengths>, scratch-words<words>, scratch-code-lengths<length-counts>)\n"
    );
    for (artifact_index, artifact) in artifacts(&source).0.into_iter().enumerate() {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(2 * 1024 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        let mut policy = limits();
        // Explicit finite port-qualification policy. No normal host limits are
        // changed, and these measurements are not full-compression timings.
        policy.execution.max_work = 100_000_000_000;
        policy.library.max_compute_work = Some(100_000_000_000);
        policy.library.max_comparison_work = Some(100_000_000);
        for case in &reference.cases {
            let mut freq = case
                .frequencies
                .iter()
                .copied()
                .map(Data::U32)
                .collect::<Vec<_>>();
            freq.resize(288, Data::U32(0));
            let args = inputs(
                &artifact,
                &[
                    ("frequencies", Data::Matrix(freq.into())),
                    ("symbols", Data::U64(case.num_symbols)),
                    ("max-length", Data::U64(case.max_len)),
                    (
                        "scratch-counts",
                        Data::Matrix(vec![Data::U64(0); 288].into()),
                    ),
                    (
                        "scratch-lengths",
                        Data::Matrix(vec![Data::U8(0); 288].into()),
                    ),
                    (
                        "scratch-words",
                        Data::Matrix(vec![Data::U32(0); 288].into()),
                    ),
                    (
                        "scratch-code-lengths",
                        Data::Matrix(vec![Data::U64(0); 17].into()),
                    ),
                ],
                &memory,
            );
            let started = std::time::Instant::now();
            let report = jit
                .evaluate_into(&args, &mut output, policy, &memory)
                .unwrap_or_else(|error| panic!("{}: {error:?}", case.name));
            let elapsed = started.elapsed();
            let Data::Tuple(result) = output
                .last()
                .unwrap()
                .as_ref()
                .unwrap()
                .canonical_data_draft()
                .unwrap()
            else {
                panic!()
            };
            let Data::Matrix(lens) = &result[0] else {
                panic!()
            };
            let Data::Matrix(words) = &result[1] else {
                panic!()
            };
            assert_eq!(
                lens.as_ref(),
                case.lengths
                    .iter()
                    .copied()
                    .map(Data::U8)
                    .collect::<Vec<_>>(),
                "{} lengths",
                case.name
            );
            for (symbol, (&length, &code)) in case.lengths.iter().zip(&case.codewords).enumerate() {
                // The ARM rbit32 branch preserves scratch bits for unused
                // symbols; libdeflate's portable table and Roc return zero.
                // Such symbols emit no bits. Every usable codeword is exact.
                let expected = if length == 0 { 0 } else { code };
                assert_eq!(
                    words[symbol],
                    Data::U32(expected),
                    "{} codeword {symbol}",
                    case.name
                );
            }
            eprintln!(
                "HUFFMAN_CASE artifact={artifact_index} name={} ns={} report={report:?}",
                case.name,
                elapsed.as_nanos()
            );
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn transition_tuple_destructuring_recomputes_bindings_on_every_visit() {
    let source = "pair(n<u64>) => (left<u64>, right<u64>) :=\n  left := n + 1u64\n  right := n + 2u64.\n#Sum(n<u64>) => <u64>\n  | :Loop(left<u64>, total<u64>)\n  | :Done(total<u64>).\n#Sum(n) -> :Loop(n, 0u64)\n  :Loop(left, total)\n    | left > 0u64 -> {\n        (first, second) := pair(left)\n      } -> :Loop(left - 1u64, total + first * 10u64 + second)\n    | * -> :Done(total)\n  :Done(total) => total.\n#Sum(input<u64>)\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for n in [0u64, 1, 4, 2, 8, 0] {
            let args = inputs(&artifact, &[("input", Data::U64(n))], &memory);
            let ordinary = super::reference(&artifact, &args);
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            super::equal(&artifact, &output, &ordinary);
            assert_eq!(
                output
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::U64(11 * n * (n + 1) / 2 + 12 * n)
            );
        }
    }
}

#[test]
fn transition_tuple_destructuring_keeps_maintained_diagnostics() {
    use mech_syntax::document::{
        AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
        parse_canonical_document,
    };
    for (statement, expected) in [
        (
            "(x, x) := (1u64, 2u64)",
            "source-semantics/variable-already-defined",
        ),
        (
            "(x, y, z) := (1u64, 2u64)",
            "source-semantics/destructure-arity",
        ),
        (
            "(x, y) := 1u64",
            "source-semantics/destructure-requires-tuple",
        ),
        (
            "x := 0u64\n      (x, y) := (1u64, 2u64)",
            "source-semantics/variable-already-defined",
        ),
    ] {
        let source = format!(
            "#Bad() => <u64>\n  | :Start\n  | :Done(result<u64>).\n#Bad() -> :Start\n  :Start -> {{\n      {statement}\n    }} -> :Done(0u64)\n  :Done(result) => result.\n#Bad()\n"
        );
        let parsed = parse_canonical_document(
            TextSnapshot::new(DocumentId(0x48554646), Revision(0), source.as_str()).unwrap(),
            ParseConfig::default(),
        );
        assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
        let document = DocumentSyntax::cast(parsed.syntax()).unwrap();
        let error = match mech_engine::CanonicalSourceFrontend
            .compile_document_with_catalog(&document, mech_stdlib::source_catalog())
        {
            Ok(_) => panic!("invalid tuple binding was accepted: {statement}"),
            Err(error) => error,
        };
        assert_eq!(error.code, expected, "{statement}: {error:?}");
    }
}

#[test]
fn transition_tuple_rhs_keeps_unused_element_errors_atomic_and_recovers() {
    let source = "#Partial(n<u64>) => <u64>\n  | :Start(n<u64>)\n  | :Done(result<u64>).\n#Partial(n) -> :Start(n)\n  :Start(n) -> {\n      (first, second) := (1u64, 2u64, 9u64 / n)\n    } -> :Done(first + second)\n  :Done(result) => result.\n#Partial(input<u64>)\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for n in [1u64, 0, 3, 2] {
            let args = inputs(&artifact, &[("input", Data::U64(n))], &memory);
            if n == 0 {
                assert!(super::reference_result(&artifact, &args).is_err());
                let prior = output
                    .iter()
                    .map(|value| value.clone().unwrap())
                    .collect::<Vec<_>>();
                let before = memory.used_bytes();
                assert!(
                    jit.evaluate_into(&args, &mut output, limits(), &memory)
                        .is_err()
                );
                super::equal(&artifact, &output, &prior);
                assert_eq!(memory.used_bytes(), before);
            } else {
                let ordinary = super::reference(&artifact, &args);
                jit.evaluate_into(&args, &mut output, limits(), &memory)
                    .unwrap();
                super::equal(&artifact, &output, &ordinary);
                assert_eq!(
                    output
                        .last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    Data::U64(3)
                );
            }
        }
    }
}

#[test]
fn nested_fsm_local_definitions_shadow_and_restore_the_callers_names() {
    let source = "#Inner(x<u64>) => <u64>\n  | :Start(x<u64>)\n  | :Done(value<u64>).\n#Inner(x) -> :Start(x)\n  :Start(x) -> {\n      ~temporary := x\n      temporary += 1u64\n    } -> :Done(temporary)\n  :Done(value) => value.\n#Outer(x<u64>) => <u64>\n  | :Start(x<u64>)\n  | :Done(value<u64>).\n#Outer(x) -> :Start(x)\n  :Start(x) -> {\n      ~temporary := x + 10u64\n      nested := #Inner(temporary)\n      temporary += nested\n    } -> :Done(temporary)\n  :Done(value) => value.\n#Outer(input<u64>)\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for n in [0u64, 3, 1, 8] {
            let args = inputs(&artifact, &[("input", Data::U64(n))], &memory);
            let ordinary = super::reference(&artifact, &args);
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            super::equal(&artifact, &output, &ordinary);
            assert_eq!(
                output
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::U64(2 * n + 21)
            );
        }
    }
}

#[test]
fn constant_tuple_reads_preserve_heterogeneous_fields_and_image_metadata() {
    use mech_gpu::native_artifact::NativeProgramDescriptor;
    let source = "(first, second, third) := (flag<bool>, payload<[u32]:1,4>, count<u64>)\n(first, second, third)\n";
    for artifact in artifacts(source).0 {
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let image = jit.descriptor().encode_build_owned_image().unwrap();
        // SAFETY: inspect an unchanged descriptor generated by this checked
        // compilation, without pairing it with a foreign executable.
        let restored =
            unsafe { NativeProgramDescriptor::decode_build_owned_image(&image, &catalog) }.unwrap();
        assert_eq!(restored.encode_build_owned_image().unwrap(), image);
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for (flag, count) in [(false, 0), (true, u64::MAX), (false, 3)] {
            let payload = Data::Matrix(
                vec![
                    Data::U32(0),
                    Data::U32(u32::MAX),
                    Data::U32(37),
                    Data::U32(1),
                ]
                .into(),
            );
            let args = inputs(
                &artifact,
                &[
                    ("flag", Data::Bool(flag)),
                    ("payload", payload.clone()),
                    ("count", Data::U64(count)),
                ],
                &memory,
            );
            let ordinary = super::reference(&artifact, &args);
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            super::equal(&artifact, &output, &ordinary);
            assert_eq!(
                output
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::Tuple(vec![Data::Bool(flag), payload, Data::U64(count)].into())
            );
        }
    }
}

#[test]
fn tuple_projection_requires_the_real_access_factory_and_constant_ordinal() {
    use mech_engine::native_artifact::{
        NativeArtifactLoweringError as E, lower_native_program_artifact,
    };
    let mut builder = mech_core::FunctionCatalogBuilder::new();
    for (key, entry) in mech_stdlib::source_catalog().resident_factories() {
        if key.module_path.as_ref() != ["access"] || key.operation_name != "scalar" {
            builder
                .insert_resident_factory_entry(entry.clone())
                .unwrap();
        }
    }
    builder
        .insert_resident_factory(
            ["access"],
            "scalar",
            mech_core::ImplementationMemoryClass::CanonicalFinalize,
            |_| panic!("validation must not execute the replacement factory"),
        )
        .unwrap();
    let replacement = builder.build().unwrap();
    for artifact in artifacts("pair := (input<u64>, 7u64)\npair[1u64]\n").0 {
        assert!(matches!(
            lower_native_program_artifact(&artifact, &replacement),
            Err(E::MissingSemanticWitness { .. })
        ));
    }
    for artifact in artifacts("pair := (input<u64>, 7u64)\npair[position<u64>]\n").0 {
        assert!(lower_native_program_artifact(&artifact, &mech_stdlib::source_catalog()).is_err());
    }
}

#[test]
fn full_huffman_resident_denial_preserves_the_real_resource_budget_error() {
    let source = format!(
        "{KERNEL}\n(run<bool>? | true => #MakeHuffman(frequencies<words>, symbols<u64>, max-length<u64>, scratch-counts<counters>, scratch-lengths<lengths>, scratch-words<words>, scratch-code-lengths<length-counts>) | false => (scratch-lengths<lengths>, scratch-words<words>))\n"
    );
    let (artifacts, last) = artifacts(&source);
    for artifact in artifacts {
        let memory = ManagedMemoryBudget::new(512 * 1024 * 1024);
        let mut freqs = vec![Data::U32(0); 288];
        freqs[0] = Data::U32(1);
        freqs[1] = Data::U32(1);
        let mut values = vec![
            ("run", Data::Bool(false)),
            ("frequencies", Data::Matrix(freqs.into())),
            ("symbols", Data::U64(2)),
            ("max-length", Data::U64(15)),
            (
                "scratch-counts",
                Data::Matrix(vec![Data::U64(0); 288].into()),
            ),
            (
                "scratch-lengths",
                Data::Matrix(vec![Data::U8(0); 288].into()),
            ),
            (
                "scratch-words",
                Data::Matrix(vec![Data::U32(0); 288].into()),
            ),
            (
                "scratch-code-lengths",
                Data::Matrix(vec![Data::U64(0); 17].into()),
            ),
        ];
        let args = inputs(&artifact, &values, &memory);
        let mut instance = mech_engine::resident::activate(
            mech_core::ReactiveInstanceId::new(0x48554646, 0),
            &artifact,
            &mech_stdlib::source_catalog(),
            &mech_engine::resident::ActivationFacts::default(),
        )
        .unwrap();
        let captured = instance
            .plan
            .inputs
            .iter()
            .map(|input| mech_engine::resident::CapturedValueInput {
                slot: input.slot,
                value: &args[artifact
                    .inputs()
                    .iter()
                    .position(|v| v.slot == input.artifact_slot)
                    .unwrap()],
            })
            .collect::<Vec<_>>();
        instance
            .prepare_turn_values(&captured)
            .unwrap()
            .publish()
            .unwrap();
        let accepted = instance
            .copied_output(last)
            .unwrap()
            .canonical_data_draft()
            .unwrap();
        assert_eq!(
            accepted,
            Data::Tuple(
                vec![
                    Data::Matrix(vec![Data::U8(0); 288].into()),
                    Data::Matrix(vec![Data::U32(0); 288].into()),
                ]
                .into()
            )
        );
        values[0].1 = Data::Bool(true);
        let args = inputs(&artifact, &values, &memory);
        let epoch = instance.published_epoch();
        let hash = instance.published_state_hash();
        for _ in 0..2 {
            let captured = instance
                .plan
                .inputs
                .iter()
                .map(|input| mech_engine::resident::CapturedValueInput {
                    slot: input.slot,
                    value: &args[artifact
                        .inputs()
                        .iter()
                        .position(|v| v.slot == input.artifact_slot)
                        .unwrap()],
                })
                .collect::<Vec<_>>();
            let error = match instance.prepare_turn_values(&captured) {
                Ok(_) => panic!("the default resident whole-control budget must remain enforced"),
                Err(error) => error,
            };
            // Packed aggregate projections now prepay their draft expansion.
            // With the unchanged default limits, compute admission is reached
            // before comparison admission. Keep the precise resource error,
            // rather than reclassifying it as InvalidOutput or lifting a cap.
            assert!(
                matches!(error, mech_engine::resident::ResidentExecutionError::Kernel {
                error: mech_core::ResidentKernelError::ResourceLimit {
                    dimension: mech_core::MemoryBudgetDimension::ComputeWork, required, limit, ..
                }, ..
            } if required > limit && limit == mech_core::RESIDENT_MAX_COMPUTE_WORK),
                "{error:?}"
            );
            assert_eq!(instance.published_epoch(), epoch);
            assert_eq!(instance.published_state_hash(), hash);
            assert_eq!(
                instance
                    .copied_output(last)
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                accepted
            );
        }
    }
}

#[test]
fn minimal_resident_nested_tuple_return_preserves_the_buffer() {
    let definitions = "<buffer> := <[u64]:1,4>\n#Pair(xs<buffer>, n<u64>) => <(buffer,u64)>\n  | :Loop(xs<buffer>, left<u64>)\n  | :Done(xs<buffer>).\n#Pair(xs, n) -> :Loop(xs, n)\n  :Loop(xs, left)\n    | left > 0u64 -> {\n        ~next := xs\n        next[1u64] = xs[1u64] + left\n      } -> :Loop(next, left - 1u64)\n    | * -> :Done(xs)\n  :Done(xs) => (xs, 7u64).\n#Project(xs<buffer>, n<u64>) => <buffer>\n  | :Start(xs<buffer>, n<u64>)\n  | :Done(xs<buffer>).\n#Project(xs, n) -> :Start(xs, n)\n  :Start(xs, n) -> {\n      (result, unused) := #Pair(xs, n)\n    } -> :Done(result)\n  :Done(xs) => xs.\n";
    for entry in [
        "#Pair(input<buffer>, count<u64>)",
        "#Project(input<buffer>, count<u64>)",
    ] {
        let source = format!("{definitions}\n{entry}\n");
        for artifact in artifacts(&source).0 {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            for n in [0u64, 1, 2, 0] {
                let args = inputs(
                    &artifact,
                    &[
                        (
                            "input",
                            Data::Matrix(
                                vec![Data::U64(1), Data::U64(2), Data::U64(3), Data::U64(4)].into(),
                            ),
                        ),
                        ("count", Data::U64(n)),
                    ],
                    &memory,
                );
                let result = super::reference_result(&artifact, &args)
                    .unwrap_or_else(|error| panic!("entry={entry} n={n}: {error}"));
                let array = Data::Matrix(
                    vec![
                        Data::U64(1 + n * (n + 1) / 2),
                        Data::U64(2),
                        Data::U64(3),
                        Data::U64(4),
                    ]
                    .into(),
                );
                let expected = if entry.starts_with("#Pair") {
                    Data::Tuple(vec![array, Data::U64(7)].into())
                } else {
                    array
                };
                assert_eq!(
                    result.last().unwrap().canonical_data_draft().unwrap(),
                    expected
                );
            }
        }
    }
}
