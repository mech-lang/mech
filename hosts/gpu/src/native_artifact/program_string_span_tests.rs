//! Safe corruption hooks at the SAME actual emitted collection callback.
//! Only deliberate private protocol faults are injected; no observer unwinds
//! through C, changes policy, renews work or replaces a normal producer.
use super::super::super::tests::{artifact_source, input, limits};
use super::*;
use crate::native_artifact::{JitProgramArtifact, NativeValueError};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Fault {
    SelectorReplay,
    MissingSelector,
    WrongSelectorCandidate,
    WrongSpanCandidate,
    OldSourceGeneration,
    ConsumedSpanReplay,
    LeakedSpanAtLatch,
    ObserveAbsentValues,
    MalformedMaximumVisit,
    MalformedOnePastVisit,
}
thread_local! {
    static FAULT: core::cell::Cell<Option<Fault>> = const { core::cell::Cell::new(None) };
    static FIRED: core::cell::Cell<bool> = const { core::cell::Cell::new(false) };
    static ABSENT: core::cell::Cell<bool> = const { core::cell::Cell::new(false) };
    static GUARD_WITH_STATE: core::cell::Cell<bool> = const { core::cell::Cell::new(false) };
    static REQUIRE_GENERATED: core::cell::Cell<bool> = const { core::cell::Cell::new(false) };
    static LAST_PIN_WITNESS: core::cell::Cell<bool> = const { core::cell::Cell::new(false) };
}

/// Deliberately forge only the incoming visit number, before the ordinary
/// collection helper takes State out of Context. The actual emitted callback
/// has already admitted this valid Prepare site. Mark its required helper
/// consumed so the test cannot be satisfied by the earlier omission check.
pub(in super::super::super) fn before_helper(
    ctx: &mut Context<'_, '_>,
    site: usize,
) -> Result<(), Error> {
    let fault = FAULT.with(core::cell::Cell::get);
    if !matches!(
        fault,
        Some(Fault::MalformedMaximumVisit | Fault::MalformedOnePastVisit)
    ) || FIRED.with(core::cell::Cell::get)
    {
        return Ok(());
    }
    let Action::PrepareStringSpanSelector { collection } =
        ctx.control.descriptor.plan.blocks()[site].action()
    else {
        return Ok(());
    };
    let state = ctx
        .collections
        .as_ref()
        .ok_or_else(|| invalid("test malformed visit lost collection State"))?;
    if state.current != Some(*collection) || ctx.consumed {
        return Err(invalid("test malformed visit lacks fresh active builder"));
    }
    GUARD_WITH_STATE.with(|guard| guard.set(true));
    ctx.consumed = true;
    FIRED.with(|fired| fired.set(true));
    let malformed = if fault == Some(Fault::MalformedMaximumVisit) {
        u64::MAX
    } else {
        ctx.control.descriptor.plan.blocks().len() as u64
    };
    ctx.visit(malformed)
}

