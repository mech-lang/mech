//! Frozen selection plus pre-admitted live-geometry binding of ONE leaf.
//! No graph scheduling, native visit permission or alternate leaf semantics.
use super::{
    Error, NativeCollectionCall, NativeProgramLibraryTemplate, bind_current, current_template,
    finite_limits,
    invocation::{
        NativeCollectionInvocationError as InvokeError, NativePlanCacheState, metadata_envelope_for,
    },
};
use crate::{
    ProgramArtifact,
    resident::{budget::NativeLibraryWorkBudget, general::live},
};
use mech_core::{
    CurrentMemoryFootprint, FunctionCatalog, ManagedMemoryBudget, ManagedMemoryReservation,
    ManagedNodeReservation, MemoryBudgetLimits, MemoryPlanError, ResidentKernelFactory,
    ResourceDemand, SchemaTable, Value,
};
use std::sync::Arc;

fn composite_shape_from_children(template: &NativeProgramLibraryTemplate) -> bool {
    matches!(
        template.primitive(),
        crate::resident::native_collection_semantics::NativeCollectionPrimitive::CompositePack(_)
    ) && !template.output().dimension_parameters().is_empty()
        && matches!(
            template.output().body(),
            mech_core::SchemaBody::Tuple(_) | mech_core::SchemaBody::Record(_)
        )
}

fn buffer_shape_follows_input(template: &NativeProgramLibraryTemplate) -> bool {
    #[cfg(feature = "sequence_buffers")]
    return matches!(
        template.primitive(),
        crate::native_artifact::NativeCollectionPrimitive::SequenceReserve
            | crate::native_artifact::NativeCollectionPrimitive::SequenceAppend
            | crate::native_artifact::NativeCollectionPrimitive::SequenceAppendLeBytes
            | crate::native_artifact::NativeCollectionPrimitive::SequenceAppendSublist
    );
    #[cfg(not(feature = "sequence_buffers"))]
    {
        let _ = template;
        false
    }
}

fn record_selector(
    template: &NativeProgramLibraryTemplate,
    inputs: &[Option<Value>],
) -> Option<u64> {
    if template.primitive() != crate::native_artifact::NativeCollectionPrimitive::RecordFieldAccess
    {
        return None;
    }
    match inputs.get(1)?.as_ref()?.data() {
        mech_core::ValueData::Id(id) => Some(*id),
        _ => None,
    }
}

// Reuse the existing validated descriptor as immutable schema/type metadata.
// A neutral shape is only a preparation witness, never a runtime argument.
// Schemas that reject it retain the ordinary live constructor and error order.
fn descriptor_templates(
    template: &NativeProgramLibraryTemplate,
) -> Option<Box<[mech_core::ResolvedValueDescriptor]>> {
    template
        .inputs()
        .iter()
        .chain(core::iter::once(template.output()))
        .map(|schema| {
            let shape = schema
                .instantiate_shape(vec![0; schema.dimension_parameters().len()].into())
                .ok()?;
            mech_core::ResolvedValueDescriptor::from_schema(schema.clone(), shape).ok()
        })
        .collect()
}

#[cfg(any(
    feature = "pure_text",
    feature = "map_ops",
    feature = "sequence_buffers"
))]
#[path = "native_collection_prepared_image.rs"]
mod image;

#[cfg(feature = "sequence_buffers")]
#[path = "native_collection_buffer_frames.rs"]
mod buffer_frames;
#[cfg(feature = "sequence_buffers")]
#[path = "native_collection_frame_admission.rs"]
mod frame_admission;
#[cfg(feature = "sequence_buffers")]
#[path = "native_collection_frame_composition.rs"]
mod frame_composition;
#[cfg(feature = "sequence_buffers")]
#[path = "native_collection_frame_shape.rs"]
mod frame_shape;
#[path = "native_collection_histogram.rs"]
mod histogram;
#[cfg(feature = "sequence_buffers")]
#[path = "native_collection_record_selector.rs"]
mod record_selector;
#[path = "native_collection_record_words.rs"]
mod record_words;
#[path = "native_collection_string_map_entry.rs"]
mod string_map_entry;
#[path = "native_collection_string_span.rs"]
mod string_span;
#[cfg(feature = "sequence_buffers")]
#[path = "native_collection_unsigned_buffers.rs"]
mod unsigned_buffers;
#[cfg(feature = "pure_text")]
#[path = "native_collection_unsigned_lengths.rs"]
mod unsigned_lengths;
#[path = "native_collection_word_reads.rs"]
mod word_reads;
#[cfg(feature = "sequence_buffers")]
#[path = "native_collection_word_updates.rs"]
mod word_updates;

pub use string_span::NativeSelectedStringSpanSelector;
#[cfg(feature = "pure_text")]
#[path = "native_collection_private_string.rs"]
mod private_string;
#[cfg(feature = "pure_text")]
pub use private_string::NativePrivateStringBuffer;
#[cfg(feature = "pure_text")]
#[path = "native_collection_selected_join.rs"]
mod selected_join;
#[cfg(feature = "pure_text")]
pub use selected_join::NativeSelectedStringMatrix;
#[cfg(feature = "pure_text")]
#[path = "native_collection_text_lines.rs"]
mod text_lines;
#[cfg(feature = "pure_text")]
pub use text_lines::{
    NativeTextLineCaptureFacts, NativeTextLineChunks, NativeTextLineJoinChunk,
    NativeTextLineMatrix, NativeTextLineMatrixChoice, NativeTextLineWindow,
};

