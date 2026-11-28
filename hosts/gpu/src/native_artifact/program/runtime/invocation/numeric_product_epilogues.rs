//! One checked source-closed product/cell-expression or producer/sum loan. No intermediate
//! matrix owner or payload exists; only the final initialized bank publishes.
use super::numeric_binary::{cells, dims, fits};
use super::{Error, Invocation, invalid};
use crate::native_artifact::iteration_abi::Progress;
use crate::native_artifact::program::{
    prepare::{
        numeric_binary::Kind,
        numeric_product_epilogues::{MAX_INPUTS, Source, Step},
    },
    protocol::{
        GlobalHelper,
        abi::{NumericBinaryFrame, NumericProductFrame},
    },
};
use mech_core::{
    ManagedMemoryReservation, ManagedNodeReservation, ResourceDemand, ShapeInstance, Value,
    snapshot::F64Bits,
};
use mech_engine::native_artifact::NativeScalarKind;

pub(in crate::native_artifact::program::runtime) struct Loan {
    pub frame: NumericProductFrame,
    issued: NumericProductFrame,
    site: usize,
    output: u32,
    epoch: u64,
    _inputs: Vec<Option<Value>>,
    cells: Vec<F64Bits>,
    shape: ShapeInstance,
    reservation: ManagedMemoryReservation,
    _nodes: ManagedNodeReservation,
}
fn source_dims(source: Source, external: &[[usize; 2]], nodes: &[[usize; 2]]) -> [usize; 2] {
    match source {
        Source::External(at) => external[at],
        Source::Node(at) => nodes[at],
    }
}
impl Invocation<'_, '_, '_> {
    pub(super) fn numeric_product_helper(
        &mut self,
        site: usize,
        tag: GlobalHelper,
        args: [u64; 3],
    ) -> Result<Option<u64>, Error> {
        if tag == GlobalHelper::NumericProductFinish {
            return self.finish_numeric_product(site, args);
        }
        let descriptor = self.descriptor;
        let m = descriptor.metadata();
        let recipe = m
            .numeric_binary
            .epilogue(site)
            .ok_or_else(|| invalid("product epilogue outside source proof"))?;
        let reduction_axis = recipe.reduction_axis();
        let product_index = recipe.product_index();
        if tag != GlobalHelper::NumericProductPrepare
            || args != [self.arguments as u64, recipe.scalars.len() as u64, 0]
            || self.arguments.is_null()
            || self.argument_capacity < recipe.scalars.len()
            || self.state.consumed
            || self.numeric_product.is_some()
            || self.state.control.previous != Some(site as u32)
            || self.state.control.current != recipe.parent
        {
            return Err(invalid("product epilogue outside fresh exact entry"));
        }
        if self.worker.is_some()
            || self.state.scalar_path.is_some()
            || self.numeric_prepaid.is_some()
            || self.has_uninventoried_private_owners()
            || recipe.steps.iter().any(|s| {
                let at = s.site() as usize;
                let v = &m.visits[at];
                v.progress != Progress::Normal
                    || v.complete
                    || v.arithmetic
                    || v.reject
                    || v.integrity
                    || m.map_admission.group(at).is_some()
            })
        {
            return Ok(Some(0));
        }
        // Exact synchronous session-owned argument lane, never an image pointer.
        let scalar_words =
            unsafe { core::slice::from_raw_parts(self.arguments, recipe.scalars.len()) };
        let mut words = [0u64; MAX_INPUTS];
        for ((at, _), word) in recipe.scalars.iter().zip(scalar_words) {
            words[*at] = *word;
        }
        let memory = self.state.library.memory_budget();
        let mut external_dims = [[1usize; 2]; MAX_INPUTS];
        let mut external_bytes = [0u64; MAX_INPUTS];
        for (at, &id) in recipe.external.iter().enumerate() {
            let ty = &m.plan.values()[id.get() as usize];
            if ty.scalar_kind() == Some(NativeScalarKind::F64) {
                continue;
            }
            if words[at] != 0 {
                return Err(invalid("product epilogue managed argument has native bits"));
            }
            let Some(value) = self
                .state
                .arena
                .native_entry_preview(id.get(), &self.state.control.work)?
            else {
                return Ok(Some(0));
            };
            if value.schema() != ty.schema() || value.schema_key() != ty.schema_key() {
                return Ok(Some(0));
            }
            let Some(bytes) = value.memory_budget_retained_bytes(&memory) else {
                return Ok(Some(0));
            };
            external_bytes[at] = bytes;
            let Some(dimensions) = dims(value, ty.definition()) else {
                return Ok(Some(0));
            };
            external_dims[at] = dimensions;
        }
        let admitted = (|| {
            let mut node_dims = Vec::with_capacity(recipe.nodes.len());
            let mut demands: Vec<ResourceDemand> = Vec::with_capacity(recipe.nodes.len());
            let mut modes = [1u64; MAX_INPUTS];
            let mut product_axes = [0u64; 3];
            let mut domain_axes = [0usize; 2];
            // Pure source bounds are computed once for this exact geometry.
            // Reuse their replay credit; no metadata or loan survives the call.
            let mut replay = 0u64;
            let mut output_shape = None;
            for (index, node) in recipe.nodes.iter().enumerate() {
                let call = m.calls[node.site as usize].as_ref()?;
                let inputs = node
                    .inputs
                    .map(|s| source_dims(s, &external_dims, &node_dims));
                let (rows, columns, inner) = match node.kind {
                    Kind::Transpose { .. } if recipe.transposed_left() && index == 0 => {
                        let [r, c, count] = call.f64_matrix_transpose_geometry(inputs[0])?;
                        if r.checked_mul(c)? != count {
                            return None;
                        }
                        (r, c, 0)
                    }
                    Kind::Product if Some(index) == product_index => {
                        let [r, c, k] = call.f64_matrix_product_geometry(inputs[0], inputs[1])?;
                        product_axes = [r, c, k];
                        (r, c, k)
                    }
                    Kind::Elementwise(op)
                        if (product_index.is_some_and(|p| index > p)
                            || reduction_axis.is_some())
                            && call.template().f64_matrix_binary_op() == Some(op) =>
                    {
                        let [r, c, left, right] =
                            call.f64_matrix_binary_geometry(inputs[0], inputs[1])?;
                        if reduction_axis.is_none()
                            && ((r != 1 && r != product_axes[0])
                                || (c != 1 && c != product_axes[1]))
                        {
                            return None;
                        }
                        for (input, source) in node.inputs.iter().enumerate() {
                            if reduction_axis.is_none()
                                && let Source::External(at) = *source
                            {
                                modes[at] = call.f64_matrix_binary_input_mode(
                                    input,
                                    external_dims[at],
                                    [
                                        usize::try_from(product_axes[0]).ok()?,
                                        usize::try_from(product_axes[1]).ok()?,
                                    ],
                                )?;
                            }
                        }
                        let _ = (left, right);
                        (r, c, 0)
                    }
                    Kind::Sum { axis, .. }
                        if reduction_axis == Some(axis)
                            && index + 1 == recipe.nodes.len()
                            && call.template().f64_matrix_sum_axis() == Some(axis) =>
                    {
                        let [r, c, input_count, k] = call.f64_matrix_sum_geometry(inputs[0])?;
                        if r.checked_mul(c)?.checked_mul(k)? != input_count {
                            return None;
                        }
                        domain_axes = inputs[0];
                        product_axes = [r, c, k];
                        (r, c, k)
                    }
                    _ => return None,
                };
                let count = rows.checked_mul(columns)?;
                let input_count = inputs[..node.operands().len()]
                    .iter()
                    .try_fold(0u64, |sum, axes| {
                        sum.checked_add((axes[0] as u64).checked_mul(axes[1] as u64)?)
                    })?;
                let (demand, node_replay) = match node.kind {
                    Kind::Product => {
                        call.f64_matrix_product_source_bound(input_count, count, inner)?
                    }
                    Kind::Elementwise(_) => {
                        call.f64_matrix_binary_source_bound(input_count, count)?
                    }
                    Kind::Sum { .. } => call.f64_matrix_sum_source_bound(input_count, count)?,
                    Kind::Transpose { .. } => call.f64_matrix_transpose_source_bound(count)?,
                };
                replay = replay.checked_add(node_replay)?;
                // Every omitted logical factory still has its original shape.
                mech_core::shape_for_resolved_extents(call.template().output(), &[rows, columns])
                    .ok()?;
                let source_shape = mech_core::shape_for_resolved_extents(
                    m.plan.values()[node.output.get() as usize].definition(),
                    &[rows, columns],
                )
                .ok()?;
                if index + 1 == recipe.nodes.len() && node.output == recipe.output {
                    output_shape = Some(source_shape);
                }
                node_dims.push([usize::try_from(rows).ok()?, usize::try_from(columns).ok()?]);
                demands.push(demand);
            }
            if reduction_axis.is_some() {
                // Every pointwise node is evaluated at coordinates in the
                // original sum input. Only ordinary scalar/row/column/exact
                // broadcasting can repeat a smaller producer at those cells.
                for (index, node) in recipe.nodes.iter().enumerate() {
                    if !matches!(node.kind, Kind::Elementwise(_)) {
                        continue;
                    }
                    if node_dims[index]
                        .iter()
                        .zip(domain_axes)
                        .any(|(&a, d)| a != 1 && a != d)
                    {
                        return None;
                    }
                    let call = m.calls[node.site as usize].as_ref()?;
                    for (input, source) in node.operands().iter().enumerate() {
                        if let Source::External(at) = *source {
                            modes[at] = call.f64_matrix_binary_input_mode(
                                input,
                                external_dims[at],
                                domain_axes,
                            )?;
                        }
                    }
                }
            }
            let [rows, columns, inner] = product_axes;
            if node_dims.last().copied()
                != Some([usize::try_from(rows).ok()?, usize::try_from(columns).ok()?])
            {
                return None;
            }
            let count = rows.checked_mul(columns)?;
            // The last-node axis equality above proves these are the same
            // extents. Only the exact same source output may reuse its shape;
            // a differently named alias retains its original validation.
            let shape = match output_shape {
                Some(shape) => shape,
                None => mech_core::shape_for_resolved_extents(
                    m.plan.values()[recipe.output.get() as usize].definition(),
                    &[rows, columns],
                )
                .ok()?,
            };
            let metadata = (core::mem::size_of::<Loan>() as u64)
                .checked_add(
                    (recipe.external.len() as u64)
                        .checked_mul(core::mem::size_of::<Option<Value>>() as u64)?,
                )?
                .checked_add(1024)?;
            let mut compute = 0u64;
            let mut comparison = 0u64;
            let mut temporary = metadata;
            let mut nodes = 5u64;
            let mut cloned = 0u64;
            let mut max_output = 0u64;
            let mut max_elements = count;
            for (index, (node, demand)) in recipe.nodes.iter().zip(&demands).enumerate() {
                compute = compute.checked_add(demand.work.compute)?;
                comparison = comparison.checked_add(demand.work.comparison)?;
                temporary = temporary
                    .checked_add(demand.turn_peak_bytes)?
                    .checked_add(demand.persistent_bytes)?;
                nodes = nodes.checked_add(demand.retained_nodes)?;
                cloned = cloned.checked_add(demand.cloned_bytes)?;
                max_output = max_output.max(demand.persistent_bytes);
                max_elements = max_elements.max(demand.output_elements);
                // Ordinary input copies and canonical puts/retirements remain
                // source obligations even when their physical owners disappear.
                let managed = node
                    .operands()
                    .iter()
                    .filter(|source| match **source {
                        Source::Node(_) => true,
                        Source::External(at) => m.plan.values()[recipe.external[at].get() as usize]
                            .scalar_kind()
                            .is_none(),
                    })
                    .count() as u64;
                compute = compute.checked_add(managed.checked_mul(32)?)?;
                if index + 1 < recipe.nodes.len() {
                    compute = compute
                        .checked_add(demand.persistent_bytes.checked_mul(4)?.checked_add(128)?)?;
                }
            }
            for step in &recipe.steps {
                if step.site() != recipe.entry {
                    compute = compute.checked_add(m.visits[step.site() as usize].work)?;
                }
                if let Step::Alias { source, output, .. } = *step {
                    let retained = match source {
                        Source::External(at) => external_bytes[at],
                        Source::Node(at) => demands[at].persistent_bytes,
                    };
                    compute = compute.checked_add(retained.checked_mul(4)?.checked_add(128)?)?;
                    if m.plan.values()[output.get() as usize]
                        .scalar_kind()
                        .is_none()
                    {
                        let ds = source_dims(source, &external_dims, &node_dims);
                        mech_core::shape_for_resolved_extents(
                            m.plan.values()[output.get() as usize].definition(),
                            &ds.map(|v| v as u64),
                        )
                        .ok()?;
                    }
                }
            }
            // Each cheap pointwise node may be repeated across broadcast
            // consumers. Admit that bounded physical work explicitly; the
            // ordered product itself is never repeated across output cells.
            let native_steps =
                (recipe.nodes.len() as u64).checked_add(recipe.external.len() as u64)?;
            let physical_cells = if reduction_axis.is_some() {
                count.checked_mul(inner)?
            } else {
                count
            };
            compute =
                compute.checked_add(physical_cells.checked_mul(native_steps.checked_mul(128)?)?)?;
            comparison = comparison
                .checked_add(physical_cells.checked_mul(native_steps.checked_mul(4)?)?)?;
            let credit = compute
                .checked_mul(3)?
                .checked_add(replay.checked_mul(2)?)?;
            Some((
                [rows, columns, inner],
                count,
                shape,
                modes,
                compute,
                comparison,
                temporary,
                nodes,
                cloned,
                max_output,
                max_elements,
                credit,
            ))
        })();
        let Some((
            [rows, columns, inner],
            count,
            shape,
            modes,
            compute,
            comparison,
            temporary,
            nodes,
            cloned,
            max_output,
            max_elements,
            credit,
        )) = admitted
        else {
            return Ok(Some(0));
        };
        let caps = self.limits;
        if !fits(max_elements, caps.max_output_elements)
            || !fits(max_output, caps.max_output_bytes)
            || !fits(temporary, caps.max_temporary_bytes)
            || !fits(nodes, caps.max_retained_nodes)
            || !fits(cloned, caps.max_cloned_bytes)
            || !fits(compute, caps.max_compute_work)
            || !fits(comparison, caps.max_comparison_work)
            || !fits(0, caps.max_scalar_instructions)
            || !fits(0, caps.max_transfer_bytes)
            || !fits(0, caps.max_storage_buffer_bytes)
            || caps.max_storage_bindings.is_none()
            || self
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
                .is_none_or(|(w, c)| w < credit || comparison.checked_mul(3).is_none_or(|n| c < n))
        {
            return Ok(Some(0));
        }
        let Some(prior) = self.retained_caller_geometry(site)? else {
            return Ok(Some(0));
        };
        if prior
            .live_bytes
            .checked_add(temporary)
            .is_none_or(|n| !fits(n, caps.max_temporary_bytes))
            || prior
                .live_nodes
                .checked_add(nodes)
                .is_none_or(|n| !fits(n, caps.max_retained_nodes))
        {
            return Ok(Some(0));
        }
        let reservation = match memory.reserve_capacity(temporary) {
            Ok(v) => v,
            Err(mech_core::MemoryRuntimeError::BudgetExceeded { .. }) => return Ok(Some(0)),
            Err(e) => return Err(e.into()),
        };
        let node_probe = match memory.reserve_nodes(nodes) {
            Ok(v) => v,
            Err(mech_core::MemoryRuntimeError::BudgetExceeded { .. }) => return Ok(Some(0)),
            Err(e) => return Err(e.into()),
        };
        drop(node_probe);
        let nodes = memory.reserve_nodes(5)?;
        let Ok(length) = usize::try_from(count) else {
            return Ok(Some(0));
        };
        if core::alloc::Layout::array::<F64Bits>(length).is_err() {
            return Ok(Some(0));
        }
        self.state.control.work.charge(
            crate::native_artifact::NativeExecutionPhase::Node(
                m.plan.blocks()[site].origin().node(),
            ),
            compute,
        )?;
        self.state
            .library
            .charge(0, comparison)
            .map_err(|error| Error::Collection {
                origin: m.plan.blocks()[site].origin(),
                error,
            })?;
        let mut inputs = Vec::with_capacity(recipe.external.len());
        let mut frame = NumericProductFrame::default();
        for (at, &id) in recipe.external.iter().enumerate() {
            if m.plan.values()[id.get() as usize].scalar_kind() == Some(NativeScalarKind::F64) {
                if f64::from_bits(words[at]).is_nan() {
                    return Ok(Some(0));
                }
                frame.leaves[at].scalar = words[at];
                frame.leaves[at].mode = 0;
                inputs.push(None);
            } else {
                let value = self
                    .state
                    .arena
                    .copy_current(id.get(), &mut self.state.control.work)?;
                if value.validate_against(&m.schemas).is_err() {
                    return Ok(Some(0));
                }
                let Some(bank) = cells(&value) else {
                    return Ok(Some(0));
                };
                if bank.iter().any(|x| x.to_f64().is_nan()) {
                    return Ok(Some(0));
                }
                frame.leaves[at].pointer = bank.as_ptr() as u64;
                frame.leaves[at].mode = modes[at];
                frame.leaves[at].scalar = bank.first().map_or(0, |x| x.to_f64().to_bits());
                inputs.push(Some(value));
            }
        }
        let Some([left, right]) = recipe.physical_roots() else {
            return Err(invalid("product epilogue lost original product sources"));
        };
        frame.product = NumericBinaryFrame {
            left: frame.leaves[left].pointer,
            right: frame.leaves[right].pointer,
            output: 0,
            count,
            columns,
            inner,
            left_mode: 1,
            right_mode: 1,
            left_scalar: 0,
            right_scalar: 0,
        };
        let mut loan = Box::new(Loan {
            frame,
            issued: NumericProductFrame::default(),
            site,
            output: recipe.output.get(),
            epoch: self.state.control.visits,
            _inputs: inputs,
            cells: vec![F64Bits::from_bits(0); length],
            shape,
            reservation,
            _nodes: nodes,
        });
        loan.frame.product.output = loan.cells.as_mut_ptr() as u64;
        loan.issued = loan.frame;
        let pointer = &loan.frame as *const NumericProductFrame as u64;
        #[cfg(all(test, feature = "canonical-native-program"))]
        {
            let mut present = false;
            for &value in &recipe.elided_values {
                present |= self
                    .state
                    .arena
                    .native_entry_preview(value, &self.state.control.work)?
                    .is_some();
            }
            super::super::composition_tests::observe::product_epilogue(
                site,
                recipe.nodes.len(),
                length,
                present,
            );
        }
        let _ = rows;
        self.numeric_product = Some(loan);
        Ok(Some(pointer))
    }
    fn finish_numeric_product(
        &mut self,
        site: usize,
        args: [u64; 3],
    ) -> Result<Option<u64>, Error> {
        let mut loan = self
            .numeric_product
            .take()
            .ok_or_else(|| invalid("product epilogue finish without loan"))?;
        let recipe = self
            .descriptor
            .metadata()
            .numeric_binary
            .epilogue(site)
            .ok_or_else(|| invalid("product epilogue finish lost source proof"))?;
        if site != loan.site
            || self.state.control.previous != Some(site as u32)
            || self.state.consumed
            || self.state.control.visits != loan.epoch
            || self.state.control.current != recipe.parent
            || loan.output != recipe.output.get()
            || args[0] != &loan.frame as *const NumericProductFrame as u64
            || args[1] != loan.cells.len() as u64
            || args[2] > 1
            || loan.frame != loan.issued
            || loan.frame.product.output != loan.cells.as_mut_ptr() as u64
            || loan.reservation.budget() != self.state.library.memory_budget()
        {
            return Err(invalid(
                "product epilogue finish differs from current sealed loan",
            ));
        }
        // Cancellation discards every private cell and leaves the original
        // admitted producer unconsumed, with complete ordinary replay credit.
        if args[2] == 1 {
            return Ok(Some(0));
        }
        let m = self.descriptor.metadata();
        let ty = &m.types[loan.output as usize];
        let value = Value::from_f64_matrix(
            ty.schema,
            loan.shape,
            loan.cells.into_boxed_slice(),
            std::sync::Arc::clone(&m.schemas),
        )
        .map_err(Error::Snapshot)?
        .into_memory_budget(&mut loan.reservation)?;
        if value.schema_key() != ty.key {
            return Err(invalid("product epilogue final schema differs"));
        }
        self.state
            .arena
            .put(loan.output, &value, &mut self.state.control.work)?;
        self.state.control.previous = Some(recipe.end);
        self.state.consumed = true;
        Ok(Some(1))
    }
}