/// Returns true only for the intentional forged omitted-Push case. All other
/// modes call the unchanged production stage immediately after this safe hook.
pub(in super::super) fn before_stage(
    state: &mut State,
    ctx: &mut Context<'_, '_>,
    site: usize,
    words: [u64; 2],
) -> Result<bool, Error> {
    let Some(fault) = FAULT.with(core::cell::Cell::get) else {
        return Ok(false);
    };
    if FIRED.with(core::cell::Cell::get) {
        return Ok(false);
    }
    let action = ctx.control.descriptor.plan.blocks()[site].action();
    let (id, stage) = match action {
        Action::PrepareStringSpanSelector { collection } => (*collection, 0),
        Action::ProjectStringSpan { collection } => (*collection, 1),
        Action::CollectionPush { collection, .. }
            if ctx.control.descriptor.plan.collections()[*collection as usize]
                .string_span_projection()
                .is_some() =>
        {
            (*collection, 2)
        }
        _ => return Ok(false),
    };
    let expected = match fault {
        Fault::SelectorReplay => 0,
        Fault::MissingSelector | Fault::WrongSelectorCandidate => 1,
        _ => 2,
    };
    if stage != expected {
        return Ok(false);
    }
    FIRED.with(|fired| fired.set(true));
    match fault {
        Fault::SelectorReplay => state.prepare_string_span(ctx, site, id, words)?,
        Fault::MissingSelector => {
            if state.string_spans[id as usize].take().is_some() {
                state.active_string_spans = state
                    .active_string_spans
                    .checked_sub(1)
                    .ok_or_else(|| invalid("test missing selector counter setup"))?;
            }
        }
        Fault::WrongSelectorCandidate => {
            if let Some(Lane::Selector { visit, .. }) = state.string_spans[id as usize].as_mut() {
                visit.remaining = visit.remaining.wrapping_add(1);
            }
        }
        Fault::WrongSpanCandidate => {
            if let Some(Lane::Span { visit, .. }) = state.string_spans[id as usize].as_mut() {
                visit.remaining = visit.remaining.wrapping_add(1);
            }
        }
        Fault::OldSourceGeneration => {
            let source = ctx.control.descriptor.plan.collections()[id as usize]
                .string_span_projection()
                .ok_or_else(|| invalid("test span proof"))?
                .source();
            if REQUIRE_GENERATED.with(core::cell::Cell::get) {
                let Some(Lane::Span { owner, .. }) = state.string_spans[id as usize].as_ref()
                else {
                    return Err(invalid("test generated source lacks real pin"));
                };
                let input = ctx.control.descriptor.plan.inputs()[0].value();
                let distinct = ctx
                    .arena
                    .test_current_value(input.get())
                    .is_some_and(|input| !input.shares_frozen_storage(owner));
                let aliases = (0..ctx.control.descriptor.types.len())
                    .filter(|&slot| {
                        ctx.arena
                            .test_current_value(slot as u32)
                            .is_some_and(|value| value.shares_frozen_storage(owner))
                    })
                    .count();
                let no_call_alias = ctx
                    .call_inputs
                    .values()
                    .iter()
                    .flatten()
                    .all(|value| !value.shares_frozen_storage(owner));
                let no_source_alias =
                    state
                        .sources
                        .iter()
                        .flatten()
                        .all(|source| match &source.backing {
                            super::super::SourceBacking::Value(value) => {
                                !value.shares_frozen_storage(owner)
                            }
                            super::super::SourceBacking::U64Range { .. } => true,
                            super::super::SourceBacking::ConsumedString => true,
                        });
                let witness = distinct && aliases == 1 && no_call_alias && no_source_alias;
                LAST_PIN_WITNESS.with(|slot| slot.set(witness));
                if !witness {
                    return Err(invalid(
                        "test generated source still has an unrelated owner",
                    ));
                }
            }
            ctx.arena
                .retire_current(source.get(), &mut ctx.control.work)?;
        }
        Fault::ConsumedSpanReplay => {
            let output = ctx.control.descriptor.plan.collections()[id as usize]
                .string_span_projection()
                .ok_or_else(|| invalid("test span proof"))?
                .output();
            let pin = state.take_string_yield(ctx, id, output)?;
            state.release_string_yield(pin)?;
        }
        Fault::LeakedSpanAtLatch => return Ok(true),
        Fault::ObserveAbsentValues => {
            let proof = ctx.control.descriptor.plan.collections()[id as usize]
                .string_span_projection()
                .ok_or_else(|| invalid("test span proof"))?;
            let selector_absent = matches!(
                ctx.arena
                    .copy_current(proof.selector().get(), &mut ctx.control.work),
                Err(NativeValueError::StaleHandle)
            );
            let output_absent = matches!(
                ctx.arena
                    .copy_current(proof.output().get(), &mut ctx.control.work),
                Err(NativeValueError::StaleHandle)
            );
            ABSENT.with(|absent| absent.set(selector_absent && output_absent));
        }
        Fault::MalformedMaximumVisit | Fault::MalformedOnePastVisit => {
            return Err(invalid("test malformed visit did not reach Context guard"));
        }
    }
    Ok(false)
}

struct Reset;
impl Drop for Reset {
    fn drop(&mut self) {
        FAULT.with(|fault| fault.set(None));
        FIRED.with(|fired| fired.set(false));
        ABSENT.with(|absent| absent.set(false));
        GUARD_WITH_STATE.with(|guard| guard.set(false));
        REQUIRE_GENERATED.with(|required| required.set(false));
        LAST_PIN_WITNESS.with(|witness| witness.set(false));
    }
}

