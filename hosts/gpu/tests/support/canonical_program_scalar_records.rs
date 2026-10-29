//! Private named results use the maintained scalar aggregate word transport.
use super::{artifacts, equal, inputs, limits, reference, reference_result};
use mech_core::{ManagedMemoryBudget, SchemaBody, ValueDataDraft as Data};
use mech_engine::native_artifact::{
    NativeCollectionPrimitive as Primitive, NativeIterationAction as Action,
    NativeScalarAggregateAction as Aggregate,
};
use mech_gpu::native_artifact::{JitProgramArtifact, emit_program_object};

const CHOICE: &str = "#Pick(n<u64>, best<u32>, item<u32>, cp<u64>) => <{best<u32> item<u32> cache-ptr<u64>}>\n  | :Try(left<u64>, best<u32>, item<u32>, cp<u64>)\n  | :Done(best<u32>, item<u32>, cp<u64>).\n#Pick(n, best, item, cp) -> :Try(n, best, item, cp)\n  :Try(left, best, item, cp)\n    | left > 0u64 -> :Try(left - 1u64, item, best, cp + 1u64)\n    | * -> :Done(best, item, cp)\n  :Done(best, item, cp) => {best: best, item: item, cache-ptr: cp}.\nchoice := (branch<bool>? | true => #Pick(steps<u64>, best<u32>, item<u32>, cp<u64>) | false => #Pick(0u64, best<u32>, item<u32>, cp<u64>))\nleft<u64> := choice.best\nright<u64> := choice.item\nleft * 65537u64 + right + choice.cache-ptr\n";

fn selected_records(jit: &JitProgramArtifact) {
    let plan = jit.descriptor().plan();
    let aggregate = plan.scalar_aggregates();
    let mut packs = 0;
    let mut reads = 0;
    for (site, block) in plan.blocks().iter().enumerate() {
        let Action::LibraryCall {
            template, output, ..
        } = block.action()
        else {
            continue;
        };
        if matches!(template.primitive(), Primitive::CompositePack(_))
            && matches!(template.output().body(), SchemaBody::Record(_))
        {
            assert!(aggregate.slot(*output).is_some(), "record pack site {site}");
            assert!(matches!(
                aggregate.action(site),
                Some(Aggregate::Pack { .. })
            ));
            packs += 1;
        } else if template.primitive() == Primitive::RecordFieldAccess {
            assert!(
                matches!(aggregate.action(site), Some(Aggregate::Observe { .. })),
                "record read site {site}"
            );
            reads += 1;
        }
    }
    assert!(packs > 0 && reads >= 3);
}

