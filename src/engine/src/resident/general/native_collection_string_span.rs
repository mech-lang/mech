//! Constructor-private storage for the already selected range/String leaf.
//! No producer evaluation, fake Matrix, scheduling, or alternate String index.
use super::{
    CachedBinding, InvokeError, NativePlanCacheState, NativePreparedLibraryCall,
    NativePreparedLibraryCallCache, NativeProgramLibraryTemplate, bind_current, finite_limits,
};
use crate::resident::budget::native_turn_plan_cache::NativeProjectionIndexContext;
use crate::resident::{budget::NativeLibraryWorkBudget, text};
use mech_core::{
    DimensionExpr, IntegerWidth, ManagedMemoryBudget, MemoryBudgetLimits, ResidentKernelError,
    ResourceDemand, SchemaBody, SchemaKey, Value,
};
use std::sync::Arc;

#[cfg(test)]
#[path = "native_collection_string_span_geometry_tests.rs"]
mod geometry_tests;

/// Non-Value metadata produced ONLY by a maintained selected U64 range.
/// It owns no selector payload, schema wrapper, borrowed text, or permission
/// to enter a native action. The generated current-visit proof is separate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeSelectedStringSpanSelector {
    start: u64,
    count: u64,
    schema_key: SchemaKey,
}
impl NativeSelectedStringSpanSelector {
    pub const fn start(&self) -> u64 {
        self.start
    }
    pub const fn count(&self) -> u64 {
        self.count
    }
    #[cfg(feature = "pure_text")]
    pub(super) const fn schema_key(&self) -> SchemaKey {
        self.schema_key
    }
}

fn dimension_geometry(expression: &DimensionExpr) -> Option<(u64, u64)> {
    match expression {
        DimensionExpr::Hole | DimensionExpr::Constant(_) | DimensionExpr::Parameter(_) => {
            Some((1, 1))
        }
        DimensionExpr::Add(children)
        | DimensionExpr::Multiply(children)
        | DimensionExpr::Min(children)
        | DimensionExpr::Max(children) => {
            let (nodes, depth) =
                children
                    .iter()
                    .try_fold((1u64, 0u64), |(nodes, depth), child| {
                        let (next_nodes, next_depth) = dimension_geometry(child)?;
                        Some((nodes.checked_add(next_nodes)?, depth.max(next_depth)))
                    })?;
            Some((nodes, depth.checked_add(1)?))
        }
    }
}

// The current maintained shape solver allocates two/three P-word tapes, a
// P-byte fixed bitmap and two rank-two witness pairs. Error messages contain
// fixed text and u64 ordinals, not payload/schema Debug dumps. Its witness
// inversion has at most TWO independent axes, each with at most u64::BITS
// monotone-search evaluations, unless the exact witness grammar below proves
// every assignment uses the SAME core solver's direct-Parameter branch.
// Recursive verification is bounded by measured
// expression depth; all bounds are computed once from the actual immutable
// schema, never from a workload name or observed successful allocation.
pub(super) fn selector_geometry_bounds(
    template: &NativeProgramLibraryTemplate,
) -> Option<(u64, u64)> {
    use crate::native_artifact::NativeCollectionPrimitive as Primitive;
    if !matches!(
        template.primitive(),
        Primitive::RangeInclusive | Primitive::RangeExclusive
    ) {
        return Some((0, 0));
    }
    selector_schema_geometry_bounds(template.output())
}

// This is a work-bound classification, not another shape evaluator. The
// maintained assign_unique_dimension_witness directly assigns a bare matching
// Parameter. With at most one parameter, the unique-witness loop has at most
// two rounds. Constants never enter it. After that shared optimization these
// exact witnesses cannot execute any binary-search evaluation. All declared
// lower/upper-bound trees, shape validation and fixed overhead stay funded.
fn direct_selector_witnesses(body: &SchemaBody, parameters: usize) -> bool {
    matches!(body, SchemaBody::Matrix { element, dimensions }
    if element.as_ref() == &SchemaBody::UnsignedInteger(IntegerWidth::W64)
    && parameters <= 1
    && dimensions.len() == 2
    && dimensions[0] == DimensionExpr::Constant(1)
    && dimensions.iter().all(|expression| match expression {
        DimensionExpr::Constant(_) => true,
        DimensionExpr::Parameter(parameter) => (parameter.get() as usize) < parameters,
        _ => false,
    }))
}