#[test]
fn emitted_string_span_private_phase_generation_replay_and_escape_faults_are_atomic() {
    for (source, generated) in [
        (
            "+> text\ntext/join([(seed<string>)[1u64..=2u64] | attempt <- 1u64..=2u64], \"|\")\n",
            false,
        ),
        (
            "+> text\ntext/join([piece[1u64..=2u64] | attempt <- 1u64..=2u64, piece := text/join([part | part <- seed<string>], \"\")], \"|\")\n",
            true,
        ),
    ] {
        exercise_faults(source, generated);
    }
}

fn exercise_faults(source: &str, generated: bool) {
    let original = artifact_source(source);
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    for artifact in [original, decoded] {
        let memory = mech_core::ManagedMemoryBudget::new(256 * 1024 * 1024);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog())
            .unwrap_or_else(|error| panic!("{source}: {error:?}"));
        assert_eq!(
            jit.descriptor()
                .plan()
                .collections()
                .iter()
                .filter(|collection| collection.string_span_projection().is_some())
                .count(),
            1
        );
        let args = [input(&artifact, &memory)];
        let mut output = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap();
        let prior = output
            .iter()
            .map(|value| {
                value
                    .as_ref()
                    .unwrap()
                    .canonical_snapshot_bytes(artifact.schemas())
                    .unwrap()
            })
            .collect::<Vec<_>>();
        let baseline = memory.used_bytes();
        for fault in [
            Fault::SelectorReplay,
            Fault::MissingSelector,
            Fault::WrongSelectorCandidate,
            Fault::WrongSpanCandidate,
            Fault::OldSourceGeneration,
            Fault::ConsumedSpanReplay,
            Fault::LeakedSpanAtLatch,
            Fault::ObserveAbsentValues,
            Fault::MalformedMaximumVisit,
            Fault::MalformedOnePastVisit,
        ] {
            let _reset = Reset;
            FAULT.with(|slot| slot.set(Some(fault)));
            FIRED.with(|fired| fired.set(false));
            REQUIRE_GENERATED.with(|required| required.set(generated));
            let result = jit.evaluate_into(&args, &mut output, limits(), &memory);
            assert!(
                FIRED.with(core::cell::Cell::get),
                "actual emitted selected stage must be reached: {fault:?}"
            );
            if fault == Fault::ObserveAbsentValues {
                result.unwrap();
                assert!(
                    ABSENT.with(core::cell::Cell::get),
                    "no elided selector/String Value can be read from the real arena"
                );
            } else {
                let error = result.unwrap_err();
                if generated && fault == Fault::OldSourceGeneration {
                    assert!(
                        LAST_PIN_WITNESS.with(core::cell::Cell::get),
                        "generated source must have exactly its arena root and private pin, distinct from external input and all call/source aliases"
                    );
                    assert!(
                        matches!(error, Error::Value(NativeValueError::StaleHandle)),
                        "must retire the generated source successfully and reject its old generation: {error:?}"
                    );
                }
                if matches!(
                    fault,
                    Fault::MalformedMaximumVisit | Fault::MalformedOnePastVisit
                ) {
                    assert!(
                        GUARD_WITH_STATE.with(core::cell::Cell::get),
                        "collection State must be present at the checked guard"
                    );
                    assert!(
                        matches!(
                            error,
                            Error::Base(crate::native_artifact::NativeExecutionError::Descriptor {
                                reason: "program visit site outside descriptor",
                            })
                        ),
                        "must reach checked site lookup, not earlier omitted helper rejection: {fault:?}: {error:?}"
                    );
                }
                assert!(
                    matches!(
                        error,
                        Error::Value(NativeValueError::StaleHandle)
                            | Error::Base(
                                crate::native_artifact::NativeExecutionError::Descriptor { .. }
                            )
                    ),
                    "{fault:?}: {error:?}"
                );
            }
            assert_eq!(memory.used_bytes(), baseline, "{fault:?}");
            for (actual, prior) in output.iter().zip(&prior) {
                assert_eq!(
                    actual
                        .as_ref()
                        .unwrap()
                        .canonical_snapshot_bytes(artifact.schemas())
                        .unwrap(),
                    *prior
                );
            }
            FAULT.with(|slot| slot.set(None));
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            assert_eq!(memory.used_bytes(), baseline);
        }
        drop((args, output));
        assert_eq!(memory.used_bytes(), 0);
    }
}