/// Compiler-owned selection/immutable schemas, not a bound runtime call or
/// authority to enter its source site. No artifact/catalog traversal occurs
/// during invoke_current. Its complete temporary binding is admitted first.
#[derive(Debug, serde::Serialize)]
pub struct NativePreparedLibraryCall {
    template: NativeProgramLibraryTemplate,
    // The enclosing descriptor exports the SAME canonical schema table once.
    #[serde(skip)]
    schemas: Arc<SchemaTable>,
    // A build image names the selected implementation through template, never
    // a process address. Loading must resolve its exact build-owned binding.
    #[serde(skip)]
    factory: ResidentKernelFactory,
    #[serde(skip)]
    descriptor_templates: Option<Box<[mech_core::ResolvedValueDescriptor]>>,
    binding_bytes: u64,
    binding_work: u64,
    binding_nodes: u64,
    output_parameters: usize,
    input_keys: Box<[mech_core::SchemaKey]>,
    output_key: mech_core::SchemaKey,
    lookup_work: u64,
    output_follows_input: bool,
    #[serde(skip)]
    cache_identity: Arc<()>,
    #[cfg(feature = "sequence_buffers")]
    #[serde(skip)]
    frame_geometry: Option<Box<buffer_frames::FrameGeometry>>,
    #[cfg(feature = "sequence_buffers")]
    #[serde(skip)]
    record_selector: Option<record_selector::RecordSelector>,
    #[cfg(feature = "sequence_buffers")]
    #[serde(skip)]
    frame_composition: Option<Box<frame_composition::Composition>>,
    span_selector_metadata_bytes: u64,
    span_selector_work: u64,
    #[cfg(feature = "pure_text")]
    selection_shape_bounds: (u64, u64),
    #[cfg(feature = "pure_text")]
    selection_range_shape_bounds: (u64, u64),
}

/// One owning native invocation's lazy per-site binding slot. This retains
/// neither arguments nor results and grants no permission to visit a site.
/// It is intentionally non-Clone and cannot migrate to another work ledger.
#[derive(Debug, Default)]
pub struct NativePreparedLibraryCallCache {
    binding: Option<Box<CachedBinding>>,
}

#[derive(Debug)]
struct CachedBinding {
    // Retire all bound metadata before releasing its admitted capacity.
    plans: NativePlanCacheState,
    call: Option<NativeCollectionCall>,
    identity: Arc<()>,
    limits: MemoryBudgetLimits,
    work: NativeLibraryWorkBudget,
    projection_count: Option<u64>,
    binding_nodes: u64,
    projection_index: crate::resident::text_index::snapshot::Slot,
    _reservation: ManagedMemoryReservation,
    _nodes: ManagedNodeReservation,
}

impl NativePreparedLibraryCallCache {
    /// Read this cache's admitted metadata demand without inspecting Values,
    /// rebuilding plans or renewing a call allowance. This grants no selected
    /// site/visit permission. Canonical payloads belong to the caller's separate
    /// snapshot census; this cache retains no input or output Values.
    pub fn retained_metadata_demand(
        &self,
        work: &NativeLibraryWorkBudget,
    ) -> Result<ResourceDemand, InvokeError> {
        let Some(binding) = self.binding.as_ref() else {
            work.charge(256, 0)?;
            return Ok(ResourceDemand::default());
        };
        if !binding.work.shares_account(work)
            || binding._reservation.budget() != work.memory_budget()
        {
            return Err(InvokeError::InvalidInput);
        }
        work.charge(256, 0)?;
        // Fixed binding metadata is one retained owner, rather than one node
        // per reserved byte. Recursive conversion plans retain their prepared
        // staging-node witness; dynamic indexes have their own witness below.
        let metadata = binding._reservation.capacity_bytes();
        let mut demand = ResourceDemand {
            turn_peak_bytes: metadata
                .checked_add(binding.projection_index.bytes())
                .ok_or(InvokeError::InvalidInput)?,
            retained_nodes: 1u64
                .checked_add(if binding.call.is_some() {
                    binding.binding_nodes
                } else {
                    0
                })
                .and_then(|n| n.checked_add(binding.projection_index.nodes()))
                .ok_or(InvokeError::InvalidInput)?,
            ..ResourceDemand::default()
        };
        if let NativePlanCacheState::Cached(plans) = &binding.plans {
            let plans = plans.retained_metadata_demand(work)?;
            demand.turn_peak_bytes = demand
                .turn_peak_bytes
                .checked_add(plans.turn_peak_bytes)
                .ok_or(InvokeError::InvalidInput)?;
            demand.retained_nodes = demand
                .retained_nodes
                .checked_add(plans.retained_nodes)
                .ok_or(InvokeError::InvalidInput)?;
        }
        Ok(demand)
    }
}

