//! Exact-site loans for one generated F64 matrix operation.
//! Inputs stay pinned; one initialized private result bank publishes atomically.
use super::*;
use crate::native_artifact::program::prepare::numeric_binary::Kind;
use crate::native_artifact::program::protocol::{GlobalHelper, abi::NumericBinaryFrame};
use mech_core::{
    DimensionExpr, FloatWidth, ManagedMemoryReservation, ManagedNodeReservation, SchemaBody,
    ShapeInstance,
    snapshot::{F64Bits, SequenceView},
};

pub(in crate::native_artifact::program::runtime) enum Pending {
    Single(Single),
    Region(super::numeric_regions::Region),
}
pub(in crate::native_artifact::program::runtime) struct Single {
    pub frame: NumericBinaryFrame,
    issued: NumericBinaryFrame,
    site: usize,
    output: u32,
    inputs: [Option<Value>; 2],
    cells: Vec<F64Bits>,
    shape: ShapeInstance,
    reservation: ManagedMemoryReservation,
    _nodes: ManagedNodeReservation,
}
pub(super) fn fits(value: u64, cap: Option<u64>) -> bool {
    cap.is_some_and(|cap| value <= cap)
}
fn dims(value: &Value, schema: &mech_core::Schema) -> Option<[usize; 2]> {
    let (
        SchemaBody::Matrix {
            element,
            dimensions,
        },
        mech_core::ValueData::Matrix(matrix),
    ) = (schema.body(), value.data())
    else {
        return None;
    };
    if element.as_ref() != &SchemaBody::FloatingPoint(FloatWidth::W64)
        || value.shape().parameter_values().len() != schema.dimension_parameters().len()
    {
        return None;
    }
    let [row, column] = dimensions.as_ref() else {
        return None;
    };
    let axis = |d: &DimensionExpr| match d {
        DimensionExpr::Constant(_) | DimensionExpr::Parameter(_) => {
            usize::try_from(value.shape().resolve_dimension(d).ok()?).ok()
        }
        _ => None,
    };
    let rows = axis(row)?;
    let columns = axis(column)?;
    let SequenceView::F64(cells) = matrix.elements() else {
        return None;
    };
    (rows.checked_mul(columns)? == cells.len()).then_some([rows, columns])
}
pub(super) fn cells(value: &Value) -> Option<&[F64Bits]> {
    let mech_core::ValueData::Matrix(matrix) = value.data() else {
        return None;
    };
    let SequenceView::F64(cells) = matrix.elements() else {
        return None;
    };
    Some(cells)
}
impl Invocation<'_, '_, '_> {
    pub(super) fn numeric_binary_helper(
        &mut self,
        site: usize,
        tag: GlobalHelper,
        args: [u64; 3],
    ) -> Result<Option<u64>, Error> {
        if tag == GlobalHelper::NumericBinaryFinish {
            return self.finish_numeric_binary(site, args);
        }
        if args[2] == 1 {
            return self.prepare_numeric_region(site, args);
        }
        let descriptor = self.descriptor;
        let m = descriptor.metadata();
        let op = m
            .numeric_binary
            .at(site)
            .ok_or_else(|| invalid("matrix arithmetic outside source selection"))?;
        if tag != GlobalHelper::NumericBinaryPrepare
            || self.state.consumed
            || self.numeric_binary.is_some()
            || self.state.control.previous != Some(site as u32)
        {
            return Err(invalid("matrix arithmetic outside fresh exact visit"));
        }
        if self.worker.is_some()
            || self.state.scalar_path.is_some()
            || self.has_uninventoried_private_owners()
        {
            return Ok(Some(0));
        }
        let arity = op.operands.len();
        if !(1..=2).contains(&arity) {
            return Err(invalid("matrix operation arity is unsupported"));
        }
        if args != [self.arguments as u64, arity as u64, 0]
            || self.argument_capacity < arity
            || self.arguments.is_null()
        {
            return Err(invalid("matrix arithmetic source argument lane differs"));
        }
        // The session owns this exact disjoint initialized lane and suspends
        // generated code during this callback. No reference to it escapes.
        let mut words = [0; 2];
        unsafe {
            words[..arity].copy_from_slice(core::slice::from_raw_parts(self.arguments, arity));
        }
        let memory = self.state.library.memory_budget();
        let mut values: [Option<Value>; 2] = [None, None];
        let mut dimensions = [[1usize, 1usize]; 2];
        for at in 0..arity {
            let id = op.operands[at];
            let ty = &m.plan.values()[id as usize];
            if ty.scalar_kind() == Some(mech_engine::native_artifact::NativeScalarKind::F64) {
                continue;
            }
            if words[at] != 0 {
                return Err(invalid("managed arithmetic input has native bits"));
            }
            let Some(value) = self
                .state
                .arena
                .native_entry_preview(id, &self.state.control.work)?
            else {
                return Ok(Some(0));
            };
            if value.schema() != ty.schema()
                || value.schema_key() != ty.schema_key()
                || value.memory_budget_retained_bytes(&memory).is_none()
            {
                return Ok(Some(0));
            }
            let Some(d) = dims(value, ty.definition()) else {
                return Ok(Some(0));
            };
            dimensions[at] = d;
        }
        let call = m.calls[site]
            .as_ref()
            .ok_or_else(|| invalid("matrix arithmetic selected call missing"))?;
        let (rows, columns, left_mode, right_mode, inner) = match op.kind {
            Kind::Elementwise(kind) => {
                if call.template().f64_matrix_binary_op() != Some(kind) {
                    return Err(invalid("matrix arithmetic selected factory differs"));
                }
                let Some([rows, columns, left, right]) =
                    call.f64_matrix_binary_geometry(dimensions[0], dimensions[1])
                else {
                    return Ok(Some(0));
                };
                (rows, columns, left, right, 0)
            }
            Kind::Transpose { .. } => {
                let Some([rows, columns, count]) =
                    call.f64_matrix_transpose_geometry(dimensions[0])
                else {
                    return Ok(Some(0));
                };
                if rows.checked_mul(columns) != Some(count) {
                    return Ok(Some(0));
                }
                (rows, columns, 1, 1, dimensions[0][1] as u64)
            }
            Kind::Sum { axis, .. } => {
                if call.template().f64_matrix_sum_axis() != Some(axis) {
                    return Err(invalid("matrix sum selected factory differs"));
                }
                let Some([rows, columns, input_count, inner]) =
                    call.f64_matrix_sum_geometry(dimensions[0])
                else {
                    return Ok(Some(0));
                };
                if rows.checked_mul(columns).and_then(|n| n.checked_mul(inner)) != Some(input_count)
                {
                    return Ok(Some(0));
                }
                (rows, columns, 1, 1, inner)
            }
            Kind::Product => {
                if !call.template().f64_matrix_product() {
                    return Err(invalid("matrix product selected factory differs"));
                }
                let Some([rows, columns, inner]) =
                    call.f64_matrix_product_geometry(dimensions[0], dimensions[1])
                else {
                    return Ok(Some(0));
                };
                (rows, columns, 1, 1, inner)
            }
        };
        let Some(count) = rows.checked_mul(columns) else {
            return Ok(Some(0));
        };
        let input_count = dimensions[..arity].iter().try_fold(0u64, |n, d| {
            n.checked_add((d[0] as u64).checked_mul(d[1] as u64)?)
        });
        let Some(input_count) = input_count else {
            return Ok(Some(0));
        };
        let bound = match op.kind {
            Kind::Elementwise(_) => call.f64_matrix_binary_source_bound(input_count, count),
            Kind::Transpose { .. } => call.f64_matrix_transpose_source_bound(count),
            Kind::Sum { .. } => call.f64_matrix_sum_source_bound(input_count, count),
            Kind::Product => call.f64_matrix_product_source_bound(input_count, count, inner),
        };
        let Some((demand, replay)) = bound else {
            return Ok(Some(0));
        };
        let caps = self.limits;
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
        let Some(credit) = demand.work.compute.checked_add(replay) else {
            return Ok(Some(0));
        };
        let Some(comparison_credit) = demand.work.comparison.checked_mul(2) else {
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
                .is_none_or(|(w, c)| w < credit || c < comparison_credit)
        {
            return Ok(Some(0));
        }
        let Some(prior) = self.retained_caller_geometry(site)? else {
            return Ok(Some(0));
        };
        if prior
            .live_bytes
            .checked_add(demand.turn_peak_bytes)
            .is_none_or(|v| !fits(v, caps.max_temporary_bytes))
            || prior
                .live_nodes
                .checked_add(demand.retained_nodes)
                .is_none_or(|v| !fits(v, caps.max_retained_nodes))
        {
            return Ok(Some(0));
        }
        let reservation = match memory.reserve_capacity(demand.turn_peak_bytes) {
            Ok(v) => v,
            Err(mech_core::MemoryRuntimeError::BudgetExceeded { .. }) => return Ok(Some(0)),
            Err(e) => return Err(e.into()),
        };
        let node_probe = match memory.reserve_nodes(demand.retained_nodes) {
            Ok(v) => v,
            Err(mech_core::MemoryRuntimeError::BudgetExceeded { .. }) => return Ok(Some(0)),
            Err(e) => return Err(e.into()),
        };
        drop(node_probe);
        // The full coexistence probe includes this private owner and the
        // canonical publication's additional nodes; do not hold the peak twice.
        let nodes = memory.reserve_nodes(3)?;
        // One complete old-source envelope funds all discovery, input owner
        // copies, initialized staging, construction, publication and retirement.
        self.state.control.work.charge(
            crate::native_artifact::NativeExecutionPhase::Node(
                m.plan.blocks()[site].origin().node(),
            ),
            demand.work.compute,
        )?;
        self.state
            .library
            .charge(0, demand.work.comparison)
            .map_err(|error| Error::Collection {
                origin: m.plan.blocks()[site].origin(),
                error,
            })?;
        for at in 0..arity {
            let id = op.operands[at];
            let ty = &m.plan.values()[id as usize];
            if ty.scalar_kind().is_none() {
                let value = self
                    .state
                    .arena
                    .copy_current(id, &mut self.state.control.work)?;
                if value.validate_against(&m.schemas).is_err() {
                    return Ok(Some(0));
                }
                // Retain exact ordinary NaN payload choice after host/native
                // instruction selection. No output is changed on this miss.
                if !matches!(op.kind, Kind::Transpose { .. })
                    && cells(&value).is_none_or(|v| v.iter().any(|x| x.to_f64().is_nan()))
                {
                    return Ok(Some(0));
                }
                values[at] = Some(value);
            } else if f64::from_bits(words[at]).is_nan() {
                return Ok(Some(0));
            }
        }
        let extents = [rows, columns];
        if mech_core::shape_for_resolved_extents(call.template().output(), &extents).is_err() {
            return Ok(Some(0));
        }
        let Ok(shape) = mech_core::shape_for_resolved_extents(
            m.plan.values()[op.output as usize].definition(),
            &extents,
        ) else {
            return Ok(Some(0));
        };
        let Ok(count) = usize::try_from(count) else {
            return Ok(Some(0));
        };
        if core::alloc::Layout::array::<F64Bits>(count).is_err() {
            return Ok(Some(0));
        }
        let mut allocation = Box::new(Pending::Single(Single {
            frame: NumericBinaryFrame {
                left: 0,
                right: 0,
                output: 0,
                count: count as u64,
                columns,
                inner,
                left_mode,
                right_mode,
                left_scalar: words[0],
                right_scalar: words[1],
            },
            issued: NumericBinaryFrame::default(),
            site,
            output: op.output,
            inputs: values,
            cells: vec![F64Bits::from_bits(0); count],
            shape,
            reservation,
            _nodes: nodes,
        }));
        let Pending::Single(pending) = allocation.as_mut() else {
            unreachable!()
        };
        for at in 0..arity {
            if let Some(value) = &pending.inputs[at] {
                let source =
                    cells(value).ok_or_else(|| invalid("matrix arithmetic lost typed source"))?;
                let pointer = source.as_ptr() as u64;
                if at == 0 {
                    pending.frame.left = pointer;
                    if left_mode == 0 {
                        pending.frame.left_scalar = source[0].to_f64().to_bits();
                    }
                } else {
                    pending.frame.right = pointer;
                    if right_mode == 0 {
                        pending.frame.right_scalar = source[0].to_f64().to_bits();
                    }
                }
            }
        }
        pending.frame.output = pending.cells.as_mut_ptr() as u64;
        pending.issued = pending.frame;
        let pointer = &pending.frame as *const NumericBinaryFrame as u64;
        self.numeric_binary = Some(allocation);
        Ok(Some(pointer))
    }
    fn finish_numeric_binary(&mut self, site: usize, args: [u64; 3]) -> Result<Option<u64>, Error> {
        let mut allocation = self
            .numeric_binary
            .take()
            .ok_or_else(|| invalid("matrix arithmetic finish without loan"))?;
        if matches!(allocation.as_ref(), Pending::Region(_)) {
            return self.finish_numeric_region(site, args, allocation);
        }
        let Pending::Single(pending) = allocation.as_mut() else {
            unreachable!()
        };
        let pointer = &pending.frame as *const NumericBinaryFrame as u64;
        if site != pending.site
            || self.state.control.previous != Some(site as u32)
            || self.state.consumed
            || args != [pointer, pending.cells.len() as u64, 0]
            || pending.reservation.budget() != self.state.library.memory_budget()
            || pending.frame.output != pending.cells.as_mut_ptr() as u64
            || pending.frame.count != pending.cells.len() as u64
            || pending.frame != pending.issued
        {
            return Err(invalid(
                "matrix arithmetic finish differs from current loan",
            ));
        }
        let Pending::Single(mut pending) = *allocation else {
            unreachable!()
        };
        let metadata = self.descriptor.metadata();
        let ty = &metadata.types[pending.output as usize];
        let value = Value::from_f64_matrix(
            ty.schema,
            pending.shape,
            pending.cells.into_boxed_slice(),
            std::sync::Arc::clone(&metadata.schemas),
        )
        .map_err(Error::Snapshot)?
        .into_memory_budget(&mut pending.reservation)?;
        if value.schema_key() != ty.key {
            return Err(invalid("matrix arithmetic output schema differs"));
        }
        self.state
            .arena
            .put(pending.output, &value, &mut self.state.control.work)?;
        self.state.consumed = true;
        Ok(Some(1))
    }
}