pub(super) fn selector_schema_geometry_bounds(schema: &mech_core::Schema) -> Option<(u64, u64)> {
    let SchemaBody::Matrix { dimensions, .. } = schema.body() else {
        return Some((0, 0));
    };
    let parameters = u64::try_from(schema.dimension_parameters().len()).ok()?;
    let mut nodes = 0u64;
    let mut depth = 0u64;
    for expression in dimensions
        .iter()
        .chain(schema.dimension_parameters().iter().flat_map(|parameter| {
            core::iter::once(parameter.lower_bound()).chain(parameter.upper_bound())
        }))
    {
        let (next_nodes, next_depth) = dimension_geometry(expression)?;
        nodes = nodes.checked_add(next_nodes)?;
        depth = depth.max(next_depth);
    }
    let metadata = parameters
        .checked_mul(3 * core::mem::size_of::<u64>() as u64 + 1)?
        .checked_add(2 * core::mem::size_of::<(&DimensionExpr, u64)>() as u64)?
        .checked_add(3 * core::mem::size_of::<mech_core::ShapeInstance>() as u64)?
        .checked_add(3 * core::mem::size_of::<Vec<u64>>() as u64)?
        .checked_add(512)?;
    let traversed = nodes
        .checked_mul(core::mem::size_of::<DimensionExpr>() as u64)?
        .checked_add(parameters.checked_mul(core::mem::size_of::<u64>() as u64)?)?
        .checked_add(core::mem::size_of::<mech_core::Schema>() as u64)?;
    let search_passes =
        if direct_selector_witnesses(schema.body(), schema.dimension_parameters().len()) {
            0
        } else {
            2 * u64::BITS as u64
        };
    let passes = depth
        .checked_add(search_passes)?
        .checked_add(parameters.checked_mul(2)?)?
        .checked_add(16)?;
    let work = traversed
        .checked_mul(passes)?
        .checked_mul(8)?
        .checked_add(256)?;
    Some((metadata, work))
}

impl NativePreparedLibraryCallCache {
    /// Exact admitted selected-projection metadata, not semantic output or a
    /// source-owner credit. Other native sites include these in coexistence.
    pub fn retained_projection_bytes(&self) -> u64 {
        self.binding
            .as_ref()
            .filter(|binding| binding.projection_count.is_some())
            .map_or(0, |binding| {
                binding
                    ._reservation
                    .capacity_bytes()
                    .checked_add(binding.projection_index.bytes())
                    .unwrap_or(u64::MAX)
            })
    }
    pub fn retained_projection_nodes(&self) -> u64 {
        self.binding
            .as_ref()
            .filter(|binding| binding.projection_count.is_some())
            .map_or(0, |binding| {
                1u64.checked_add(binding.projection_index.nodes())
                    .unwrap_or(u64::MAX)
            })
    }
}

impl NativePreparedLibraryCall {
    pub const fn selected_u64_string_span_selector_metadata_bytes(&self) -> u64 {
        self.span_selector_metadata_bytes
    }
    pub const fn selected_u64_string_span_selector_work(&self) -> u64 {
        self.span_selector_work
    }

    /// The caller pre-admits ONLY the concrete shape/witness scratch above
    /// and prepays its exact-schema traversal bound on the same work account.
    /// No full binding, canonical selector or payload is constructed here.
    pub fn selected_u64_string_span_selector(
        &self,
        start: u64,
        end: u64,
    ) -> Result<NativeSelectedStringSpanSelector, InvokeError> {
        self.selected_u64_string_span_selector_reusing_shape(start, end, None)
    }