/// Prepare only the actual current private-witness-selected site. This is
/// compiler/setup allocation; it does not enter a work scope or execute a leaf.
pub fn prepare_native_program_library_call(
    artifact: &ProgramArtifact,
    template: &NativeProgramLibraryTemplate,
    catalog: &FunctionCatalog,
) -> Result<NativePreparedLibraryCall, Error> {
    let template = current_template(artifact, template, catalog)?;
    let node = template.origin().node();
    let key = template.selected_key();
    let factory = catalog
        .resident_factory(&key.module_path, &key.operation_name)
        .ok_or(Error::MissingSemanticWitness { node })?
        .factory();
    let ports = (template.inputs().len() as u64)
        .checked_add(1)
        .ok_or_else(|| super::invalid(node))?;
    // The common envelope covers the original shallow selected families.
    // Recursive convert/set additionally funds its SAME semantic plan trees
    // with the shared schema-depth/evidence bound below, before bind_values.
    // Numeric ranges, positional matrix selection and scalar-key map lookup
    // contain at most rank-two scalar matrices or two atomic map children;
    // selector state has fixed metadata size. Tuple/record packs add one input
    // port per component. Their current-shape resolver closes component schema
    // trees and stages dimension witnesses; these are bounded by the port-schema
    // clone envelope below, independently of the buffers' element counts.
    // Build/NoAdditionalScratch ranges add no implementation scratch entries;
    // CanonicalFinalize String selection/matrix conversion uses the same output entry family.
    // Tests assert the actual NativeHost plan stays within this entry bound.
    // Their deferred AnyValue planning has <=2 allocations/port, <=8 additional
    // output stage/scratch allocations, one transaction and one witness/port.
    // Reserve extra entry slack too. Numeric extents do not create plan
    // entries; actual port schema trees (including tuple/record children and
    // dimension expressions) retain their deep-clone bound inside the common
    // envelope. Only the immutable full schema arena is shared.
    let entries = ports
        .checked_mul(3)
        .and_then(|n| n.checked_add(16))
        .ok_or_else(|| super::invalid(node))?;
    let common_binding_bytes = metadata_envelope_for(
        template
            .inputs()
            .iter()
            .chain(core::iter::once(template.output())),
        ports,
        entries,
    )
    .ok_or_else(|| super::invalid(node))?;
    #[cfg(feature = "convert")]
    let (conversion_bytes, binding_nodes, conversion_work) = if matches!(
        template.primitive(),
        crate::resident::native_collection_semantics::NativeCollectionPrimitive::MatrixToSetConversion
    ) {
        let bound =
            mech_core::conversion_schema_staging_bound(&template.inputs()[0], template.output())
                .ok_or_else(|| super::invalid(node))?;
        (bound.plan_bytes, bound.plan_nodes, bound.plan_work)
    } else {
        (0, 0, 0)
    };
    #[cfg(not(feature = "convert"))]
    let (conversion_bytes, binding_nodes, conversion_work) = (0, 0, 0);
    let binding_bytes = common_binding_bytes
        .checked_add(conversion_bytes)
        .ok_or_else(|| super::invalid(node))?;
    // Covers construction/traversal and eventual complete metadata retirement
    // before either occurs, including error paths. Never a per-call refill.
    let binding_work = binding_bytes
        .checked_mul(8)
        .and_then(|n| n.checked_add(256))
        .and_then(|n| n.checked_add(conversion_work))
        .ok_or_else(|| super::invalid(node))?;
    let output_parameters = template.output().dimension_parameters().len();
    // Schema::key encodes and hashes its whole tree. Retain immutable keys
    // here, not inside an ostensibly constant-work runtime cache lookup.
    let input_keys = template
        .inputs()
        .iter()
        .map(|schema| schema.key())
        .collect();
    let output_key = template.output().key();
    // Lookup reads only borrowed validated shape tapes. Fund their complete
    // declared lengths, including the prior-output/zero-default signature.
    let parameters = template
        .inputs()
        .iter()
        .chain(core::iter::once(template.output()))
        .try_fold(0u64, |total, schema| {
            total.checked_add(u64::try_from(schema.dimension_parameters().len()).ok()?)
        })
        .ok_or_else(|| super::invalid(node))?;
    let lookup_work = ports
        .checked_mul(64)
        .and_then(|n| n.checked_add(parameters.checked_mul(16)?))
        .and_then(|n| n.checked_add(256))
        .ok_or_else(|| super::invalid(node))?;
    let output_follows_input = composite_shape_from_children(&template)
        || buffer_shape_follows_input(&template)
        || matches!(
            template.contract(),
            mech_core::ResolvedOperationContract::Declared(contract)
                if matches!(&contract.outputs[0].construction,
                    mech_core::OutputConstruction::FullWrite {
                        shape: mech_core::ShapeRule::SameAsInput { .. }
                    } | mech_core::OutputConstruction::ReadModifyWrite { .. })
        );
    let (span_selector_metadata_bytes, span_selector_work) =
        string_span::selector_geometry_bounds(&template).ok_or_else(|| super::invalid(node))?;
    #[cfg(feature = "pure_text")]
    let selection_shape_bounds = if matches!(
        template.primitive(),
        crate::native_artifact::NativeCollectionPrimitive::MatrixRangeAccess
            | crate::native_artifact::NativeCollectionPrimitive::TextLines
    ) {
        string_span::selector_schema_geometry_bounds(template.output())
            .ok_or_else(|| super::invalid(node))?
    } else {
        (0, 0)
    };
    #[cfg(feature = "pure_text")]
    let selection_range_shape_bounds = if template.primitive()
        == crate::native_artifact::NativeCollectionPrimitive::MatrixRangeAccess
    {
        template
            .inputs()
            .get(1)
            .and_then(string_span::selector_schema_geometry_bounds)
            // Optional strategy metadata must not reject other ordinary
            // MatrixRangeAccess programs. Eligible Range output schemas have
            // already passed this same bound during their preparation.
            .unwrap_or((0, 0))
    } else {
        (0, 0)
    };
    #[cfg(feature = "sequence_buffers")]
    let frame_geometry =
        buffer_frames::FrameGeometry::derive(&template, artifact.schemas(), binding_bytes);
    #[cfg(feature = "sequence_buffers")]
    let record_selector = record_selector::RecordSelector::derive(&template);
    let descriptor_templates = descriptor_templates(&template);
    Ok(NativePreparedLibraryCall {
        template,
        schemas: Arc::new(artifact.schemas().clone()),
        factory,
        descriptor_templates,
        binding_bytes,
        binding_work,
        binding_nodes,
        output_parameters,
        input_keys,
        output_key,
        lookup_work,
        output_follows_input,
        cache_identity: Arc::new(()),
        #[cfg(feature = "sequence_buffers")]
        frame_geometry,
        #[cfg(feature = "sequence_buffers")]
        record_selector,
        #[cfg(feature = "sequence_buffers")]
        frame_composition: None,
        span_selector_metadata_bytes,
        span_selector_work,
        #[cfg(feature = "pure_text")]
        selection_shape_bounds,
        #[cfg(feature = "pure_text")]
        selection_range_shape_bounds,
    })
}

