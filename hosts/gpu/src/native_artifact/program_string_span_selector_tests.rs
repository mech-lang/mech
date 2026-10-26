//! Fixed-policy witnesses around the SAME emitted selector account/constructor.
//! Copy-only observers cannot replace a helper, mutate a ledger, or unwind C.
use super::super::super::tests::{artifact_source, limits};
use super::{Context, Error, Lane, State, Visit};
use crate::native_artifact::{JitProgramArtifact, NativeProgramExecutionLimits};
use mech_core::{
    DimensionExpr, IntegerWidth, ManagedMemoryBudget, MemoryBudgetDimension, ResidentKernelError,
    SchemaBody, Value,
    snapshot::{SnapshotValidationContext, ValueDataDraft, ValueDraft},
};
use mech_engine::{
    ProgramArtifact,
    native_artifact::{NativeCollectionInvocationError, NativeMatchOrigin},
};

#[derive(Clone, Copy, Debug)]
struct Record {
    site: u32,
    collection: u32,
    iteration: Option<u32>,
    current: Option<u32>,
    remaining: Option<u64>,
    words: [u64; 2],
    prior: Option<u64>,
    price: u64,
    native_before: u64,
    native_after: u64,
    builder_after: Option<u64>,
    account_ok: Option<bool>,
    constructor_called: bool,
    constructor_ok: Option<bool>,
    native_constructed: Option<u64>,
    installed: Option<Visit>,
    selector_count: Option<u64>,
    active: Option<u64>,
    calls: u64,
}
thread_local! {
    static RECORD: core::cell::Cell<Option<Record>> = const { core::cell::Cell::new(None) };
}
fn update(change: impl FnOnce(&mut Record)) {
    RECORD.with(|cell| {
        if let Some(mut record) = cell.get() {
            change(&mut record);
            cell.set(Some(record));
        }
    });
}
pub(super) fn before_account(
    state: &State,
    ctx: &Context<'_, '_>,
    site: usize,
    collection: u32,
    words: [u64; 2],
    price: u64,
) {
    let iteration = ctx
        .control
        .descriptor
        .metadata()
        .plan
        .blocks()
        .get(site)
        .and_then(|b| b.iteration());
    let remaining = iteration.and_then(|i| {
        (i as usize)
            .checked_mul(2)
            .and_then(|i| ctx.control.progress.get(i).copied())
    });
    let prior = state
        .builders
        .get(collection as usize)
        .and_then(Option::as_ref)
        .map(|b| b.compute);
    let native = ctx.control.work.used();
    RECORD.with(|cell| {
        let calls = cell.get().map_or(1, |r| r.calls.saturating_add(1));
        cell.set(Some(Record {
            site: site as u32,
            collection,
            iteration,
            current: ctx.control.current,
            remaining,
            words,
            prior,
            price,
            native_before: native,
            native_after: native,
            builder_after: prior,
            account_ok: None,
            constructor_called: false,
            constructor_ok: None,
            native_constructed: None,
            installed: None,
            selector_count: None,
            active: None,
            calls,
        }));
    });
}
pub(super) fn after_account(state: &State, ctx: &Context<'_, '_>, ok: bool) {
    update(|r| {
        r.account_ok = Some(ok);
        r.native_after = ctx.control.work.used();
        r.builder_after = state
            .builders
            .get(r.collection as usize)
            .and_then(Option::as_ref)
            .map(|b| b.compute);
    });
}
pub(super) fn before_constructor() {
    update(|r| r.constructor_called = true);
}
pub(super) fn after_constructor(ctx: &Context<'_, '_>, ok: bool) {
    update(|r| {
        r.constructor_ok = Some(ok);
        r.native_constructed = Some(ctx.control.work.used());
    });
}
pub(super) fn after_install(state: &State, collection: u32) {
    update(|r| {
        r.installed = match state
            .string_spans
            .get(collection as usize)
            .and_then(Option::as_ref)
        {
            Some(Lane::Selector { visit, selector }) => {
                r.selector_count = Some(selector.count());
                Some(*visit)
            }
            _ => None,
        };
        r.active = Some(state.active_string_spans);
    });
}

