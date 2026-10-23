//! Test-only observations of the actual consumer, never replacement execution.
use super::super::tests::{artifact_source, limits};
use super::*;
use crate::native_artifact::{JitProgramArtifact, NativeProgramExecutionLimits};
use mech_core::ManagedMemoryBudget;
use std::cell::Cell;

#[derive(Clone, Copy, Debug)]
pub(super) struct Demand {
    pub temporary: u64,
    pub output: u64,
    pub cloned: u64,
    pub nodes: u64,
    pub compute: u64,
    pub compute_prefix: u64,
    pub envelope: u64,
    pub selection_slot_empty: bool,
}
#[derive(Clone, Copy)]
enum Pressure {
    Memory(bool),
    Import(u32),
}
thread_local! {
    static LAST: Cell<Option<Demand>> = const { Cell::new(None) };
    static PRESSURE: Cell<Option<Pressure>> = const { Cell::new(None) };
    static FAULT: Cell<Option<Fault>> = const { Cell::new(None) };
    static FIRED: Cell<bool> = const { Cell::new(false) };
}

#[derive(Clone, Copy, Debug)]
enum Fault {
    ReplaySelection,
    MissingSelection,
    WrongRegion,
    WrongParent,
    SkipConsumer,
    MalformedVisit,
    ConsumeTwice,
}

// Corrupt only private protocol state at the actual emitted consumer. The
// ordinary production guards and selected calls still decide the outcome.
pub(super) fn before_call(
    ctx: &mut Context<'_, '_>,
    site: usize,
    role: Site,
) -> Result<bool, Error> {
    let Some(fault) = FAULT.with(Cell::get) else {
        return Ok(false);
    };
    if !matches!(role, Site::Join(_)) || ctx.selected_join.is_none() || FIRED.with(Cell::get) {
        return Ok(false);
    }
    FIRED.with(|fired| fired.set(true));
    match fault {
        Fault::ReplaySelection => {
            let index = ctx.selected_join.as_ref().unwrap().region;
            let selection = ctx.control.descriptor.selected_joins.regions[index].selection();
            ctx.selected_join_call(selection as usize, &[0, 0])?;
        }
        Fault::MissingSelection => {
            ctx.selected_join = None;
        }
        Fault::WrongRegion => {
            ctx.selected_join.as_mut().unwrap().region =
                ctx.control.descriptor.selected_joins.regions.len();
        }
        Fault::WrongParent => {
            ctx.selected_join.as_mut().unwrap().parent = Some(u32::MAX);
        }
        Fault::SkipConsumer => return Ok(true),
        Fault::MalformedVisit => {
            // Isolate the actual bounds/pending-state guard from omission.
            ctx.consumed = true;
            ctx.visit(u64::MAX)?;
        }
        Fault::ConsumeTwice => {
            ctx.selected_join_call(site, &[0, 0])?;
        }
    }
    Ok(false)
}