    /// Reuse only a successful shape/physical-layout proof for the SAME schema
    /// and cardinality. Endpoint arithmetic is always evaluated afresh. The
    /// constructor-private selector contains no payload, work credit, source
    /// owner or visit permission. A different count/schema uses the maintained
    /// resolver. Callers still reserve scratch and pay the full selector bound.
    pub fn selected_u64_string_span_selector_reusing_shape(
        &self,
        start: u64,
        end: u64,
        previous: Option<&NativeSelectedStringSpanSelector>,
    ) -> Result<NativeSelectedStringSpanSelector, InvokeError> {
        let scalar = SchemaBody::UnsignedInteger(IntegerWidth::W64);
        let SchemaBody::Matrix {
            element,
            dimensions,
        } = self.template.output().body()
        else {
            return Err(InvokeError::InvalidInput);
        };
        if self.span_selector_metadata_bytes == 0
            || element.as_ref() != &scalar
            || dimensions.len() != 2
        {
            return Err(InvokeError::InvalidInput);
        }
        // Preserve the ordinary Range's rejection of empty/reversed/overflowing
        // unit ranges before considering the earlier successful shape proof.
        let count = self.selected_u64_range_count(start, end)?;
        if !previous.is_some_and(|old| old.schema_key == self.output_key && old.count == count) {
            self.validate_selected_u64_range_shape(count)?;
        }
        Ok(NativeSelectedStringSpanSelector {
            start,
            count,
            schema_key: self.output_key,
        })
    }

    fn validate_string_span_source(
        &self,
        source: &Value,
        memory: &ManagedMemoryBudget,
    ) -> Result<u64, InvokeError> {
        let mech_core::ResolvedOperationContract::Declared(contract) = self.template.contract()
        else {
            return Err(InvokeError::InvalidInput);
        };
        if source.schema() != contract.inputs[0].schema
            || source.schema_key() != self.input_keys[0]
            || !source.shape().parameter_values().is_empty()
            || !matches!(source.data(), mech_core::ValueData::String(_))
        {
            return Err(InvokeError::InvalidInput);
        }
        // This complete wrapper/schema-owner test is intentionally NOT
        // shares_frozen_storage: equal data Arcs can have different imports.
        source
            .memory_budget_retained_bytes(memory)
            .ok_or(InvokeError::UnadmittedValue)
    }

