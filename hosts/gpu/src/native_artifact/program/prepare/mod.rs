//! Immutable same-plan metadata preparation. No backend or execution authority.
use crate::native_artifact as native;
use crate::native_artifact::NativeValueSlot;
use crate::native_artifact::iteration_abi::NativeIterationVisit;
use crate::native_artifact::program::prepare::retirement::RetirementTables;
use mech_core::{FunctionCatalog, SchemaTable, Value};
use mech_engine::ProgramArtifact;
use mech_engine::native_artifact::{
    NativeIterationAction, NativePreparedLibraryCall, NativeProgramArtifactPlan,
    NativeTextLineRetentionFrontier, prepare_native_program_library_call,
};
use std::sync::Arc;

#[derive(Debug)]
pub(in crate::native_artifact) struct ProgramMetadata {
    pub(in crate::native_artifact) plan: NativeProgramArtifactPlan,
    pub(in crate::native_artifact) schemas: Arc<SchemaTable>,
    // Query entries are prepared from their SAME sink step and fully checked
    // by validate_query_stages before this immutable descriptor can escape.
    // Runtime stage lookup never rebinds or clones those selected templates.
    pub(in crate::native_artifact) calls: Box<[Option<NativePreparedLibraryCall>]>,
    pub(in crate::native_artifact) types: Box<[NativeValueSlot]>,
    pub(in crate::native_artifact) visits: Box<[NativeIterationVisit]>,
    pub(in crate::native_artifact) retirements: RetirementTables,
    pub(in crate::native_artifact) constant_bytes: Box<[u64]>,
    // Derived cache, recomputed from the exact bounds when an image is loaded.
    // It grants no independent authority and adds no serialized field.
    pub(in crate::native_artifact) constant_admission_bytes: u64,
    pub(in crate::native_artifact) scalar_bytes: u64,
    pub(in crate::native_artifact) observation_schema_bytes: u64,
    pub(in crate::native_artifact) max_library_inputs: usize,
    // Derived from this SAME immutable plan; callers cannot inject a region
    // from another artifact or rebind one to a different selected factory.
    // Same-plan last-use candidates are recomputed on every image load. The
    // runtime still checks exact current owners and exclusive core storage.
    pub(in crate::native_artifact) unsigned_buffer_calls:
        Box<[Option<mech_engine::native_artifact::NativeUnsignedBufferCall>]>,
    // Pure derived cache. Recomputed from the validated plan on image load;
    // it is neither serialized authority nor another preparation pipeline.
    pub(in crate::native_artifact) uniform_word_collections:
        Box<[mech_engine::native_artifact::NativeUniformWordCollection]>,
    pub(in crate::native_artifact) window_regions:
        Box<[mech_engine::native_artifact::NativeWindowRegion]>,
    pub(in crate::native_artifact) string_map_regions:
        Box<[mech_engine::native_artifact::NativeStringMapRegion]>,
    pub(in crate::native_artifact) scalar_aggregates:
        mech_engine::native_artifact::NativeScalarAggregates,
    // Complete same-plan candidate layout, preserved through standalone images.
    // Merely carrying this metadata does not select retained runtime storage.
    pub(in crate::native_artifact) map_owner_words:
        Option<mech_engine::native_artifact::NativeMapOwnerWords>,
    pub(in crate::native_artifact) frozen_maps: native::program::prepare::frozen_map::Layout,
    pub(in crate::native_artifact) map_admission: native::program::prepare::map_admission::Layout,
    pub(in crate::native_artifact) parallel: native::program::prepare::parallel::Layout,
    pub(in crate::native_artifact) borrowed_strings: native::program::prepare::borrowed::Layout,
    pub(in crate::native_artifact) closed_tails: native::program::prepare::tail::Layout,
    pub(in crate::native_artifact) selected_joins: native::program::prepare::selected_join::Layout,
    pub(in crate::native_artifact) line_matrices: native::program::prepare::line_matrix::Layout,
    pub(in crate::native_artifact) prefix: Option<native::program::prepare::prefix::Region>,
}