const SOURCE: &str = "+> text\ntext/join([(alphabet<string>)[begin<u64>..=until<u64>] | attempt <- 1u64..=1u64], \"\")\n";
const EXPECTED: &str = "é🧑‍🚀";
fn arguments(artifact: &ProgramArtifact, memory: &ManagedMemoryBudget) -> Vec<Value> {
    let mut names = std::collections::BTreeSet::new();
    let arguments = artifact
        .inputs()
        .iter()
        .map(|input| {
            let name = mech_engine::decode_source_input_name(&input.name).unwrap();
            let data = match name.as_str() {
                "alphabet" => ValueDataDraft::String("é🧑‍🚀x".into()),
                "begin" => ValueDataDraft::U64(1),
                "until" => ValueDataDraft::U64(2),
                _ => panic!("unexpected fixture input {name}"),
            };
            assert!(names.insert(name));
            let value = ValueDraft {
                schema: input.schema,
                shape_values: Box::new([]),
                data,
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap();
            let mut reservation = memory
                .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
                .unwrap();
            value.into_memory_budget(&mut reservation).unwrap()
        })
        .collect();
    assert_eq!(
        names,
        ["alphabet", "begin", "until"]
            .map(str::to_owned)
            .into_iter()
            .collect()
    );
    arguments
}
#[derive(Clone, Copy)]
struct Target {
    site: u32,
    collection: u32,
    iteration: u32,
    range_origin: NativeMatchOrigin,
    selection_origin: NativeMatchOrigin,
    collection_origin: NativeMatchOrigin,
    price: u64,
}
fn target(jit: &JitProgramArtifact) -> Target {
    let plan = jit.descriptor().plan();
    let projections = plan
        .collections()
        .iter()
        .enumerate()
        .filter_map(|(id, c)| c.string_span_projection().map(|p| (id, c, p)))
        .collect::<Vec<_>>();
    assert_eq!(projections.len(), 1);
    let (id, collection, projection) = projections[0];
    let schema = projection.range_template().output();
    let SchemaBody::Matrix {
        element,
        dimensions,
    } = schema.body()
    else {
        panic!("selected Range must have its real Matrix schema");
    };
    assert_eq!(
        element.as_ref(),
        &SchemaBody::UnsignedInteger(IntegerWidth::W64)
    );
    assert_eq!(schema.dimension_parameters().len(), 1);
    assert!(
        matches!(dimensions.as_ref(), [DimensionExpr::Constant(1), DimensionExpr::Parameter(p)] if p.get() == 0)
    );
    for (operand, expected) in projection.operands().iter().zip(["begin", "until"]) {
        let input = plan
            .inputs()
            .iter()
            .find(|input| input.value() == *operand)
            .unwrap();
        assert_eq!(
            mech_engine::decode_source_input_name(input.name()).as_deref(),
            Some(expected)
        );
    }
    let site = projection.range_site();
    Target {
        site,
        collection: id as u32,
        iteration: plan.blocks()[site as usize].iteration().unwrap(),
        range_origin: projection.range_template().origin(),
        selection_origin: projection.selection_template().origin(),
        collection_origin: collection.origin(),
        price: jit.descriptor().metadata().calls[site as usize]
            .as_ref()
            .unwrap()
            .selected_u64_string_span_selector_work(),
    }
}
fn observed(
    jit: &JitProgramArtifact,
    args: &[Value],
    output: &mut [Option<Value>],
    caps: NativeProgramExecutionLimits,
    memory: &ManagedMemoryBudget,
) -> (
    Result<crate::native_artifact::NativeProgramExecutionReport, Error>,
    Option<Record>,
) {
    RECORD.with(|cell| cell.set(None));
    let result = jit.evaluate_into(args, output, caps, memory);
    (result, RECORD.with(core::cell::Cell::get))
}
fn record(record: Option<Record>, target: Target, ok: bool) -> Record {
    let r = record.expect("actual selected Range account must be reached");
    assert_eq!(
        (
            r.site,
            r.collection,
            r.iteration,
            r.current,
            r.remaining,
            r.calls
        ),
        (
            target.site,
            target.collection,
            Some(target.iteration),
            Some(target.iteration),
            Some(1),
            1
        )
    );
    assert_eq!(r.words, [1, 2]);
    assert_eq!(r.price, target.price);
    assert!(r.price > 0 && r.native_before > 0);
    assert_eq!(r.account_ok, Some(ok));
    let prior = r.prior.unwrap();
    if ok {
        assert_eq!(r.native_after.checked_sub(r.native_before), Some(r.price));
        assert_eq!(r.builder_after, prior.checked_add(r.price));
        assert!(r.constructor_called);
        assert_eq!(r.constructor_ok, Some(true));
        assert_eq!(r.native_constructed, Some(r.native_after));
        assert_eq!(
            r.installed,
            Some(Visit {
                iteration: target.iteration,
                remaining: 1
            })
        );
        assert_eq!(r.selector_count, Some(2));
        assert_eq!(r.active, Some(1));
    } else {
        assert_eq!(
            r.native_after, r.native_before,
            "denial cannot debit or refund spent prefix work"
        );
        assert_eq!(r.builder_after, Some(prior));
        assert!(!r.constructor_called);
        assert_eq!(r.selector_count, None);
        assert_eq!(
            (
                r.constructor_ok,
                r.native_constructed,
                r.installed,
                r.active
            ),
            (None, None, None, None)
        );
    }
    r
}
fn bytes(artifact: &ProgramArtifact, outputs: &[Option<Value>]) -> Vec<Box<[u8]>> {
    outputs
        .iter()
        .map(|v| {
            v.as_ref()
                .unwrap()
                .canonical_snapshot_bytes(artifact.schemas())
                .unwrap()
        })
        .collect()
}
fn assert_output(outputs: &[Option<Value>]) {
    assert_eq!(outputs.len(), 1);
    assert_eq!(
        outputs[0].as_ref().unwrap().canonical_data_draft().unwrap(),
        ValueDataDraft::String(EXPECTED.into())
    );
}

#[test]
fn actual_selected_range_selector_compute_exact_one_short_preserves_spent_work_and_recovers() {
    let original = artifact_source(SOURCE);
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    for artifact in [original, decoded] {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let target = target(&jit);
        assert_ne!(target.range_origin, target.selection_origin);
        let args = arguments(&artifact, &memory);
        let mut output = vec![None; artifact.outputs().len()];
        let (result, observation) = observed(&jit, &args, &mut output, limits(), &memory);
        result.unwrap();
        record(observation, target, true);
        assert_output(&output);
        let prior = bytes(&artifact, &output);
        let baseline = memory.used_bytes();
        // Measure with the old publication ALREADY present, as in exact/short
        // runs. Its real retirement prepayment is part of the native prefix.
        let (result, observation) = observed(&jit, &args, &mut output, limits(), &memory);
        result.unwrap();
        let funded = record(observation, target, true);
        assert_eq!(bytes(&artifact, &output), prior);
        assert_eq!(memory.used_bytes(), baseline);
        let exact = funded.prior.unwrap().checked_add(funded.price).unwrap();
        assert!(exact > 0 && exact <= limits().library.max_compute_work.unwrap());
        let mut caps = limits();
        caps.library.max_compute_work = Some(exact);
        let (result, observation) = observed(&jit, &args, &mut output, caps, &memory);
        let admitted = record(observation, target, true);
        assert_eq!(
            (admitted.prior, admitted.native_before),
            (funded.prior, funded.native_before)
        );
        if let Err(error) = result {
            let later = match &error {
                Error::Library {
                    origin,
                    error:
                        NativeCollectionInvocationError::Kernel(ResidentKernelError::ResourceLimit {
                            dimension: MemoryBudgetDimension::ComputeWork,
                            required,
                            limit,
                            ..
                        }),
                } => *origin == target.selection_origin && *required > *limit && *limit == exact,
                Error::Collection {
                    origin,
                    error:
                        ResidentKernelError::ResourceLimit {
                            dimension: MemoryBudgetDimension::ComputeWork,
                            required,
                            limit,
                            ..
                        },
                } => *origin == target.collection_origin && *required > *limit && *limit == exact,
                _ => false,
            };
            assert!(
                later,
                "selector succeeded; only a truthful later component denial is allowed: {error:?}"
            );
        }
        assert_eq!(bytes(&artifact, &output), prior);
        assert_eq!(memory.used_bytes(), baseline);
        caps.library.max_compute_work = Some(exact - 1);
        let (result, observation) = observed(&jit, &args, &mut output, caps, &memory);
        let denied = record(observation, target, false);
        assert_eq!(
            (denied.prior, denied.native_before),
            (funded.prior, funded.native_before)
        );
        let error = result.unwrap_err();
        assert!(
            matches!(error, Error::Collection { origin, error: ResidentKernelError::ResourceLimit {
            phase: mech_core::ResidentResourcePhase::Admission,
            dimension: MemoryBudgetDimension::ComputeWork, required, limit, ..
        }} if origin == target.collection_origin && required == exact && limit == exact - 1),
            "the unchanged actual selected Range account must reject before constructor/debit: {error:?}"
        );
        assert_eq!(bytes(&artifact, &output), prior);
        assert_eq!(memory.used_bytes(), baseline);
        let (result, observation) = observed(&jit, &args, &mut output, limits(), &memory);
        result.unwrap();
        record(observation, target, true);
        assert_output(&output);
        assert_eq!(bytes(&artifact, &output), prior);
        assert_eq!(memory.used_bytes(), baseline);
        drop((args, output));
        assert_eq!(memory.used_bytes(), 0);
    }
}