impl NativePreparedLibraryCall {
    pub fn template(&self) -> &NativeProgramLibraryTemplate {
        &self.template
    }
    pub const fn binding_metadata_bytes(&self) -> u64 {
        self.binding_bytes
    }
    pub const fn binding_work(&self) -> u64 {
        self.binding_work
    }

    /// Constant-size selected range semantics for a certified native source
    /// strategy, NOT a kernel invocation or a materialized Matrix. The caller
    /// prepays binding_work and reserves binding_metadata_bytes on the SAME
    /// invocation account before this schema/shape traversal or allocation.
    #[cfg(any(
        feature = "pure_text",
        feature = "map_ops",
        feature = "sequence_buffers"
    ))]
    pub fn selected_u64_range_cardinality(&self, start: u64, end: u64) -> Result<u64, InvokeError> {
        let count = self.selected_u64_range_count(start, end)?;
        self.validate_selected_u64_range_shape(count)?;
        Ok(count)
    }

    // Endpoint arithmetic remains fresh even when a constructor-private
    // selector carries an already checked shape for the same cardinality.
    #[cfg(any(
        feature = "pure_text",
        feature = "map_ops",
        feature = "sequence_buffers"
    ))]
    fn selected_u64_range_count(&self, start: u64, end: u64) -> Result<u64, InvokeError> {
        use crate::native_artifact::NativeCollectionPrimitive as Primitive;
        use mech_core::{IntegerWidth, SchemaBody, snapshot::ValueDataDraft};
        let inclusive = match self.template.primitive() {
            Primitive::RangeInclusive => true,
            Primitive::RangeExclusive => false,
            _ => return Err(InvokeError::InvalidInput),
        };
        let scalar = SchemaBody::UnsignedInteger(IntegerWidth::W64);
        if self.template.inputs().len() != 2
            || self
                .template
                .inputs()
                .iter()
                .any(|ty| !ty.dimension_parameters().is_empty() || ty.body() != &scalar)
        {
            return Err(InvokeError::InvalidInput);
        }
        let count = crate::resident::numeric::canonical_range_draft_cardinality(
            &[ValueDataDraft::U64(start), ValueDataDraft::U64(end)],
            inclusive,
            false,
        )
        .filter(|count| *count != 0)
        .ok_or(InvokeError::Kernel(
            mech_core::ResidentKernelError::InvalidInput,
        ))?;
        u64::try_from(count)
            .map_err(|_| InvokeError::Kernel(mech_core::ResidentKernelError::InvalidShape))
    }

    #[cfg(any(
        feature = "pure_text",
        feature = "map_ops",
        feature = "sequence_buffers"
    ))]
    fn validate_selected_u64_range_shape(&self, count: u64) -> Result<(), InvokeError> {
        // Exactly the ordinary range's shape resolver, including parameter
        // bounds and fixed cardinality constraints. Never silently retag it.
        mech_core::shape_for_resolved_extents(self.template.output(), &[1, count])
            .map_err(|_| InvokeError::Kernel(mech_core::ResidentKernelError::InvalidShape))?;
        // Preserve the ordinary materializing range's physical Layout bound
        // even when the certified source strategy never constructs its Matrix.
        crate::resident::numeric::canonical_u64_range_selector_representation(count)?;
        Ok(())
    }

    /// Maintained TextJoin mechanics after a constructor-private composed
    /// strategy has prepaid the append/growth. Never invokes a fake Matrix.
    pub fn selected_text_join_append(
        &self,
        target: &mut String,
        separator: &str,
        has_previous: bool,
        value: &str,
    ) -> Result<(), InvokeError> {
        #[cfg(feature = "pure_text")]
        if self.template.primitive() == crate::native_artifact::NativeCollectionPrimitive::TextJoin
        {
            // The native sink already prepaid fixed per-append metadata work.
            // Fail before either push if its promised capacity is absent; this
            // public selected helper must never allocate as an implicit repair.
            let bytes = target
                .len()
                .checked_add(if has_previous { separator.len() } else { 0 })
                .and_then(|n| n.checked_add(value.len()))
                .ok_or(InvokeError::Kernel(
                    mech_core::ResidentKernelError::InvalidShape,
                ))?;
            if bytes > target.capacity() {
                return Err(InvokeError::Kernel(
                    mech_core::ResidentKernelError::InvalidShape,
                ));
            }
            crate::resident::text_ops::append_joined(target, separator, has_previous, value);
            return Ok(());
        }
        let _ = (target, separator, has_previous, value);
        Err(InvokeError::InvalidInput)
    }

    fn validate_values(
        &self,
        inputs: &[Option<Value>],
        output: &Option<Value>,
        memory: &ManagedMemoryBudget,
    ) -> Result<(), InvokeError> {
        let mech_core::ResolvedOperationContract::Declared(contract) = self.template.contract()
        else {
            return Err(InvokeError::InvalidInput);
        };
        for ((value, key), port) in inputs.iter().zip(&self.input_keys).zip(&contract.inputs) {
            let value = value.as_ref().ok_or(InvokeError::InvalidInput)?;
            if value.schema() != port.schema || value.schema_key() != *key {
                return Err(InvokeError::InvalidInput);
            }
            if value.memory_budget_retained_bytes(memory).is_none() {
                return Err(InvokeError::UnadmittedValue);
            }
        }
        if let Some(value) = output.as_ref() {
            if value.schema() != contract.outputs[0].schema || value.schema_key() != self.output_key
            {
                return Err(InvokeError::InvalidInput);
            }
            if value.memory_budget_retained_bytes(memory).is_none() {
                return Err(InvokeError::UnadmittedValue);
            }
        }
        Ok(())
    }

    // Called only after the enclosing binding reservation and work debit.
    // Keep the SAME live shape resolver on both first cached binding and
    // parameterized calls; empty shape tapes do not waive consistency checks.
    fn bind_values(
        &self,
        inputs: &[Option<Value>],
        output: &Option<Value>,
        limits: MemoryBudgetLimits,
    ) -> Result<NativeCollectionCall, InvokeError> {
        let mech_core::ResolvedOperationContract::Declared(contract) = self.template.contract()
        else {
            return Err(InvokeError::InvalidInput);
        };
        let parameters = inputs
            .iter()
            .map(|value| value.as_ref().unwrap().shape().parameter_values())
            .collect::<Vec<_>>();
        let current_shape;
        let empty_shape;
        let shape_input = match &contract.outputs[0].construction {
            mech_core::OutputConstruction::FullWrite {
                shape: mech_core::ShapeRule::SameAsInput { input },
            } => Some(*input),
            mech_core::OutputConstruction::ReadModifyWrite { base_input, .. } => Some(*base_input),
            _ => None,
        };
        let output_parameters = if buffer_shape_follows_input(&self.template) {
            let source = inputs
                .first()
                .and_then(Option::as_ref)
                .ok_or(InvokeError::InvalidInput)?;
            let mech_core::ValueData::Matrix(matrix) = source.data() else {
                return Err(InvokeError::InvalidInput);
            };
            let count = matrix.elements().len() as u64;
            #[cfg(feature = "sequence_buffers")]
            let count = match self.template.primitive() {
                crate::native_artifact::NativeCollectionPrimitive::SequenceAppend => {
                    count.checked_add(1).ok_or(InvokeError::InvalidInput)?
                }
                crate::native_artifact::NativeCollectionPrimitive::SequenceAppendSublist => count
                    .checked_add(
                        prepared_sublist_count(&self.template, inputs)
                            .ok_or(InvokeError::InvalidInput)?,
                    )
                    .ok_or(InvokeError::InvalidInput)?,
                crate::native_artifact::NativeCollectionPrimitive::SequenceAppendLeBytes => {
                    let Some(mech_core::ValueData::U8(bytes @ 0..=8)) =
                        inputs.get(2).and_then(Option::as_ref).map(Value::data)
                    else {
                        return Err(InvokeError::InvalidInput);
                    };
                    count
                        .checked_add(u64::from(*bytes))
                        .ok_or(InvokeError::InvalidInput)?
                }
                _ => count,
            };
            current_shape =
                mech_core::shape_for_resolved_extents(self.template.output(), &[1, count])
                    .map_err(|_| InvokeError::InvalidInput)?;
            current_shape.parameter_values()
        } else if composite_shape_from_children(&self.template) {
            // Each child's parameter tape belongs to its own schema. Infer
            // the aggregate's CURRENT shape before binding, including after a
            // prior result with different extents. This runs inside the same
            // prepaid schema/binding envelope as bind_current below.
            let children = inputs
                .iter()
                .map(|value| {
                    let value = value.as_ref().unwrap();
                    (value.schema(), value.shape().clone())
                })
                .collect::<Vec<_>>();
            current_shape = mech_core::snapshot::CompositeSnapshotConstructor::shape_for_children(
                contract.outputs[0].schema,
                &children,
                &self.schemas,
            )
            .map_err(|_| InvokeError::InvalidInput)?;
            current_shape.parameter_values()
        } else if let Some(input) = shape_input {
            let index = usize::from(input);
            let source = inputs
                .get(index)
                .and_then(Option::as_ref)
                .ok_or(InvokeError::InvalidInput)?;
            let schema = self
                .template
                .inputs()
                .get(index)
                .ok_or(InvokeError::InvalidInput)?;
            let dimensions = match schema.body() {
                mech_core::SchemaBody::Matrix { dimensions, .. } => dimensions.as_ref(),
                _ => &[],
            };
            let extents = dimensions
                .iter()
                .map(|dimension| source.shape().resolve_dimension(dimension))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| InvokeError::InvalidInput)?;
            // SameAsInput describes the CURRENT logical extents, not
            // the parameter numbering of another schema or a stale
            // published output. Reuse the canonical shape resolver.
            current_shape = mech_core::shape_for_resolved_extents(self.template.output(), &extents)
                .map_err(|_| InvokeError::InvalidInput)?;
            current_shape.parameter_values()
        } else if let Some(value) = output.as_ref() {
            value.shape().parameter_values()
        } else {
            empty_shape = vec![0; self.output_parameters];
            &empty_shape
        };
        bind_current(
            &self.schemas,
            self.template.clone(),
            self.factory,
            self.descriptor_templates.as_deref(),
            &parameters,
            output_parameters,
            record_selector(&self.template, inputs),
            limits,
        )
        .map_err(InvokeError::Binding)
    }

    fn geometry_matches(
        &self,
        call: &NativeCollectionCall,
        inputs: &[Option<Value>],
        output: &Option<Value>,
    ) -> bool {
        #[cfg(feature = "sequence_buffers")]
        if self.template.primitive()
            == crate::native_artifact::NativeCollectionPrimitive::SequenceAppendSublist
        {
            let Some(mech_core::ValueData::Matrix(source)) =
                inputs.first().and_then(Option::as_ref).map(Value::data)
            else {
                return false;
            };
            let Some(count) = prepared_sublist_count(&self.template, inputs) else {
                return false;
            };
            let mech_core::SchemaBody::Matrix { dimensions, .. } = self.template.output().body()
            else {
                return false;
            };
            if (source.elements().len() as u64).checked_add(count)
                != call
                    .output()
                    .shape_instance
                    .resolve_dimension(&dimensions[1])
                    .ok()
            {
                return false;
            }
        }
        #[cfg(feature = "sequence_buffers")]
        if self.template.primitive()
            == crate::native_artifact::NativeCollectionPrimitive::SequenceAppendLeBytes
        {
            let Some(mech_core::ValueData::Matrix(source)) =
                inputs.first().and_then(Option::as_ref).map(Value::data)
            else {
                return false;
            };
            let Some(mech_core::ValueData::U8(count @ 0..=8)) =
                inputs.get(2).and_then(Option::as_ref).map(Value::data)
            else {
                return false;
            };
            let mech_core::SchemaBody::Matrix { dimensions, .. } = self.template.output().body()
            else {
                return false;
            };
            let expected = (source.elements().len() as u64).checked_add(u64::from(*count));
            if expected
                != call
                    .output()
                    .shape_instance
                    .resolve_dimension(&dimensions[1])
                    .ok()
            {
                return false;
            }
        }
        if self.template.primitive()
            == crate::native_artifact::NativeCollectionPrimitive::RecordFieldAccess
            && call
                .inputs()
                .get(1)
                .and_then(|input| input.resolved_selector)
                != record_selector(&self.template, inputs)
                    .map(mech_core::ResidentResolvedSelector::Id)
        {
            return false;
        }
        if inputs.len() != call.inputs().len()
            || !inputs
                .iter()
                .zip(call.inputs())
                .zip(self.template.inputs())
                .all(|((value, bound), schema)| {
                    let Some(value) = value.as_ref() else {
                        return false;
                    };
                    let actual = value.shape().parameter_values();
                    actual.len() == schema.dimension_parameters().len()
                        && actual == bound.shape_instance.parameter_values()
                })
        {
            return false;
        }
        if self.output_follows_input {
            // The SAME bind_values verified the input->output extent mapping
            // on the first bind. Identical input tapes and immutable schemas
            // therefore imply identical derived output geometry, even when the
            // old published output has a different tape or parameter numbering.
            return true;
        }
        let bound = call.output().shape_instance.parameter_values();
        if let Some(value) = output.as_ref() {
            let actual = value.shape().parameter_values();
            actual.len() == self.output_parameters && actual == bound
        } else {
            // bind_values uses the declared-length zero tape for absent old
            // output. Compare it without allocating a placeholder Vec.
            bound.len() == self.output_parameters && bound.iter().all(|value| *value == 0)
        }
    }

    /// Reuse only the exact current binding geometry at this site within one
    /// owning native invocation. Every hit measures fresh Values; a shape miss
    /// retires the old prepaid binding before constructing its replacement
    /// through the SAME live binder. No argument or result is retained.
    pub fn invoke_current_cached(
        &self,
        cache: &mut NativePreparedLibraryCallCache,
        inputs: &[Option<Value>],
        output: &mut Option<Value>,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
    ) -> Result<bool, InvokeError> {
        finite_limits(self.template.origin().node(), limits).map_err(InvokeError::Binding)?;
        if inputs.len() != self.template.inputs().len() {
            return Err(InvokeError::InvalidInput);
        }
        work.with_call_limits(
            limits.max_compute_work.unwrap(),
            limits.max_comparison_work.unwrap(),
            || {
                work.charge(self.lookup_work, 0)?;
                let memory = work.memory_budget();
                if let Some(binding) = cache.binding.as_ref() {
                    if binding.projection_count.is_some()
                        || !Arc::ptr_eq(&binding.identity, &self.cache_identity)
                        || binding.limits != limits
                        || !binding.work.shares_account(work)
                        || binding._reservation.budget() != memory
                    {
                        return Err(InvokeError::InvalidInput);
                    }
                }
                self.validate_values(inputs, output, &memory)?;
                let hit = cache
                    .binding
                    .as_ref()
                    .and_then(|binding| binding.call.as_ref())
                    .is_some_and(|call| self.geometry_matches(call, inputs, output));
                if !hit && cache.binding.is_none() {
                    work.charge(512, 0)?;
                    // Binding capacity is already subtracted from the nested
                    // temporary allowance below. Their complete coexistence
                    // bound is therefore the original finite allowance, plus
                    // the two concrete scope owners. Prove that headroom
                    // before retaining a new optional binding. No binder or
                    // kernel runs while these probe bytes are held.
                    let headroom = limits.max_temporary_bytes.unwrap().checked_add(
                        crate::resident::budget::payload::native_scope_metadata_bytes(),
                    );
                    let fits = match headroom {
                        Some(bytes) => match memory.reserve_capacity(bytes) {
                            Ok(probe) => {
                                drop(probe);
                                true
                            }
                            Err(mech_core::MemoryRuntimeError::BudgetExceeded { .. }) => false,
                            Err(error) => return Err(error.into()),
                        },
                        None => false,
                    };
                    if !fits {
                        work.charge(self.binding_work, 0)?;
                        return self.invoke_validated_current(inputs, output, limits, work);
                    }
                }
                if !hit {
                    let owner_bytes = core::mem::size_of::<CachedBinding>() as u64;
                    let bytes = self
                        .binding_bytes
                        .checked_add(owner_bytes)
                        .ok_or(InvokeError::InvalidInput)?;
                    let setup_work = if cache.binding.is_none() {
                        self.binding_work
                            .checked_add(
                                owner_bytes
                                    .checked_mul(8)
                                    .and_then(|n| n.checked_add(256))
                                    .ok_or(InvokeError::InvalidInput)?,
                            )
                            .ok_or(InvokeError::InvalidInput)?
                    } else {
                        self.binding_work
                    };
                    // Preserve the full original construction+retirement work
                    // envelope. Only an actual geometry hit avoids rebuilding.
                    work.charge(setup_work, 0)?;
                    live::check_measurement_with_limits(
                        CurrentMemoryFootprint::default(),
                        ResourceDemand {
                            turn_peak_bytes: bytes,
                            retained_nodes: self.binding_nodes,
                            ..ResourceDemand::default()
                        },
                        limits,
                    )?;
                    let mut nested = limits;
                    nested.max_temporary_bytes = Some(
                        limits
                            .max_temporary_bytes
                            .unwrap()
                            .checked_sub(bytes)
                            .ok_or(MemoryPlanError::TargetAddressOverflow)?,
                    );
                    nested.max_retained_nodes = Some(
                        limits
                            .max_retained_nodes
                            .unwrap()
                            .checked_sub(self.binding_nodes)
                            .ok_or(MemoryPlanError::TargetAddressOverflow)?,
                    );
                    if let Some(binding) = cache.binding.as_mut() {
                        // Lookup, identities, input validation and the complete
                        // replacement work/temporary checks already succeeded.
                        // Retire the old prepaid call BEFORE any new allocation.
                        // Keep a funded guard if replacement admission fails.
                        binding.plans = NativePlanCacheState::default();
                        drop(binding.call.take());
                        binding._nodes.resize(1)?;
                        binding._reservation.resize_capacity(owner_bytes)?;
                        let nodes = memory.reserve_nodes(self.binding_nodes)?;
                        binding._reservation.resize_capacity(bytes)?;
                        match self.bind_values(inputs, output, nested) {
                            Ok(call) => {
                                binding._nodes.merge(nodes)?;
                                binding.call = Some(call);
                            }
                            Err(error) => {
                                binding._reservation.resize_capacity(owner_bytes)?;
                                return Err(error);
                            }
                        }
                    } else {
                        let nodes = memory.reserve_nodes(
                            1u64.checked_add(self.binding_nodes)
                                .ok_or(InvokeError::InvalidInput)?,
                        )?;
                        let reservation = memory.reserve_capacity(bytes)?;
                        let call = self.bind_values(inputs, output, nested)?;
                        cache.binding = Some(Box::new(CachedBinding {
                            plans: NativePlanCacheState::default(),
                            call: Some(call),
                            identity: self.cache_identity.clone(),
                            limits,
                            work: work.clone(),
                            projection_count: None,
                            binding_nodes: self.binding_nodes,
                            projection_index: Default::default(),
                            _reservation: reservation,
                            _nodes: nodes,
                        }));
                    }
                }
                // The same maintained planner, current payload facts, bounded
                // kernel, admission and atomic publication run on EVERY visit.
                let binding = cache.binding.as_mut().unwrap();
                binding
                    .call
                    .as_ref()
                    .ok_or(InvokeError::InvalidInput)?
                    .invoke_cached(&mut binding.plans, inputs, output, work)
            },
        )?
    }

    /// Invoke the one selected leaf at the actual current input/prior-output
    /// geometry. The generated caller must separately prove its current visit.
    /// All binding metadata is charged/reserved BEFORE vectors, schema clones,
    /// plans or the kernel are built. The SAME enclosing call scope covers
    /// binding and execution, so the inner invoke cannot renew its allowance.
    pub fn invoke_current(
        &self,
        inputs: &[Option<Value>],
        output: &mut Option<Value>,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
    ) -> Result<bool, InvokeError> {
        finite_limits(self.template.origin().node(), limits).map_err(InvokeError::Binding)?;
        if inputs.len() != self.template.inputs().len() {
            return Err(InvokeError::InvalidInput);
        }
        work.with_call_limits(
            limits.max_compute_work.unwrap(),
            limits.max_comparison_work.unwrap(),
            || {
                work.charge(self.binding_work, 0)?;
                let memory = work.memory_budget();
                self.validate_values(inputs, output, &memory)?;
                self.invoke_validated_current(inputs, output, limits, work)
            },
        )?
    }
    // Both entry points already validated their exact operands and charged
    // binding work inside the original enclosing call scope. Sharing this
    // leaf path cannot renew that scope or change its publication boundary.
    fn invoke_validated_current(
        &self,
        inputs: &[Option<Value>],
        output: &mut Option<Value>,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
    ) -> Result<bool, InvokeError> {
        let memory = work.memory_budget();
        live::check_measurement_with_limits(
            CurrentMemoryFootprint::default(),
            ResourceDemand {
                turn_peak_bytes: self.binding_bytes,
                retained_nodes: self.binding_nodes,
                ..ResourceDemand::default()
            },
            limits,
        )?;
        let _binding = memory.reserve_capacity(self.binding_bytes)?;
        // This record remains live during invocation. Its reservation is
        // therefore subtracted from the nested temporary allowance; a
        // large invocation account cannot lift the original call policy.
        let mut nested = limits;
        nested.max_temporary_bytes = Some(
            limits
                .max_temporary_bytes
                .unwrap()
                .checked_sub(self.binding_bytes)
                .ok_or(MemoryPlanError::TargetAddressOverflow)?,
        );
        nested.max_retained_nodes = Some(
            limits
                .max_retained_nodes
                .unwrap()
                .checked_sub(self.binding_nodes)
                .ok_or(MemoryPlanError::TargetAddressOverflow)?,
        );
        let call = self.bind_values(inputs, output, nested)?;
        call.invoke(inputs, output, work)
    }
}

#[cfg(feature = "sequence_buffers")]
fn prepared_sublist_count(
    template: &NativeProgramLibraryTemplate,
    inputs: &[Option<Value>],
) -> Option<u64> {
    use crate::intrinsics::sequence_buffers as buffers;
    let source = inputs.get(1)?.as_ref()?;
    let width = buffers::width(buffers::output_element(
        buffers::Operation::AppendSublist,
        template.output().body(),
    )?)?;
    let start = buffers::scalar_word(inputs.get(2)?.as_ref()?.data()).ok()?;
    let count = buffers::scalar_word(inputs.get(3)?.as_ref()?.data()).ok()?;
    Some(
        buffers::sublist_range(source, width, start, count)
            .ok()?
            .len() as u64,
    )
}