    /// Project one real admitted String at the ORIGINAL selected StringRange
    /// visit. `coexistence` contains the actual pre-projection State and OTHER
    /// sites' projection metadata, excluding this cache. We conservatively add
    /// the complete current source owner (not span bytes) and this site's real
    /// binding/index residency. No second global source reservation is taken.
    pub fn selected_u64_string_span(
        &self,
        cache: &mut NativePreparedLibraryCallCache,
        source: &Value,
        selector: &NativeSelectedStringSpanSelector,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        mut coexistence: ResourceDemand,
    ) -> Result<core::ops::Range<usize>, InvokeError> {
        use crate::native_artifact::NativeCollectionPrimitive as Primitive;
        finite_limits(self.template.origin().node(), limits).map_err(InvokeError::Binding)?;
        if self.template.primitive() != Primitive::StringRangeAccess
            || self.template.inputs().len() != 2
            || self.template.inputs()[0].body() != &SchemaBody::String
            || !self.template.inputs()[0].dimension_parameters().is_empty()
            || self.template.output().body() != &SchemaBody::String
            || !self.template.output().dimension_parameters().is_empty()
            || selector.schema_key != self.input_keys[1]
        {
            return Err(InvokeError::InvalidInput);
        }
        work.with_call_limits(
            limits.max_compute_work.unwrap(),
            limits.max_comparison_work.unwrap(),
            || {
                let memory = work.memory_budget();
                // Both calls inspect constant-size immutable headers only. The
                // lookup debit below prepays checks/error paths and refcount work.
                let owner_bytes = source.memory_budget_retained_bytes(&memory).unwrap_or(0);
                coexistence.turn_peak_bytes = coexistence
                    .turn_peak_bytes
                    .checked_add(owner_bytes)
                    .ok_or(ResidentKernelError::InvalidShape)?;
                coexistence.retained_nodes = coexistence
                    .retained_nodes
                    .checked_add(2)
                    .ok_or(ResidentKernelError::InvalidShape)?;
                let context = NativeProjectionIndexContext::new(work, limits, coexistence);
                context.check_capacity(
                    cache.retained_projection_bytes(),
                    cache.retained_projection_nodes(),
                )?;
                context.charge(self.lookup_work, 0)?;
                self.validate_string_span_source(source, &memory)?;
                if let Some(binding) = cache.binding.as_ref() {
                    if binding.projection_count.is_none()
                        || !Arc::ptr_eq(&binding.identity, &self.cache_identity)
                        || binding.limits != limits
                        || !binding.work.shares_account(work)
                        || binding._reservation.budget() != memory
                    {
                        return Err(InvokeError::InvalidInput);
                    }
                }
                let hit = cache.binding.as_ref().is_some_and(|binding| {
                    binding.projection_count == Some(selector.count) && binding.call.is_some()
                });
                if !hit {
                    let bytes = self
                        .binding_bytes
                        .checked_add(core::mem::size_of::<CachedBinding>() as u64)
                        .ok_or(ResidentKernelError::InvalidShape)?;
                    // Only a genuine geometry miss constructs the full selected
                    // binding and pays its unchanged metadata traversal/retirement.
                    let setup_work = self
                        .binding_work
                        .checked_add(
                            (core::mem::size_of::<CachedBinding>() as u64)
                                .checked_mul(8)
                                .and_then(|n| n.checked_add(256))
                                .ok_or(ResidentKernelError::InvalidShape)?,
                        )
                        .ok_or(ResidentKernelError::InvalidShape)?;
                    context.charge(setup_work, 0)?;
                    // The old cache owns no source, output, or active loan. Drop
                    // every old index/call BEFORE taking replacement capacity.
                    drop(cache.binding.take());
                    context.check_capacity(bytes, 1)?;
                    let nodes = memory.reserve_nodes(
                        1u64.checked_add(self.binding_nodes)
                            .ok_or(ResidentKernelError::InvalidShape)?,
                    )?;
                    let reservation = memory.reserve_capacity(bytes)?;
                    let shape = mech_core::shape_for_resolved_extents(
                        self.template.inputs().get(1).unwrap(),
                        &[1, selector.count],
                    )
                    .map_err(|_| InvokeError::Kernel(ResidentKernelError::InvalidShape))?;
                    // Bind the ACTUAL selected maintained factory using exact
                    // current shape metadata; there is no selector Value, input
                    // footprint witness, fabricated turn plan, or kernel invoke.
                    let call = bind_current(
                        &self.schemas,
                        self.template.clone(),
                        self.factory,
                        self.descriptor_templates.as_deref(),
                        &[source.shape().parameter_values(), shape.parameter_values()],
                        &[],
                        None,
                        limits,
                    )
                    .map_err(InvokeError::Binding)?;
                    text::validate_selected_u64_string_span_geometry(
                        &call.kernel,
                        &shape,
                        selector.count,
                    )?;
                    cache.binding = Some(Box::new(CachedBinding {
                        plans: NativePlanCacheState::default(),
                        call: Some(call),
                        identity: self.cache_identity.clone(),
                        limits,
                        work: work.clone(),
                        projection_count: Some(selector.count),
                        binding_nodes: self.binding_nodes,
                        projection_index: Default::default(),
                        _reservation: reservation,
                        _nodes: nodes,
                    }));
                }
                let binding = cache.binding.as_ref().ok_or(InvokeError::InvalidInput)?;
                let context =
                    context.with_added_capacity(binding._reservation.capacity_bytes(), 1)?;
                let index = context
                    .prepare_index(&binding.projection_index, source)
                    .map_err(|error| {
                        context
                            .take_memory_error()
                            .map(InvokeError::Memory)
                            .unwrap_or(InvokeError::Kernel(error))
                    })?;
                let call = binding.call.as_ref().ok_or(InvokeError::InvalidInput)?;
                text::selected_u64_string_span(
                    &call.kernel,
                    selector.start,
                    selector.count,
                    &index,
                    |compute, comparison| context.charge(compute, comparison),
                )
                .map_err(InvokeError::Kernel)
            },
        )?
    }
}