pub(super) fn observe(
    ctx: &Context<'_, '_>,
    site: usize,
    demand: Demand,
) -> Result<Option<ManagedMemoryReservation>, Error> {
    LAST.with(|slot| slot.set(Some(demand)));
    let memory = ctx.library.memory_budget();
    match PRESSURE.with(Cell::get) {
        None => Ok(None),
        Some(Pressure::Import(steps)) => {
            memory.inject_snapshot_import_failure_after(steps);
            Ok(None)
        }
        Some(Pressure::Memory(short)) => {
            let remaining = demand.envelope - u64::from(short);
            let competing = memory
                .limit_bytes()
                .checked_sub(memory.used_bytes())
                .and_then(|free| free.checked_sub(remaining))
                .ok_or_else(|| invalid("test shared-account pressure cannot reach target"))?;
            memory
                .reserve_capacity(competing)
                .map(Some)
                .map_err(|error| library(ctx, site, error.into()))
        }
    }
}
fn take() -> Demand {
    LAST.with(Cell::take)
        .expect("actual selected join consumer was reached")
}
fn variants() -> [mech_engine::ProgramArtifact; 2] {
    let original = artifact_source(
        "+> text\nwords := input<[string]>\nselect := indices<[u64]>\ntext/join(words[select], separator<string>)\n",
    );
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    [original, decoded]
}
fn inputs(artifact: &mech_engine::ProgramArtifact, memory: &ManagedMemoryBudget) -> Vec<Value> {
    artifact
        .inputs()
        .iter()
        .map(|input| {
            let data = match mech_engine::decode_source_input_name(&input.name)
                .unwrap()
                .as_str()
            {
                "input" => ValueDataDraft::Matrix(
                    (0..4)
                        .map(|_| ValueDataDraft::String("a".repeat(256 * 1024)))
                        .collect(),
                ),
                "indices" => ValueDataDraft::Matrix([4, 1, 4, 2].map(ValueDataDraft::U64).into()),
                "separator" => ValueDataDraft::String("|".into()),
                name => panic!("unexpected input {name}"),
            };
            let shape_values = if let ValueDataDraft::Matrix(cells) = &data {
                mech_core::shape_for_resolved_extents(
                    artifact.schemas().get(input.schema).unwrap(),
                    &[1, cells.len() as u64],
                )
                .unwrap()
                .parameter_values()
                .to_vec()
                .into_boxed_slice()
            } else {
                Box::new([])
            };
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
            value.into_memory_budget(&mut claim).unwrap()
        })
        .collect()
}
fn required(demand: Demand, dimension: Dimension) -> u64 {
    match dimension {
        Dimension::TemporaryBytes => demand.temporary,
        Dimension::OutputBytes => demand.output,
        Dimension::ClonedBytes => demand.cloned,
        Dimension::RetainedNodes => demand.nodes,
        Dimension::ComputeWork => demand.compute,
        Dimension::OutputElements => 1,
        _ => panic!("unused boundary"),
    }
}
fn cap(limits: &mut NativeProgramExecutionLimits, dimension: Dimension, value: u64) {
    match dimension {
        Dimension::TemporaryBytes => limits.library.max_temporary_bytes = Some(value),
        Dimension::OutputBytes => limits.library.max_output_bytes = Some(value),
        Dimension::ClonedBytes => limits.library.max_cloned_bytes = Some(value),
        Dimension::RetainedNodes => limits.library.max_retained_nodes = Some(value),
        Dimension::ComputeWork => limits.library.max_compute_work = Some(value),
        Dimension::OutputElements => limits.library.max_output_elements = Some(value),
        _ => panic!("unused boundary"),
    }
}
fn preserved(outputs: &[Option<Value>], prior: &[Option<Value>]) {
    for (actual, old) in outputs.iter().zip(prior) {
        assert!(
            actual
                .as_ref()
                .unwrap()
                .shares_frozen_storage(old.as_ref().unwrap())
        );
    }
}

