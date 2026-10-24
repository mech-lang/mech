//! Same-artifact metadata for generated ABI16, not an evaluator.
use super::{NativeValueSlot, iteration_abi::NativeIterationVisit};
use mech_core::{FunctionCatalog, SchemaTable, Value};
use mech_engine::{
    ProgramArtifact,
    native_artifact::{
        NativeIterationAction, NativePreparedLibraryCall, NativeProgramArtifactPlan,
        prepare_native_program_library_call,
    },
};
use std::sync::Arc;

#[path = "program_descriptor_image.rs"]
pub(super) mod image;

#[path = "program_retirement.rs"]
mod retirement;
pub(super) use retirement::{RetirementKind, RetirementTables};

#[derive(Debug, serde::Serialize)]
pub struct NativeProgramDescriptor {
    pub(super) plan: NativeProgramArtifactPlan,
    #[serde(serialize_with = "image::serialize_schemas")]
    pub(super) schemas: Arc<SchemaTable>,
    // Query entries are prepared from their SAME sink step and fully checked
    // by validate_query_stages before this immutable descriptor can escape.
    // Runtime stage lookup never rebinds or clones those selected templates.
    pub(super) calls: Box<[Option<NativePreparedLibraryCall>]>,
    pub(super) types: Box<[NativeValueSlot]>,
    pub(super) visits: Box<[NativeIterationVisit]>,
    pub(super) retirements: RetirementTables,
    pub(super) constant_bytes: Box<[u64]>,
    pub(super) scalar_bytes: u64,
    pub(super) observation_schema_bytes: u64,
    pub(super) max_library_inputs: usize,
    // Derived from this SAME immutable plan; callers cannot inject a region
    // from another artifact or rebind one to a different selected factory.
    pub(super) window_regions: Box<[mech_engine::native_artifact::NativeWindowRegion]>,
    pub(super) string_map_regions: Box<[mech_engine::native_artifact::NativeStringMapRegion]>,
    pub(super) scalar_aggregates: mech_engine::native_artifact::NativeScalarAggregates,
    // Complete same-plan candidate layout, preserved through standalone images.
    // Merely carrying this metadata does not select retained runtime storage.
    pub(super) map_owner_words: Option<mech_engine::native_artifact::NativeMapOwnerWords>,
    pub(super) frozen_maps: super::program_frozen_map_layout::Layout,
    pub(super) map_admission: super::program_map_admission::Layout,
    pub(super) parallel: super::program_parallel_layout::Layout,
    pub(super) borrowed_strings: super::program_borrowed_layout::Layout,
    pub(super) closed_tails: super::program_tail_layout::Layout,
    pub(super) selected_joins: super::program_selected_join_layout::Layout,
    pub(super) line_matrices: super::program_line_matrix_layout::Layout,
    pub(super) prefix: Option<super::program_prefix_layout::Region>,
    pub(super) stack: u64,
    pub(super) code: u64,
    pub(super) identity: [u8; 32],
}
impl NativeProgramDescriptor {
    pub(super) fn prepare(
        artifact: &ProgramArtifact,
        catalog: &FunctionCatalog,
        plan: NativeProgramArtifactPlan,
        stack: u64,
        code: u64,
    ) -> Result<Self, String> {
        if artifact.revision() != plan.program_revision() {
            return Err("mixed canonical revision mismatch".into());
        }
        plan.validate_current_visits().map_err(str::to_owned)?;
        let parallel = super::program_parallel_layout::Layout::derive(&plan).map_err(str::to_owned)?;
        let retirements = RetirementTables::derive(&plan).map_err(str::to_owned)?;
        let completion = (plan.transport_output_count() as u64)
            .checked_mul(64)
            .and_then(|n| n.checked_add((plan.iterations().len() as u64).checked_mul(8)?))
            .and_then(|n| n.checked_add(256))
            .ok_or("program completion overflow")?;
        let borrowed_strings = super::program_borrowed_layout::Layout::derive(&plan)?;
        let selected_joins = super::program_selected_join_layout::Layout::derive(&plan)?;
        let line_matrices = super::program_line_matrix_layout::Layout::derive(&plan)?;
        let mut visits = super::iteration_abi::control_visits(
            plan.blocks(),
            plan.iterations().iter().map(Into::into),
            completion,
        )
        .map_err(str::to_owned)?;
        selected_joins
            .fund_visit_checks(&mut visits)
            .map_err(str::to_owned)?;
        // Derive this proof once from the SAME immutable plan. Both layout and
        // identity inspect the complete result, including unsupported/multiple
        // frontiers; no caller-provided or persistent proof cache is consulted.
        let line_retention_frontiers = plan.text_line_retention_frontiers();
        let prefix = super::program_prefix_layout::derive_with_frontiers(
            &plan,
            &borrowed_strings,
            &visits,
            &line_retention_frontiers,
        )?;
        for (site, visit) in visits.iter_mut().enumerate() {
            visit.work = visit
                .work
                .checked_add(borrowed_strings.extra_work(site)?)
                .and_then(|work| {
                    work.checked_add(super::program_prefix_layout::extra_work(
                        prefix.as_ref(),
                        site,
                    ))
                })
                .ok_or("borrowed String visit work overflow")?;
        }
        let map_owner_words = plan.map_owner_words().map_err(str::to_owned)?;
        for (site, visit) in visits.iter_mut().enumerate() {
            visit.work = visit.work.checked_add(super::program_map_owner_layout::extra_work(&map_owner_words, site)?)
                .ok_or("map owner visit work overflow")?;
        }
        let closed_tails =
            super::program_tail_layout::Layout::derive(&plan, &borrowed_strings, &visits)?;
        for (site, visit) in visits.iter_mut().enumerate() {
            visit.work = visit
                .work
                .checked_add(closed_tails.extra_work(site))
                .ok_or("closed Tail work overflow")?;
        }
        let map_admission = super::program_map_admission::Layout::derive(&plan, &map_owner_words, &visits)?;
        let visits = visits.into();
        let calls = plan
            .blocks()
            .iter()
            .map(|block| match block.action() {
                NativeIterationAction::LibraryCall { template, .. } => {
                    prepare_native_program_library_call(artifact, template, catalog)
                        .map(Some)
                        .map_err(|e| e.to_string())
                }
                NativeIterationAction::PrepareStringSpanSelector { collection }
                | NativeIterationAction::ProjectStringSpan { collection } => {
                    let projection = plan.collections()[*collection as usize]
                        .string_span_projection()
                        .ok_or("String span action lacks its selected proof")?;
                    let template = if matches!(
                        block.action(),
                        NativeIterationAction::PrepareStringSpanSelector { .. }
                    ) {
                        projection.range_template()
                    } else {
                        projection.selection_template()
                    };
                    prepare_native_program_library_call(artifact, template, catalog)
                        .map(Some)
                        .map_err(|error| error.to_string())
                }
                NativeIterationAction::ProjectStringMapEntry { collection } => {
                    let projection = plan.collections()[*collection as usize]
                        .string_element_map_projection()
                        .ok_or("String map projection lacks its selected proof")?;
                    prepare_native_program_library_call(
                        artifact,
                        projection.selection_template(),
                        catalog,
                    )
                    .map(Some)
                    .map_err(|error| error.to_string())
                }
                NativeIterationAction::HistogramQueryStep { collection, step } => {
                    let mech_engine::native_artifact::NativeCollectionResult::GuardedHistogramQuery(
                        sink,
                    ) = plan.collections()[*collection as usize].result()
                    else {
                        return Err("histogram query stage lacks selected proof".into());
                    };
                    sink.steps()
                        .get(*step as usize)
                        .ok_or("histogram query step outside proof")?
                        .template()
                        .map(|template| {
                            prepare_native_program_library_call(artifact, template, catalog)
                        })
                        .transpose()
                        .map_err(|error| error.to_string())
                }
                NativeIterationAction::HistogramFinish { collection } => {
                    let mech_engine::native_artifact::NativeCollectionResult::Histogram(sink) =
                        plan.collections()[*collection as usize].result()
                    else {
                        return Err("histogram consumer lacks selected sink proof".into());
                    };
                    prepare_native_program_library_call(artifact, sink.template(), catalog)
                        .map(Some)
                        .map_err(|error| error.to_string())
                }
                NativeIterationAction::TextJoinFinish { collection } => {
                    let mech_engine::native_artifact::NativeCollectionResult::TextJoin(sink) =
                        plan.collections()[*collection as usize].result()
                    else {
                        return Err("text join consumer lacks selected sink proof".into());
                    };
                    prepare_native_program_library_call(artifact, sink.template(), catalog)
                        .map(Some)
                        .map_err(|e| e.to_string())
                }
                NativeIterationAction::PrepareU64Range { iteration } => {
                    let mech_engine::native_artifact::NativeControlLoop::Collection(generator) =
                        &plan.iterations()[*iteration as usize]
                    else {
                        return Err("range source is not a collection".into());
                    };
                    let mech_engine::native_artifact::NativeCollectionSource::U64Range(cursor) =
                        generator.source()
                    else {
                        return Err("range source lacks selected proof".into());
                    };
                    prepare_native_program_library_call(artifact, cursor.template(), catalog)
                        .map(Some)
                        .map_err(|e| e.to_string())
                }
                _ => Ok(None),
            })
            .collect::<Result<Box<[_]>, _>>()?;
        validate_query_stages(&plan, &calls).map_err(str::to_owned)?;
        let window_regions = plan.window_regions().into_boxed_slice();
        let string_map_regions = plan.string_map_regions().into_boxed_slice();
        let scalar_aggregates = plan.scalar_aggregates();
        let occupied_frame = plan.values().len()
            .checked_add(plan.iterations().len())
            .and_then(|n| n.checked_add(scalar_aggregates.words()))
            .and_then(|n| n.checked_add(borrowed_strings.words))
            .and_then(|n| n.checked_add(closed_tails.words))
            .ok_or("map owner frame prefix overflow")?;
        let map_owner_words = map_owner_words
            .after_frame_words(occupied_frame).map_err(str::to_owned)?;
        // Programs without owner candidates keep no second copy of their
        // existing scalar slot/action/constant inventories in the descriptor.
        let map_owner_words = (!map_owner_words.regions().is_empty()).then_some(map_owner_words);
        let max_library_inputs = plan
            .blocks()
            .iter()
            .filter_map(|block| match block.action() {
                NativeIterationAction::LibraryCall { operands, .. } => Some(operands.len()),
                _ => None,
            })
            .max()
            .unwrap_or(0)
            .max(
                if window_regions.is_empty() && string_map_regions.is_empty() {
                    0
                } else {
                    5
                },
            )
            .max(
                borrowed_strings
                    .regions
                    .iter()
                    .map(|r| r.capture_words())
                    .max()
                    .unwrap_or(0),
            )
            .max(map_owner_words.as_ref().map_or(0, super::program_map_owner_layout::initial_words))
            .max(usize::from(!parallel.regions.is_empty()))
            .max(if closed_tails.regions.is_empty() {
                0
            } else {
                4
            });
        let types = plan
            .values()
            .iter()
            .chain(
                (0..plan.transport_output_count()).map(|index| plan.transport_output_type(index)),
            )
            .map(|ty| NativeValueSlot {
                schema: ty.schema(),
                key: ty.schema_key(),
            })
            .collect();
        let constant_bytes = plan
            .constants()
            .iter()
            .map(|c| {
                c.constant()
                    .memory_budget_required_bytes()
                    .map_err(|e| format!("constant ownership: {e:?}"))?
                    .checked_add(Value::memory_budget_claim_metadata_bytes())
                    .and_then(|n| n.checked_add(4096))
                    .ok_or_else(|| "constant admission overflows".to_owned())
            })
            .collect::<Result<Box<[_]>, _>>()?;
        // Shared schema finalization avoids a schema clone, but ownership
        // admission still includes the complete schema context. Reserve a
        // conservative fixed scalar construction/import envelope BEFORE use.
        let observation_schema_bytes = artifact
            .schemas()
            .clone_allocation_bound_bytes()
            .ok_or("observation schema ownership bound overflows")?;
        let scalar_bytes = observation_schema_bytes
            .checked_add(Value::canonical_owner_allocation_bytes())
            .and_then(|n| n.checked_add(Value::memory_budget_claim_metadata_bytes()))
            .and_then(|n| n.checked_mul(4))
            .and_then(|n| n.checked_add(4096))
            .ok_or("scalar ownership bound overflows")?;
        // These complete inventories were derived above from this SAME
        // immutable plan. Hash every proof in its original order, including
        // proofs not selected by the capture prefix, without deriving again.
        let frozen_maps = map_owner_words.as_ref().map(|owner|
            super::program_frozen_map_layout::Layout::derive(&plan, owner, &closed_tails))
            .transpose().map_err(str::to_owned)?.unwrap_or_default();
        let identity = super::program_identity::typed_ir_sha256(
            &plan,
            &line_retention_frontiers,
            &selected_joins.regions,
            &line_matrices.regions,
            map_owner_words.as_ref(),
            &frozen_maps,
        );
        Ok(Self {
            plan,
            schemas: Arc::new(artifact.schemas().clone()),
            calls,
            types,
            visits,
            retirements,
            constant_bytes,
            scalar_bytes,
            observation_schema_bytes,
            max_library_inputs,
            window_regions,
            string_map_regions,
            scalar_aggregates,
            map_owner_words,
            frozen_maps,
            map_admission,
            parallel,
            borrowed_strings,
            closed_tails,
            selected_joins,
            line_matrices,
            prefix,
            stack,
            code,
            identity,
        })
    }
    pub fn plan(&self) -> &NativeProgramArtifactPlan {
        &self.plan
    }
    /// Immutable canonical types shared by this build's native/resource metadata.
    pub fn schemas(&self) -> &SchemaTable {
        &self.schemas
    }
    /// Fixed-header association only, not a new semantic or visit authority.
    /// prepare checked full selected-template equality against the actual
    /// artifact/factory once; plan and calls cannot mutate after construction.
    /// The caller funds the existing query-stage header work before lookup.
    pub(super) fn query_stage(&self, collection: u32, ordinal: u32) -> Result<(), &'static str> {
        let step = query_step(&self.plan, collection, ordinal)?;
        match self
            .plan
            .blocks()
            .get(step.site() as usize)
            .map(|b| b.action())
        {
            Some(NativeIterationAction::HistogramQueryStep {
                collection: id,
                step: index,
            }) if *id == collection && *index == ordinal => {}
            _ => return Err("histogram query stage association differs"),
        }
        if self
            .calls
            .get(step.site() as usize)
            .ok_or("histogram query stage outside calls")?
            .is_some()
            != step.template().is_some()
        {
            return Err("histogram query selected stage presence differs");
        }
        Ok(())
    }
    pub const fn stack_bytes(&self) -> u64 {
        self.stack
    }
    pub const fn code_bytes(&self) -> u64 {
        self.code
    }
    pub fn typed_ir_sha256(&self) -> &[u8; 32] {
        &self.identity
    }
}
fn query_step(
    plan: &NativeProgramArtifactPlan,
    collection: u32,
    ordinal: u32,
) -> Result<&mech_engine::native_artifact::NativeHistogramQueryStep, &'static str> {
    let Some(mech_engine::native_artifact::NativeCollectionResult::GuardedHistogramQuery(sink)) =
        plan.collections()
            .get(collection as usize)
            .map(|c| c.result())
    else {
        return Err("histogram query stage lacks selected proof");
    };
    sink.steps()
        .get(ordinal as usize)
        .ok_or("histogram query step outside proof")
}
/// Compiler-only full immutable association proof. This runs for JIT, Object
/// preparation and linked reconstruction, after canonical current-visit proof
/// and SAME actual artifact/catalog/factory preparation of every selected call.
fn validate_query_stages(
    plan: &NativeProgramArtifactPlan,
    calls: &[Option<NativePreparedLibraryCall>],
) -> Result<(), &'static str> {
    if calls.len() != plan.blocks().len() {
        return Err("histogram query call lane length differs");
    }
    for (site, block) in plan.blocks().iter().enumerate() {
        let NativeIterationAction::HistogramQueryStep {
            collection,
            step: ordinal,
        } = block.action()
        else {
            continue;
        };
        let step = query_step(plan, *collection, *ordinal)?;
        if step.site() as usize != site {
            return Err("histogram query original site differs");
        }
        match (step.template(), calls[site].as_ref()) {
            (Some(template), Some(selected)) if selected.template() == template => {}
            (None, None) => {}
            _ => return Err("histogram query selected template differs"),
        }
    }
    Ok(())
}
#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "program_query_stage_tests.rs"]
mod query_stage_tests;
#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "program_string_map_factory_tests.rs"]
mod string_map_factory_tests;
impl super::iteration_runtime::VisitPlan for NativeProgramDescriptor {
    fn entry(&self) -> u32 {
        self.plan.entry()
    }
    fn visits(&self) -> &[NativeIterationVisit] {
        &self.visits
    }
    fn iteration_count(&self) -> usize {
        self.plan.iterations().len()
    }
    fn iteration(&self, index: usize) -> mech_engine::native_artifact::NativeControlLoopRef<'_> {
        self.plan.iterations()[index].as_ref()
    }
}