#[test]
fn state_machine_record_results_use_words_through_copies_and_branch_merges() {
    for artifact in artifacts(CHOICE).0 {
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        selected_records(&jit);
        let object = emit_program_object(&artifact, &catalog).unwrap();
        assert_eq!(
            object.descriptor().typed_ir_sha256(),
            jit.descriptor().typed_ir_sha256()
        );
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for (steps, branch, best, item, cp) in [
            (0, true, 0xfedc_ba98, 19, 300),
            (1, true, 7, 0x8765_4321, 400),
            (7, false, 91, 113, 500),
            (2, true, 0xffff_ffff, 0x8000_0000, 600),
        ] {
            let args = inputs(
                &artifact,
                &[
                    ("steps", Data::U64(steps)),
                    ("branch", Data::Bool(branch)),
                    ("best", Data::U32(best)),
                    ("item", Data::U32(item)),
                    ("cp", Data::U64(cp)),
                ],
                &memory,
            );
            let aliases = args.clone();
            let mut policy = limits();
            // All selected construction/access is now native word transport.
            // The fixed frame and every ordinary control visit remain funded.
            policy.library.max_compute_work = Some(1);
            policy.library.max_comparison_work = Some(1);
            jit.evaluate_into(&args, &mut output, policy, &memory)
                .unwrap();
            equal(&artifact, &output, &reference(&artifact, &args));
            let n = if branch { steps } else { 0 };
            let (left, right) = if n % 2 == 0 {
                (best, item)
            } else {
                (item, best)
            };
            assert_eq!(
                output
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::U64(u64::from(left) * 65537 + u64::from(right) + cp + n)
            );
            assert!(
                args.iter()
                    .zip(&aliases)
                    .all(|(a, b)| a.shares_frozen_storage(b))
            );
        }
        drop(output);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn scalar_record_fields_keep_all_supported_bits_and_fresh_visits() {
    use mech_core::snapshot::F64Bits;
    let source = super::scalar_state_qualification::ALL_KINDS
        .replace("=> <(bool,u8,u16,u32,u64,i64,f64)>", "=> <{flag<bool> byte<u8> half<u16> word<u32> wide<u64> signed<i64> fraction<f64>}>")
        .replace("=> (flag, byte, half, word, wide, signed, fraction).", "=> {flag: flag, byte: byte, half: half, word: word, wide: wide, signed: signed, fraction: fraction}.")
        .replace("#Carry(steps<u64>, flag<bool>, byte<u8>, half<u16>, word<u32>, wide<u64>, signed<i64>, fraction<f64>)\n", "result := #Carry(steps<u64>, flag<bool>, byte<u8>, half<u16>, word<u32>, wide<u64>, signed<i64>, fraction<f64>)\n(result.flag, result.byte, result.half, result.word, result.wide, result.signed, result.fraction)\n");
    for artifact in artifacts(&source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        selected_records(&jit);
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        for (steps, flag, signed, bits) in [
            (0, true, i64::MIN, 0x8000_0000_0000_0000),
            (1, false, i64::MAX, 0x7ff8_0000_0000_0042),
            (2, true, -211, 0xfff8_0000_0000_0091),
        ] {
            let args = inputs(
                &artifact,
                &[
                    ("steps", Data::U64(steps)),
                    ("flag", Data::Bool(flag)),
                    ("byte", Data::U8(0xef)),
                    ("half", Data::U16(0xdcba)),
                    ("word", Data::U32(0xfedc_ba98)),
                    ("wide", Data::U64(u64::MAX)),
                    ("signed", Data::I64(signed)),
                    ("fraction", Data::F64(F64Bits::from_bits(bits))),
                ],
                &memory,
            );
            let mut output = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            equal(&artifact, &output, &reference(&artifact, &args));
            assert_eq!(
                output
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::Tuple(
                    vec![
                        Data::Bool(flag == (steps % 2 == 0)),
                        Data::U8(0xef),
                        Data::U16(0xdcba),
                        Data::U32(0xfedc_ba98),
                        Data::U64(u64::MAX),
                        Data::I64(signed),
                        Data::F64(F64Bits::from_bits(bits))
                    ]
                    .into()
                )
            );
        }
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn scalar_record_width_boundary_and_public_escape_keep_canonical_fallback() {
    for count in [32, 33] {
        let fields = (0..count)
            .map(|i| format!("f{i}: seed<u64> + {i}u64"))
            .collect::<Vec<_>>()
            .join(", ");
        let source = format!(
            "record := {{{fields}}}\nrecord.f0 + record.f{}\n",
            count - 1
        );
        for artifact in artifacts(&source).0 {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let plan = jit.descriptor().plan();
            let aggregates = plan.scalar_aggregates();
            let output = plan
                .blocks()
                .iter()
                .find_map(|b| match b.action() {
                    Action::LibraryCall {
                        template, output, ..
                    } if template.primitive() == Primitive::CompositePack(count) => Some(*output),
                    _ => None,
                })
                .unwrap();
            assert_eq!(aggregates.slot(output).is_some(), count == 32);
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let args = inputs(&artifact, &[("seed", Data::U64(17))], &memory);
            let mut output = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            equal(&artifact, &output, &reference(&artifact, &args));
            assert_eq!(
                output
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::U64(34 + count as u64 - 1)
            );
        }
    }
    for source in [
        "{best: best<u32>, item: item<u32>, cache-ptr: cp<u64>}\n",
        "record := {inner: {best: best<u32>, item: item<u32>}, cache-ptr: cp<u64>}\nrecord.inner.best\n",
    ] {
        for artifact in artifacts(source).0 {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let plan = jit.descriptor().plan();
            for block in plan.blocks() {
                if let Action::LibraryCall {
                    template, output, ..
                } = block.action()
                {
                    if matches!(template.output().body(), SchemaBody::Record(_)) {
                        assert!(plan.scalar_aggregates().slot(*output).is_none());
                    }
                }
            }
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let args = inputs(
                &artifact,
                &[
                    ("best", Data::U32(17)),
                    ("item", Data::U32(29)),
                    ("cp", Data::U64(91)),
                ],
                &memory,
            );
            let mut output = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            equal(&artifact, &output, &reference(&artifact, &args));
        }
    }
}

#[test]
fn scalar_record_checks_preserve_error_order_exact_work_publication_and_retry() {
    for arithmetic_first in [false, true] {
        let earlier = if arithmetic_first {
            "  earlier := 12u64 / divisor\n"
        } else {
            ""
        };
        let source = format!(
            "#Packet(n<u64>, allowed<bool>, value<u64>) => <{{allowed<bool> value<u64> phase<u64>}}>\n  | :Try(left<u64>)\n  | :Done(phase<u64>).\n#Packet(n, allowed, value) -> :Try(n)\n  :Try(left)\n    | left > 0u64 -> :Try(left - 1u64)\n    | * -> :Done(left)\n  :Done(phase) => {{allowed: allowed, value: value, phase: phase}}.\nchecked(allowed<bool>, value<u64>, divisor<u64>) => (out<u64>) :=\n  packet := #Packet(3u64, allowed, value)\n{earlier}  enabled! := packet.allowed\n  out := packet.value + 12u64 / divisor + packet.phase.\nchecked(allowed<bool>, value<u64>, divisor<u64>)\n"
        );
        for artifact in artifacts(&source).0 {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            selected_records(&jit);
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let mut output = vec![None; artifact.outputs().len()];
            for (allowed, value, divisor) in [
                (true, 7, 3),
                (false, 91, 0),
                (true, 17, 0),
                (true, u64::MAX, 1),
                (true, 19, 2),
            ] {
                let args = inputs(
                    &artifact,
                    &[
                        ("allowed", Data::Bool(allowed)),
                        ("value", Data::U64(value)),
                        ("divisor", Data::U64(divisor)),
                    ],
                    &memory,
                );
                let aliases = args.clone();
                let prior = output.clone();
                let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let result = jit.evaluate_into(&args, &mut output, limits(), &memory);
                let resident = reference_result(&artifact, &args);
                if allowed && divisor != 0 && value != u64::MAX {
                    result.unwrap();
                    equal(&artifact, &output, &resident.unwrap());
                    // Spent work is not the conservative entry envelope. Keep
                    // the same published owner for every isolated probe and
                    // find the actual finite admission boundary, without
                    // changing the existing successful policy.
                    let published = output.clone();
                    let before = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let probe = |budget| {
                        let mut policy = limits();
                        policy.execution.max_work = budget;
                        let mut candidate = published.clone();
                        let result = jit.evaluate_into(&args, &mut candidate, policy, &memory);
                        let succeeded = result.is_ok();
                        if succeeded {
                            assert_eq!(
                                candidate
                                    .last()
                                    .unwrap()
                                    .as_ref()
                                    .unwrap()
                                    .canonical_data_draft()
                                    .unwrap(),
                                published
                                    .last()
                                    .unwrap()
                                    .as_ref()
                                    .unwrap()
                                    .canonical_data_draft()
                                    .unwrap()
                            );
                        } else {
                            assert!(candidate.iter().zip(&published).all(|(a, b)| {
                                a.as_ref()
                                    .unwrap()
                                    .shares_frozen_storage(b.as_ref().unwrap())
                            }));
                        }
                        drop(candidate);
                        assert_eq!(
                            (memory.used_bytes(), memory.retained_snapshot_nodes()),
                            before
                        );
                        succeeded
                    };
                    let (mut low, mut high) = (0, limits().execution.max_work);
                    assert!(!probe(low) && probe(high));
                    while high - low > 1 {
                        let middle = low + (high - low) / 2;
                        if probe(middle) {
                            high = middle;
                        } else {
                            low = middle;
                        }
                    }
                    assert!(probe(high));
                    assert!(!probe(high - 1));
                    println!(
                        "scalar-record-admission first={arithmetic_first} minimum={high} one-short={}",
                        high - 1
                    );
                    assert!(output.iter().zip(&published).all(|(a, b)| {
                        a.as_ref()
                            .unwrap()
                            .shares_frozen_storage(b.as_ref().unwrap())
                    }));
                    jit.evaluate_into(&args, &mut output, limits(), &memory)
                        .unwrap();
                } else {
                    let error = format!("{:?}", result.unwrap_err());
                    let ordinary = resident.unwrap_err();
                    let assertion = !allowed && !arithmetic_first;
                    assert_eq!(error.contains("Integrity"), assertion, "{error}");
                    assert_eq!(
                        ordinary.contains("LexicalIntegrity"),
                        assertion,
                        "{ordinary}"
                    );
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        retained
                    );
                    assert!(output.iter().zip(&prior).all(|(a, b)| {
                        a.as_ref()
                            .unwrap()
                            .shares_frozen_storage(b.as_ref().unwrap())
                    }));
                }
                assert!(
                    args.iter()
                        .zip(&aliases)
                        .all(|(a, b)| a.shares_frozen_storage(b))
                );
            }
            drop(output);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}