#[test]
fn actual_selected_join_local_exact_and_one_short_reach_the_consumer_and_recover() {
    for artifact in variants() {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let args = inputs(&artifact, &memory);
        let mut outputs = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut outputs, limits(), &memory)
            .unwrap();
        let demand = take();
        assert!(
            demand.selection_slot_empty,
            "no intermediate canonical Matrix"
        );
        let consumer = jit.descriptor().selected_joins.regions[0].consumer() as usize;
        let origin = jit.descriptor().plan.blocks()[consumer].origin();
        for dimension in [
            Dimension::TemporaryBytes,
            Dimension::OutputBytes,
            Dimension::ClonedBytes,
            Dimension::RetainedNodes,
            Dimension::ComputeWork,
            Dimension::OutputElements,
        ] {
            let amount = required(demand, dimension);
            let mut exact = limits();
            cap(&mut exact, dimension, amount);
            jit.evaluate_into(&args, &mut outputs, exact, &memory)
                .unwrap();
            let admitted = take();
            assert!(admitted.selection_slot_empty);
            let prior = outputs.clone();
            let before = memory.used_bytes();
            cap(&mut exact, dimension, amount - 1);
            let error = jit
                .evaluate_into(&args, &mut outputs, exact, &memory)
                .unwrap_err();
            // Do not credit a denial from an earlier call/setup boundary.
            let denied = take();
            assert_eq!(denied.compute_prefix, admitted.compute_prefix);
            // Call allowances are relative, but the shared work ledger reports
            // absolute totals. Its already-paid prefix is never reset/refunded.
            let expected_required = if dimension == Dimension::ComputeWork {
                denied.compute_prefix.checked_add(amount).unwrap()
            } else {
                amount
            };
            assert!(
                matches!(error, Error::Library {
                origin: actual_origin,
                error: InvokeError::Kernel(ResidentKernelError::ResourceLimit {
                    dimension: actual_dimension, required, limit, ..
                }),
            } if actual_origin == origin && actual_dimension == dimension
                && required == expected_required && limit == expected_required - 1),
                "{dimension:?}: {error:?}"
            );
            assert_eq!(memory.used_bytes(), before);
            preserved(&outputs, &prior);
            jit.evaluate_into(&args, &mut outputs, limits(), &memory)
                .unwrap();
            take();
        }
        drop((outputs, args));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn actual_selected_join_global_exact_one_short_and_import_failures_keep_atomic_outputs() {
    for artifact in variants() {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let args = inputs(&artifact, &memory);
        let mut outputs = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut outputs, limits(), &memory)
            .unwrap();
        take();
        PRESSURE.with(|slot| slot.set(Some(Pressure::Memory(false))));
        let accepted = jit.evaluate_into(&args, &mut outputs, limits(), &memory);
        PRESSURE.with(|slot| slot.set(None));
        accepted.unwrap();
        take();
        for pressure in [
            Pressure::Memory(true),
            Pressure::Import(0),
            Pressure::Import(1),
        ] {
            let prior = outputs.clone();
            let before = memory.used_bytes();
            PRESSURE.with(|slot| slot.set(Some(pressure)));
            let outcome = jit.evaluate_into(&args, &mut outputs, limits(), &memory);
            PRESSURE.with(|slot| slot.set(None));
            let error = outcome.unwrap_err();
            take();
            match pressure {
                Pressure::Memory(true) => assert!(
                    matches!(error,
                    Error::Library { error: InvokeError::Memory(mech_core::MemoryRuntimeError::BudgetExceeded {
                        requested, limit, ..
                    }), .. } if requested == memory.limit_bytes() + 1 && limit == memory.limit_bytes()),
                    "{error:?}"
                ),
                Pressure::Import(_) => assert!(
                    matches!(
                        error,
                        Error::Library {
                            error: InvokeError::Memory(
                                mech_core::MemoryRuntimeError::AllocationFailed { .. }
                            ),
                            ..
                        }
                    ),
                    "{error:?}"
                ),
                _ => unreachable!(),
            }
            assert_eq!(memory.used_bytes(), before);
            preserved(&outputs, &prior);
            jit.evaluate_into(&args, &mut outputs, limits(), &memory)
                .unwrap();
            take();
        }
        drop((outputs, args));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn actual_selected_join_pending_owner_protocol_denials_are_atomic_and_recover() {
    for artifact in variants() {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let args = inputs(&artifact, &memory);
        let mut outputs = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut outputs, limits(), &memory)
            .unwrap();
        for fault in [
            Fault::ReplaySelection,
            Fault::MissingSelection,
            Fault::WrongRegion,
            Fault::WrongParent,
            Fault::SkipConsumer,
            Fault::MalformedVisit,
            Fault::ConsumeTwice,
        ] {
            let prior = outputs.clone();
            let before = memory.used_bytes();
            FAULT.with(|slot| slot.set(Some(fault)));
            FIRED.with(|slot| slot.set(false));
            let result = jit.evaluate_into(&args, &mut outputs, limits(), &memory);
            FAULT.with(|slot| slot.set(None));
            assert!(FIRED.with(Cell::get), "actual consumer reached: {fault:?}");
            let error = result.unwrap_err();
            assert!(
                matches!(
                    error,
                    Error::Base(crate::native_artifact::NativeExecutionError::Descriptor { .. })
                        | Error::Value(crate::native_artifact::NativeValueError::StaleHandle)
                ),
                "{fault:?}: {error:?}"
            );
            preserved(&outputs, &prior);
            assert_eq!(memory.used_bytes(), before);
            jit.evaluate_into(&args, &mut outputs, limits(), &memory)
                .unwrap();
        }
        LAST.with(|slot| slot.set(None));
        FIRED.with(|slot| slot.set(false));
        drop((args, outputs));
        assert_eq!(memory.used_bytes(), 0);
    }
}
