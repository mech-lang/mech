//! One current-funded loan for a pure root matrix-expression DAG. Native code
//! runs the existing kernels; this module only admits views and settles source
//! control/ownership after all private arithmetic has completed.
use super::numeric_binary::fits;
use super::{Error, Invocation, invalid};
use crate::native_artifact::program::{
    prepare::{
        numeric_binary::Kind,
        numeric_regions::{Candidate, Source, Step},
    },
    protocol::abi::NumericBinaryFrame as Frame,
};
use crate::native_artifact::{NativeExecutionPhase, values::NativeValueHandle};
use mech_core::{
    ManagedMemoryReservation, ManagedNodeReservation, ShapeInstance, Value, snapshot::F64Bits,
};
use mech_engine::native_artifact::{
    NativeIterationAction as Action, NativeMatchAction as Scalar, NativeScalarKind,
};

struct External {
    value: Option<Value>,
    handle: Option<NativeValueHandle>,
    word: u64,
    dimensions: [usize; 2],
}
struct Bank {
    dimensions: [usize; 2],
    shape: Option<ShapeInstance>,
    required: u64,
    escaped: bool,
}
pub(in crate::native_artifact::program::runtime) struct Region {
    pub(super) candidate: usize,
    pub(super) frames: Box<[Frame]>,
    issued: Box<[Frame]>,
    external: Vec<External>,
    banks: Vec<Bank>,
    outputs: Vec<Option<Vec<F64Bits>>>,
    _scratch: Vec<F64Bits>,
    handles: Vec<Option<NativeValueHandle>>,
    pub(super) reservation: ManagedMemoryReservation,
    _metadata: ManagedMemoryReservation,
    _metadata_nodes: ManagedNodeReservation,
    _nodes: ManagedNodeReservation,
    private_nodes: u64,
}
fn add(a: u64, b: u64) -> Option<u64> {
    a.checked_add(b)
}
fn zeros(count: usize) -> Option<Vec<F64Bits>> {
    let mut cells = Vec::new();
    cells.try_reserve_exact(count).ok()?;
    cells.resize(count, F64Bits::from_bits(0));
    Some(cells)
}
fn dims(value: &Value, schema: &mech_core::Schema) -> Option<[usize; 2]> {
    let mech_core::SchemaBody::Matrix {
        element,
        dimensions,
    } = schema.body()
    else {
        return None;
    };
    if element.as_ref() != &mech_core::SchemaBody::FloatingPoint(mech_core::FloatWidth::W64)
        || dimensions.len() != 2
        || value.shape().parameter_values().len() != schema.dimension_parameters().len()
    {
        return None;
    }
    let cells = super::numeric_binary::cells(value)?;
    let rows = usize::try_from(value.shape().resolve_dimension(&dimensions[0]).ok()?).ok()?;
    let cols = usize::try_from(value.shape().resolve_dimension(&dimensions[1]).ok()?).ok()?;
    (rows.checked_mul(cols)? == cells.len()).then_some([rows, cols])
}
fn source_dims(source: Source, external: &[External], banks: &[Bank]) -> [usize; 2] {
    match source {
        Source::External(at) => external[at].dimensions,
        Source::Bank(at) => banks[at].dimensions,
    }
}
fn source_pointer(
    source: Source,
    external: &[External],
    bank_slots: &[usize],
    pointers: &[u64],
) -> u64 {
    match source {
        Source::External(at) => external[at]
            .value
            .as_ref()
            .and_then(super::numeric_binary::cells)
            .map_or(0, |v| v.as_ptr() as u64),
        Source::Bank(at) => pointers[bank_slots[at]],
    }
}
fn scalar_word(source: Source, external: &[External]) -> u64 {
    match source {
        Source::External(at) => external[at]
            .value
            .as_ref()
            .and_then(super::numeric_binary::cells)
            .and_then(|v| v.first())
            .map_or(external[at].word, |v| v.to_f64().to_bits()),
        Source::Bank(_) => 0, // Loaded from the produced bank by the shared emitter.
    }
}
fn source_handle(
    candidate: &Candidate,
    handles: &[Option<NativeValueHandle>],
    external: &[External],
    actual: u32,
    source: Source,
) -> Option<NativeValueHandle> {
    for (at, step) in candidate.steps.iter().enumerate() {
        let output = match step {
            Step::Operator { output, .. } | Step::Alias { output, .. } => Some(output.get()),
            _ => None,
        };
        if output == Some(actual) {
            return handles[at];
        }
    }
    match source {
        Source::External(at) => external[at].handle,
        Source::Bank(_) => None,
    }
}