/// Exactly one owned plan and its derived metadata, shared by emission and
/// source-backed reconstruction. Complete frontiers are identity scratch.
pub(in crate::native_artifact) struct PreparedNativeProgram {
    metadata: ProgramMetadata,
    complete_frontiers: Box<[NativeTextLineRetentionFrontier]>,
}
impl PreparedNativeProgram {
    pub(in crate::native_artifact) fn prepare(
        artifact: &ProgramArtifact,
        catalog: &FunctionCatalog,
        plan: NativeProgramArtifactPlan,
    ) -> Result<Self, String> {
        if artifact.revision() != plan.program_revision() {
            return Err("mixed canonical revision mismatch".into());
        }
        plan.validate_current_visits().map_err(str::to_owned)?;
        let facts = plan.facts();
        let scalar_aggregates = facts.scalar_aggregates();
        let parallel =
            native::program::prepare::parallel::Layout::derive(&facts, &scalar_aggregates)
                .map_err(str::to_owned)?;
        let retirements =
            RetirementTables::derive(&plan, &scalar_aggregates).map_err(str::to_owned)?;
        let completion = (plan.transport_output_count() as u64)
            .checked_mul(64)
            .and_then(|n| n.checked_add((plan.iterations().len() as u64).checked_mul(8)?))
            .and_then(|n| n.checked_add(256))
            .ok_or("program completion overflow")?;
        let borrowed_strings =
            native::program::prepare::borrowed::Layout::derive(&plan, &scalar_aggregates)?;
        let string_map_regions = plan.string_map_regions().into_boxed_slice();
        let unsigned_buffer_calls = facts.unsigned_buffer_calls().into_boxed_slice();
        let selected_joins =
            native::program::prepare::selected_join::Layout::derive(&plan, &string_map_regions)?;
        let line_matrices = native::program::prepare::line_matrix::Layout::derive(&plan)?;
        let mut analysis_visits = native::iteration_abi::control_visits(
            plan.blocks(),
            plan.iterations().iter().map(Into::into),
            0,
        )
        .map_err(str::to_owned)?;
        let mut visits = native::iteration_abi::control_visits(
            plan.blocks(),
            plan.iterations().iter().map(Into::into),
            completion,
        )
        .map_err(str::to_owned)?;
        selected_joins
            .fund_visit_checks(&mut analysis_visits)
            .map_err(str::to_owned)?;
        selected_joins
            .fund_visit_checks(&mut visits)
            .map_err(str::to_owned)?;
        // Derive this proof once from the SAME immutable plan. Both layout and
        // identity inspect the complete result, including unsupported/multiple
        // frontiers; no caller-provided or persistent proof cache is consulted.
        let line_retention_frontiers = facts.text_line_retention_frontiers();
        let prefix = native::program::prepare::prefix::derive_with_frontiers(
            &plan,
            &borrowed_strings,
            &analysis_visits,
            &scalar_aggregates,
            &line_retention_frontiers,
        )?;
        compose_work(
            &mut analysis_visits,
            &mut visits,
            |site| {
                borrowed_strings
                    .extra_work(site)?
                    .checked_add(native::program::prepare::prefix::extra_work(
                        prefix.as_ref(),
                        site,
                    ))
                    .ok_or("borrowed String visit work overflow")
            },
            "borrowed String visit work overflow",
        )?;
        let map_owner_words = facts.map_owner_words().map_err(str::to_owned)?;
        compose_work(
            &mut analysis_visits,
            &mut visits,
            |site| native::program::prepare::map_owner::extra_work(&map_owner_words, site),
            "map owner visit work overflow",
        )?;
        let closed_tails = native::program::prepare::tail::Layout::derive(
            &plan,
            &borrowed_strings,
            &analysis_visits,
            &scalar_aggregates,
            &map_owner_words,
        )?;
        compose_work(
            &mut analysis_visits,
            &mut visits,
            |site| Ok(closed_tails.extra_work(site)),
            "closed Tail work overflow",
        )?;
        let map_admission = native::program::prepare::map_admission::Layout::derive(
            &plan,
            &map_owner_words,
            &analysis_visits,
        )?;
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
        query_stages::validate_query_stages(&plan, &calls).map_err(str::to_owned)?;
        let uniform_word_collections = facts.uniform_unsigned_collections().into_boxed_slice();
        let window_regions = plan.window_regions().into_boxed_slice();
        let occupied_frame =
            frame_prefix_words(&plan, &scalar_aggregates, &borrowed_strings, &closed_tails)
                .ok_or("map owner frame prefix overflow")?;
        let map_owner_words = map_owner_words
            .after_frame_words(occupied_frame)
            .map_err(str::to_owned)?;
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
            .max(
                map_owner_words
                    .as_ref()
                    .map_or(0, native::program::prepare::map_owner::initial_words),
            )
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
        let constant_admission_bytes = constant_admission_total(&constant_bytes)?;
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
        let frozen_maps = map_owner_words
            .as_ref()
            .map(|owner| {
                native::program::prepare::frozen_map::Layout::derive(&plan, owner, &closed_tails)
            })
            .transpose()
            .map_err(str::to_owned)?
            .unwrap_or_default();
        let (scalar_aggregates, line_retention_frontiers) = facts.into_preparation_inventories();
        let metadata = ProgramMetadata {
            plan,
            schemas: Arc::new(artifact.schemas().clone()),
            calls,
            types,
            visits,
            retirements,
            constant_bytes,
            constant_admission_bytes,
            scalar_bytes,
            observation_schema_bytes,
            max_library_inputs,
            uniform_word_collections,
            unsigned_buffer_calls,
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
        };
        Ok(Self {
            metadata,
            complete_frontiers: line_retention_frontiers.into_boxed_slice(),
        })
    }