// Generated kernels check each produced result before a dependent operation.
// A nonfinite result cancels this private loan and replays original execution;
// no host payload scan or floating-point reassociation grants eligibility.
impl Invocation<'_, '_, '_> {
    pub(super) fn prepare_numeric_region(
        &mut self,
        site: usize,
        args: [u64; 3],
    ) -> Result<Option<u64>, Error> {
        let descriptor = self.descriptor;
        let m = descriptor.metadata();
        let (region_id, candidate) = m
            .numeric_regions
            .at_entry(site)
            .ok_or_else(|| invalid("matrix region outside source entry"))?;
        let Some((steps, _)) = candidate.body(&m.plan) else {
            return Ok(Some(0));
        };
        if self.state.control.previous != Some(site as u32)
            || self.state.consumed
            || self.numeric_binary.is_some()
            || self.numeric_region.is_some()
            || self.state.control.current.is_some()
        {
            return Err(invalid("matrix region outside fresh root visit"));
        }
        if self.worker.is_some()
            || self.state.scalar_path.is_some()
            || self.has_uninventoried_private_owners()
            || self.has_retained_maps()
            || self
                .state
                .collections
                .as_ref()
                .is_some_and(|c| !c.is_complete())
            || steps
                .iter()
                .any(|s| m.map_admission.group(s.site() as usize).is_some())
        {
            return Ok(Some(0));
        }
        let scalar_arity = candidate
            .external
            .iter()
            .filter(|v| {
                m.plan.values()[v.get() as usize].scalar_kind() == Some(NativeScalarKind::F64)
            })
            .count();
        if args != [self.arguments as u64, scalar_arity as u64, 1]
            || self.arguments.is_null()
            || self.argument_capacity < scalar_arity
        {
            return Err(invalid("matrix region source argument lane differs"));
        }
        let memory = self.state.library.memory_budget();
        let caps = self.limits;
        let Some(prior) = self.retained_caller_geometry(site)? else {
            return Ok(Some(0));
        };
        let n = candidate.storage_slots.len();
        let slots = candidate.slots;
        let arity = candidate.external.len();
        let shape_bytes = steps.iter().try_fold(0u64, |sum, step| match step {
            Step::Operator { output, .. } => sum.checked_add(
                (m.plan.values()[output.get() as usize]
                    .definition()
                    .dimension_parameters()
                    .len() as u64)
                    .checked_mul(32)?,
            ),
            _ => Some(sum),
        });
        let metadata_bytes = (core::mem::size_of::<Region>() as u64)
            .checked_add(
                (n as u64)
                    .checked_mul(
                        (core::mem::size_of::<Bank>()
                            + 2 * core::mem::size_of::<Frame>()
                            + core::mem::size_of::<mech_core::ResourceDemand>())
                            as u64,
                    )
                    .unwrap_or(u64::MAX),
            )
            .and_then(|v| {
                add(
                    v,
                    (arity as u64).checked_mul(core::mem::size_of::<External>() as u64)?,
                )
            })
            .and_then(|v| {
                add(
                    v,
                    (candidate.steps.len() as u64)
                        .checked_mul(core::mem::size_of::<Option<NativeValueHandle>>() as u64)?,
                )
            })
            .and_then(|v| add(v, (slots as u64).checked_mul(128)?))
            .and_then(|v| add(v, shape_bytes?))
            .and_then(|v| add(v, 4096));
        let metadata_nodes = (n as u64)
            .checked_mul(2)
            .and_then(|v| add(v, arity as u64))
            .and_then(|v| add(v, 16));
        let (Some(metadata_bytes), Some(metadata_nodes)) = (metadata_bytes, metadata_nodes) else {
            return Ok(Some(0));
        };
        if prior
            .live_bytes
            .checked_add(metadata_bytes)
            .is_none_or(|v| !fits(v, caps.max_temporary_bytes))
            || prior
                .live_nodes
                .checked_add(metadata_nodes)
                .is_none_or(|v| !fits(v, caps.max_retained_nodes))
        {
            return Ok(Some(0));
        }
        let Ok(metadata) = memory.reserve_capacity(metadata_bytes) else {
            return Ok(Some(0));
        };
        let Ok(meta_nodes) = memory.reserve_nodes(metadata_nodes) else {
            return Ok(Some(0));
        };
        // The generated caller owns this exact initialized argument lane;
        // no reference to it survives the suspended callback.
        let words = unsafe { core::slice::from_raw_parts(self.arguments, scalar_arity) };
        let mut external = Vec::with_capacity(arity);
        let mut scalar_at = 0;
        for value in &candidate.external {
            let id = value.get();
            let ty = &m.plan.values()[id as usize];
            if ty.scalar_kind() == Some(NativeScalarKind::F64) {
                external.push(External {
                    value: None,
                    handle: None,
                    word: words[scalar_at],
                    dimensions: [1, 1],
                });
                scalar_at += 1;
                continue;
            }
            let Some((value, handle)) = self
                .state
                .arena
                .copy_current_prepaid(id, &self.state.control.work)?
            else {
                return Ok(Some(0));
            };
            let slot = &m.types[id as usize];
            if value.schema() != slot.schema
                || value.schema_key() != slot.key
                || value.memory_budget_retained_bytes(&memory).is_none()
            {
                return Ok(Some(0));
            }
            let Some(dimensions) = dims(&value, ty.definition()) else {
                return Ok(Some(0));
            };
            external.push(External {
                value: Some(value),
                handle: Some(handle),
                word: 0,
                dimensions,
            });
        }
        let mut banks: Vec<Bank> = Vec::with_capacity(n);
        let mut frames = Vec::with_capacity(n);
        let mut demands = Vec::with_capacity(n);
        let mut temporary = 0u64;
        let mut nodes = 0u64;
        let mut compute = 0u64;
        let mut comparison = 0u64;
        let mut replay = 0u64;
        let mut settlement = 4096u64;
        let mut publication_nodes = 0u64;
        let mut maxima = vec![0usize; slots];
        let mut escaping = vec![false; slots];
        for step in steps {
            settlement = match settlement
                .checked_add(m.visits[step.site() as usize].work)
                .and_then(|v| v.checked_add(1024))
            {
                Some(v) => v,
                None => return Ok(Some(0)),
            };
            let Step::Operator {
                site: op_site,
                kind,
                inputs,
                output,
                bank,
            } = step
            else {
                continue;
            };
            if *bank != banks.len() {
                return Err(invalid("matrix region bank order differs"));
            }
            let call = m.calls[*op_site as usize]
                .as_ref()
                .ok_or_else(|| invalid("matrix region canonical call missing"))?;
            let definition = m.plan.values()[output.get() as usize].definition();
            if call.template().output() != definition {
                return Ok(Some(0));
            }
            let ld = source_dims(inputs[0], &external, &banks);
            let rd = inputs
                .get(1)
                .map_or([1, 1], |&s| source_dims(s, &external, &banks));
            let geometry = match *kind {
                Kind::Elementwise(op) => {
                    if call.template().f64_matrix_binary_op() != Some(op) {
                        return Err(invalid("matrix region arithmetic factory differs"));
                    }
                    call.f64_matrix_binary_geometry(ld, rd)
                        .map(|[r, c, lmode, rmode]| (r, c, lmode, rmode, 0))
                }
                Kind::Transpose { .. } => {
                    call.f64_matrix_transpose_geometry(ld)
                        .and_then(|[r, c, count]| {
                            (r.checked_mul(c) == Some(count)).then_some((r, c, 1, 1, ld[1] as u64))
                        })
                }
                Kind::Sum { axis, .. } => {
                    if call.template().f64_matrix_sum_axis() != Some(axis) {
                        return Err(invalid("matrix region reduction factory differs"));
                    }
                    call.f64_matrix_sum_geometry(ld)
                        .and_then(|[r, c, count, inner]| {
                            (r.checked_mul(c).and_then(|v| v.checked_mul(inner)) == Some(count))
                                .then_some((r, c, 1, 1, inner))
                        })
                }
                Kind::Product => {
                    if !call.template().f64_matrix_product() {
                        return Err(invalid("matrix region product factory differs"));
                    }
                    call.f64_matrix_product_geometry(ld, rd)
                        .map(|[r, c, k]| (r, c, 1, 1, k))
                }
            };
            let Some((rows, columns, lmode, rmode, inner)) = geometry else {
                return Ok(Some(0));
            };
            let Some(count) = rows.checked_mul(columns) else {
                return Ok(Some(0));
            };
            let input_count = inputs.iter().try_fold(0u64, |sum, &source| {
                let d = source_dims(source, &external, &banks);
                sum.checked_add((d[0] as u64).checked_mul(d[1] as u64)?)
            });
            let Some(input_count) = input_count else {
                return Ok(Some(0));
            };
            let source_bound = match *kind {
                Kind::Elementwise(_) => call.f64_matrix_binary_source_bound(input_count, count),
                Kind::Transpose { .. } => call.f64_matrix_transpose_source_bound(count),
                Kind::Sum { .. } => call.f64_matrix_sum_source_bound(input_count, count),
                Kind::Product => call.f64_matrix_product_source_bound(input_count, count, inner),
            };
            let Some((demand, credit)) = source_bound else {
                return Ok(Some(0));
            };
            if !fits(count, caps.max_output_elements)
                || !fits(demand.persistent_bytes, caps.max_output_bytes)
                || !fits(demand.turn_peak_bytes, caps.max_temporary_bytes)
                || !fits(demand.cloned_bytes, caps.max_cloned_bytes)
                || !fits(demand.retained_nodes, caps.max_retained_nodes)
                || !fits(demand.work.compute, caps.max_compute_work)
                || !fits(demand.work.comparison, caps.max_comparison_work)
                || !fits(0, caps.max_scalar_instructions)
                || !fits(0, caps.max_transfer_bytes)
                || !fits(0, caps.max_storage_buffer_bytes)
                || caps.max_storage_bindings.is_none()
            {
                return Ok(Some(0));
            }
            let Ok(shape) = mech_core::shape_for_resolved_extents(definition, &[rows, columns])
            else {
                return Ok(Some(0));
            };
            let Ok((required, canonical_nodes)) = Value::prospective_f64_matrix_ownership(
                m.types[output.get() as usize].schema,
                &shape,
                count,
                &m.schemas,
            ) else {
                return Ok(Some(0));
            };
            let Ok(elements) = usize::try_from(count) else {
                return Ok(Some(0));
            };
            if core::alloc::Layout::array::<F64Bits>(elements).is_err() {
                return Ok(Some(0));
            }
            let slot = candidate.storage_slots[*bank];
            maxima[slot] = maxima[slot].max(elements);
            let escaped = !candidate.private_banks.contains(bank);
            if escaped {
                let Some(next) = publication_nodes.checked_add(canonical_nodes) else {
                    return Ok(Some(0));
                };
                publication_nodes = next;
            }
            escaping[slot] |= escaped;
            let Some(next) = temporary.checked_add(demand.turn_peak_bytes) else {
                return Ok(Some(0));
            };
            temporary = next;
            let Some(next) = nodes.checked_add(demand.retained_nodes) else {
                return Ok(Some(0));
            };
            nodes = next;
            let Some(next) = compute.checked_add(demand.work.compute) else {
                return Ok(Some(0));
            };
            compute = next;
            let Some(next) = comparison.checked_add(demand.work.comparison) else {
                return Ok(Some(0));
            };
            comparison = next;
            let Some(next) = replay.checked_add(credit) else {
                return Ok(Some(0));
            };
            replay = next;
            let Some(next) = required
                .checked_mul(4)
                .and_then(|v| v.checked_add(64))
                .and_then(|v| settlement.checked_add(v))
            else {
                return Ok(Some(0));
            };
            settlement = next;
            if required
                .checked_mul(4)
                .and_then(|v| v.checked_add(64))
                .is_none_or(|v| v < 96)
            {
                return Ok(Some(0));
            }
            let (Ok(r), Ok(c)) = (usize::try_from(rows), usize::try_from(columns)) else {
                return Ok(Some(0));
            };
            banks.push(Bank {
                dimensions: [r, c],
                shape: escaped.then_some(shape),
                required,
                escaped,
            });
            frames.push(Frame {
                left: 0,
                right: 0,
                output: 0,
                count,
                columns,
                inner,
                left_mode: lmode,
                right_mode: rmode,
                left_scalar: 0,
                right_scalar: 0,
            });
            demands.push(demand);
        }
        if banks.len() != n {
            return Err(invalid("matrix region source bank inventory differs"));
        }
        if prior
            .live_bytes
            .checked_add(temporary)
            .is_none_or(|v| !fits(v, caps.max_temporary_bytes))
            || prior
                .live_nodes
                .checked_add(nodes)
                .is_none_or(|v| !fits(v, caps.max_retained_nodes))
        {
            return Ok(Some(0));
        }
        let credit = compute
            .checked_add(replay)
            .and_then(|v| v.checked_add(settlement));
        let compare_credit = comparison.checked_mul(2);
        let (Some(credit), Some(compare_credit)) = (credit, compare_credit) else {
            return Ok(Some(0));
        };
        if self
            .state
            .control
            .work
            .limit()
            .saturating_sub(self.state.control.work.used())
            < credit
            || self
                .state
                .library
                .unscoped_remaining()
                .is_none_or(|(w, c)| w < credit || c < compare_credit)
        {
            return Ok(Some(0));
        }
        let (Some(data_bytes), Some(remaining_nodes)) = (
            temporary.checked_sub(metadata_bytes),
            nodes.checked_sub(metadata_nodes),
        ) else {
            return Ok(Some(0));
        };
        let Ok(reservation) = memory.reserve_capacity(data_bytes) else {
            return Ok(Some(0));
        };
        let Ok(probe) = memory.reserve_nodes(remaining_nodes) else {
            return Ok(Some(0));
        };
        drop(probe);
        let data_nodes = (slots as u64).checked_mul(2).and_then(|v| v.checked_add(8));
        let Some(data_nodes) = data_nodes.filter(|&v| {
            v.checked_add(publication_nodes)
                .is_some_and(|total| total <= remaining_nodes)
        }) else {
            return Ok(Some(0));
        };
        let Ok(data_nodes_claim) = memory.reserve_nodes(data_nodes) else {
            return Ok(Some(0));
        };
        // Full old-source envelopes pay discovery, all arithmetic, source
        // canonical construction/publication/retirement and ordinary replay.
        for (step, demand) in steps
            .iter()
            .filter(|s| matches!(s, Step::Operator { .. }))
            .zip(&demands)
        {
            self.state.control.work.charge(
                NativeExecutionPhase::Node(m.plan.blocks()[step.site() as usize].origin().node()),
                demand.work.compute,
            )?;
            self.state
                .library
                .charge(0, demand.work.comparison)
                .map_err(|error| Error::Collection {
                    origin: m.plan.blocks()[step.site() as usize].origin(),
                    error,
                })?;
        }
        for capture in &external {
            // Full original source funding precedes recursive canonical
            // validation. Kernel result checks replace all magnitude scans.
            if let Some(value) = &capture.value {
                if value.validate_against(&m.schemas).is_err() {
                    return Ok(Some(0));
                }
            }
        }
        let mut scratch_count = 0usize;
        let mut offsets = vec![0usize; slots];
        for slot in 0..slots {
            if !escaping[slot] {
                offsets[slot] = scratch_count;
                let Some(next) = scratch_count.checked_add(maxima[slot]) else {
                    return Ok(Some(0));
                };
                scratch_count = next;
            }
        }
        if core::alloc::Layout::array::<F64Bits>(scratch_count).is_err() {
            return Ok(Some(0));
        }
        let Some(mut scratch) = zeros(scratch_count) else {
            return Ok(Some(0));
        };
        let mut outputs = Vec::with_capacity(slots);
        let mut pointers = Vec::with_capacity(slots);
        for slot in 0..slots {
            if escaping[slot] {
                let Some(mut cells) = zeros(maxima[slot]) else {
                    return Ok(Some(0));
                };
                pointers.push(cells.as_mut_ptr() as u64);
                outputs.push(Some(cells));
            } else {
                let pointer = unsafe { scratch.as_mut_ptr().add(offsets[slot]) };
                pointers.push(pointer as u64);
                outputs.push(None);
            }
        }
        for step in steps {
            let Step::Operator { inputs, bank, .. } = step else {
                continue;
            };
            let frame = &mut frames[*bank];
            frame.left = source_pointer(inputs[0], &external, &candidate.storage_slots, &pointers);
            frame.left_scalar = scalar_word(inputs[0], &external);
            if let Some(&right) = inputs.get(1) {
                frame.right = source_pointer(right, &external, &candidate.storage_slots, &pointers);
                frame.right_scalar = scalar_word(right, &external);
            }
            frame.output = pointers[candidate.storage_slots[*bank]];
        }
        let frames = frames.into_boxed_slice();
        let issued = frames.clone();
        let pointer = frames.as_ptr() as u64;
        let private_nodes = metadata_nodes
            .checked_add(data_nodes)
            .ok_or_else(|| invalid("matrix region private nodes overflow"))?;
        let region = Region {
            candidate: region_id,
            frames,
            issued,
            external,
            banks,
            outputs,
            _scratch: scratch,
            handles: vec![None; candidate.steps.len()],
            reservation,
            _metadata: metadata,
            _metadata_nodes: meta_nodes,
            _nodes: data_nodes_claim,
            private_nodes,
        };
        self.numeric_region = Some(Box::new(region));
        Ok(Some(pointer))
    }

    pub(super) fn finish_numeric_region(
        &mut self,
        site: usize,
        args: [u64; 3],
        mut region: Box<Region>,
    ) -> Result<Option<u64>, Error> {
        let descriptor = self.descriptor;
        let m = descriptor.metadata();
        let candidate = m
            .numeric_regions
            .candidates
            .get(region.candidate)
            .ok_or_else(|| invalid("matrix region loan recipe absent"))?;
        let (steps, _) = candidate
            .body(&m.plan)
            .ok_or_else(|| invalid("matrix region loan source closure changed"))?;
        if site != candidate.entry as usize
            || self.state.control.previous != Some(candidate.entry)
            || self.state.consumed
            || self.state.control.current.is_some()
            || self.numeric_binary.is_some()
            || !matches!(args[2], 1 | 2)
            || args[..2] != [region.frames.as_ptr() as u64, region.frames.len() as u64]
            || region.reservation.budget() != self.state.library.memory_budget()
            || region.frames != region.issued
        {
            return Err(invalid("matrix region finish differs from issued loan"));
        }
        if args[2] == 2 {
            // The generated caller abandoned a nonfinite private result.
            // No logical producer, visit or output has been settled yet.
            // Dropping the same issued loan releases every view and claim;
            // the original source entry remains fresh for ordinary replay.
            return Ok(Some(0));
        }
        for (index, step) in steps.iter().enumerate() {
            let source_site = step.site() as usize;
            if index != 0 {
                self.visit(source_site as u64)?;
            }
            match step {
                Step::Operator {
                    inputs,
                    output,
                    bank,
                    ..
                } => {
                    if index != 0 {
                        let geometry = self
                            .retained_caller_geometry(source_site)?
                            .ok_or_else(|| invalid("matrix region caller inventory unavailable"))?;
                        geometry
                            .live_nodes
                            .checked_add(region.private_nodes)
                            .ok_or_else(|| {
                                invalid("matrix region complete caller nodes overflow")
                            })?;
                    }
                    let Action::LibraryCall { operands, .. } =
                        m.plan.blocks()[source_site].action()
                    else {
                        return Err(invalid("matrix region source operator changed"));
                    };
                    for (&operand, &source) in operands.iter().zip(inputs) {
                        if m.plan.values()[operand.get() as usize]
                            .scalar_kind()
                            .is_some()
                        {
                            continue;
                        }
                        let handle = source_handle(
                            candidate,
                            &region.handles,
                            &region.external,
                            operand.get(),
                            source,
                        )
                        .ok_or_else(|| invalid("matrix region source handle missing"))?;
                        if matches!(source,Source::Bank(at) if !region.banks[at].escaped) {
                            self.state
                                .arena
                                .check_virtual(handle, &mut self.state.control.work)?;
                        } else {
                            self.state.arena.get(handle, &mut self.state.control.work)?;
                        }
                        self.state
                            .control
                            .work
                            .charge(NativeExecutionPhase::Setup, 16)?;
                    }
                    let ty = &m.types[output.get() as usize];
                    let handle = if region.banks[*bank].escaped {
                        let slot = candidate.storage_slots[*bank];
                        let mut cells = region.outputs[slot].take().ok_or_else(|| {
                            invalid("matrix region output bank was already published")
                        })?;
                        let count = region.banks[*bank].dimensions[0]
                            .checked_mul(region.banks[*bank].dimensions[1])
                            .ok_or_else(|| invalid("matrix region output count overflow"))?;
                        cells.truncate(count);
                        let shape = region.banks[*bank]
                            .shape
                            .take()
                            .ok_or_else(|| invalid("matrix region output shape missing"))?;
                        let value = Value::from_f64_matrix(
                            ty.schema,
                            shape,
                            cells.into_boxed_slice(),
                            std::sync::Arc::clone(&m.schemas),
                        )
                        .map_err(Error::Snapshot)?
                        .into_memory_budget(&mut region.reservation)?;
                        if value.schema_key() != ty.key
                            || value
                                .memory_budget_retained_bytes(&self.state.library.memory_budget())
                                != Some(region.banks[*bank].required)
                        {
                            return Err(invalid(
                                "matrix region canonical ownership differs from admitted price",
                            ));
                        }
                        self.state
                            .arena
                            .put(output.get(), &value, &mut self.state.control.work)?
                    } else {
                        let bill = region.banks[*bank]
                            .required
                            .checked_mul(4)
                            .and_then(|v| v.checked_add(64))
                            .and_then(|v| v.checked_sub(96))
                            .ok_or_else(|| {
                                invalid("matrix region source publication bill overflow")
                            })?;
                        self.state
                            .control
                            .work
                            .charge(NativeExecutionPhase::Setup, bill)?;
                        self.state.arena.define_virtual(
                            output.get(),
                            ty.schema,
                            ty.key,
                            &mut self.state.control.work,
                        )?
                    };
                    region.handles[index] = Some(handle);
                }
                Step::Alias { source, output, .. } => {
                    if m.plan.values()[output.get() as usize]
                        .scalar_kind()
                        .is_none()
                    {
                        let Action::Scalar(Scalar::Copy { source: from, .. }) =
                            m.plan.blocks()[source_site].action()
                        else {
                            return Err(invalid("matrix region source alias changed"));
                        };
                        let handle = if matches!(*source,Source::Bank(at) if !region.banks[at].escaped)
                        {
                            let original = source_handle(
                                candidate,
                                &region.handles,
                                &region.external,
                                from.get(),
                                *source,
                            )
                            .ok_or_else(|| invalid("matrix region alias handle missing"))?;
                            self.state
                                .arena
                                .check_virtual_prepaid(original, &self.state.control.work)?;
                            let ty = &m.types[output.get() as usize];
                            self.state.arena.define_virtual(
                                output.get(),
                                ty.schema,
                                ty.key,
                                &mut self.state.control.work,
                            )?
                        } else {
                            self.state.arena.copy_current_to(
                                from.get(),
                                output.get(),
                                &mut self.state.control.work,
                            )?
                        };
                        region.handles[index] = Some(handle);
                    }
                }
                Step::Empty { .. } => {}
            }
            self.state.consumed = true;
        }
        Ok(Some(1))
    }
}