    pub(in crate::native_artifact) fn plan(&self) -> &NativeProgramArtifactPlan {
        &self.metadata.plan
    }
    pub(in crate::native_artifact) fn metadata(&self) -> &ProgramMetadata {
        &self.metadata
    }
    pub(in crate::native_artifact) fn complete_frontiers(
        &self,
    ) -> &[NativeTextLineRetentionFrontier] {
        &self.complete_frontiers
    }
    pub(in crate::native_artifact) fn into_metadata(self) -> ProgramMetadata {
        self.metadata
    }
}

// Composition is shared; analysis completion and charged completion remain
// distinct. Evaluate each supplement once, in the established order.
fn compose_work(
    analysis: &mut [NativeIterationVisit],
    charged: &mut [NativeIterationVisit],
    mut extra: impl FnMut(usize) -> Result<u64, &'static str>,
    overflow: &'static str,
) -> Result<(), &'static str> {
    debug_assert_eq!(analysis.len(), charged.len());
    for (site, (analysis, charged)) in analysis.iter_mut().zip(charged).enumerate() {
        let extra = extra(site)?;
        analysis.work = analysis.work.checked_add(extra).ok_or(overflow)?;
        charged.work = charged.work.checked_add(extra).ok_or(overflow)?;
        debug_assert!(analysis.is_completion() || analysis.work == charged.work);
    }
    Ok(())
}

// The sole canonical frame-prefix formula. Owner relocation occurs only in
// preparation; later consumers inspect this existing assignment.
fn frame_prefix_words(
    plan: &NativeProgramArtifactPlan,
    aggregates: &mech_engine::native_artifact::NativeScalarAggregates,
    borrowed: &native::program::prepare::borrowed::Layout,
    closed: &native::program::prepare::tail::Layout,
) -> Option<usize> {
    plan.values()
        .len()
        .checked_add(plan.iterations().len())
        .and_then(|n| n.checked_add(aggregates.words()))
        .and_then(|n| n.checked_add(borrowed.words))
        .and_then(|n| n.checked_add(closed.words))
}
impl ProgramMetadata {
    pub(in crate::native_artifact) fn local_words(&self) -> Result<usize, &'static str> {
        let prefix = frame_prefix_words(
            &self.plan,
            &self.scalar_aggregates,
            &self.borrowed_strings,
            &self.closed_tails,
        )
        .ok_or("local count overflow")?;
        self.map_owner_words
            .as_ref()
            .map_or(Ok(prefix), native::program::prepare::map_owner::local_words)
    }
}

pub(in crate::native_artifact) mod retirement;

pub(in crate::native_artifact) fn constant_admission_total(bounds: &[u64]) -> Result<u64, String> {
    bounds
        .iter()
        .try_fold(0u64, |sum, &bound| sum.checked_add(bound))
        .ok_or_else(|| "constant admission total overflows".to_owned())
}

pub(in crate::native_artifact) mod borrowed;
pub(in crate::native_artifact) mod frozen_map;
pub(in crate::native_artifact) mod line_matrix;
pub(in crate::native_artifact) mod map_admission;
pub(in crate::native_artifact) mod map_owner;
pub(in crate::native_artifact) mod packed_map;
pub(in crate::native_artifact) mod parallel;
pub(in crate::native_artifact) mod prefix;
pub(in crate::native_artifact) mod selected_join;
pub(in crate::native_artifact) mod tail;
pub(in crate::native_artifact) mod window_histogram;

pub(in crate::native_artifact) mod query_stages;
