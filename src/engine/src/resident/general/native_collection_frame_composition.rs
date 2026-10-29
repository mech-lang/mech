//! Same selected admission with current private leaves, never canonical records.
//! Immutable structural paths are rederived from the exact native frame proof.
use super::{InvokeError, NativePreparedLibraryCall, finite_limits};
use crate::native_artifact::{
    NativeBufferFrameField as Field, NativeBufferFrameOperation as Operation,
    NativeBufferFrames as Layout, NativeCollectionPrimitive as Primitive, NativeLibraryWorkBudget,
    NativeMatchValue, NativeScalarKind, NativeScalarValue,
};
use crate::resident::budget::{
    ResidentBudgetMeter, measure_borrowed_canonical_value_footprint,
    measure_native_scalar_frame_footprint, native_turn_plan_cache::NativeProjectionIndexContext,
};
use mech_core::{MemoryBudgetLimits, ResourceDemand, SchemaBody, SchemaId, SchemaTable, Value};

#[derive(Debug)]
enum Source {
    Input(usize),
    Leaf(Field),
}
#[derive(Debug)]
pub(super) struct Composition {
    sources: Box<[Source]>,
    schemas: Box<[SchemaId]>,
    shape: super::frame_shape::FrameShapeRecipe,
    admission: Option<super::frame_admission::FrameAdmissionRecipe>,
    metadata: u64,
    remove_wrappers: u64,
    add_containers: u64,
    elements: u64,
}

fn components(body: &SchemaBody) -> Option<Vec<&SchemaBody>> {
    match body {
        SchemaBody::Tuple(fields) => Some(fields.iter().collect()),
        SchemaBody::Record(fields) => Some(fields.iter().map(|f| &f.schema).collect()),
        _ => None,
    }
}
fn wrapper(schema: SchemaId, schemas: &SchemaTable) -> Option<u64> {
    Some(
        (core::mem::size_of::<Value>() as u64).checked_add(
            (schemas.get(schema)?.dimension_parameters().len() as u64).checked_mul(8)?,
        )?,
    )
}

struct Leaves<'a> {
    layout: &'a Layout,
    table: &'a SchemaTable,
    sources: Vec<Source>,
    schemas: Vec<SchemaId>,
    expected: Vec<&'a SchemaBody>,
    remove: u64,
    add: u64,
}
impl<'a> Leaves<'a> {
    fn composite(
        &mut self,
        expected: &'a SchemaBody,
        schema: SchemaId,
        fields: &[Field],
    ) -> Option<()> {
        let actual = self.table.get(schema)?.body();
        match (expected, actual) {
            (SchemaBody::Record(a), SchemaBody::Record(b))
                if a.len() == b.len() && a.iter().zip(b).all(|(a, b)| a.name == b.name) => {}
            (SchemaBody::Tuple(a), SchemaBody::Tuple(b)) if a.len() == b.len() => {}
            _ => return None,
        }
        let expected = components(expected)?;
        if expected.len() != fields.len() {
            return None;
        }
        self.add = self
            .add
            .checked_add(core::mem::size_of::<mech_core::ValueData>() as u64)?;
        for (expected, field) in expected.into_iter().zip(fields) {
            if let Some(children) = self.layout.children(*field) {
                self.composite(expected, field.schema(), children)?;
            } else {
                self.remove = self
                    .remove
                    .checked_add(wrapper(field.schema(), self.table)?)?;
                self.sources.push(Source::Leaf(*field));
                self.schemas.push(field.schema());
                self.expected.push(expected);
            }
        }
        Some(())
    }
}

impl NativePreparedLibraryCall {
    /// Preparation only. The same validated plan supplies all structural spans;
    /// an image loader rederives this recipe rather than trusting serialized paths.
    pub fn prepare_buffer_frame_composition(
        &mut self,
        layout: &Layout,
        operation: &Operation,
        operands: &[NativeMatchValue],
    ) -> Result<(), &'static str> {
        self.frame_composition = None;
        let mech_core::ResolvedOperationContract::Declared(contract) = self.template.contract()
        else {
            return Err("private composition has no selected contract");
        };
        let output = contract.outputs[0].schema;
        let mut leaves = Leaves {
            layout,
            table: &self.schemas,
            sources: Vec::new(),
            schemas: Vec::new(),
            expected: Vec::new(),
            remove: 0,
            add: 0,
        };
        let elements = match operation {
            Operation::Pack { .. } => {
                if !operands.iter().any(|v| layout.fields(*v).is_some()) {
                    return Ok(());
                }
                let fields = components(self.template.output().body())
                    .ok_or("private pack is not composite")?;
                if fields.len() != operands.len() || contract.inputs.len() != operands.len() {
                    return Err("private composition arity differs");
                }
                for (index, (expected, value)) in fields.into_iter().zip(operands).enumerate() {
                    let schema = contract.inputs[index].schema;
                    if let Some(fields) = layout.fields(*value) {
                        leaves.add = leaves
                            .add
                            .checked_add(
                                wrapper(schema, &self.schemas).ok_or("private wrapper overflow")?,
                            )
                            .ok_or("private logical footprint overflow")?;
                        leaves
                            .composite(expected, schema, fields)
                            .ok_or("private child structure differs")?;
                    } else {
                        leaves.sources.push(Source::Input(index));
                        leaves.schemas.push(schema);
                        leaves.expected.push(expected);
                    }
                }
                operands.len() as u64
            }
            Operation::Project {
                source, ordinal, ..
            } => {
                let Some(field) = layout.fields(*source).and_then(|f| f.get(*ordinal)) else {
                    return Ok(());
                };
                let Some(children) = layout.children(*field) else {
                    return Ok(());
                };
                if self.template.primitive() != Primitive::RecordFieldAccess
                    || field.schema() != output
                {
                    return Err("private child projection schema differs");
                }
                leaves.add = wrapper(output, &self.schemas).ok_or("private wrapper overflow")?;
                leaves
                    .composite(self.template.output().body(), output, children)
                    .ok_or("private projection structure differs")?;
                1
            }
            Operation::Copy { .. } => return Ok(()),
        };
        let shape = super::frame_shape::FrameShapeRecipe::derive_leaves(
            output,
            &leaves.schemas,
            &leaves.expected,
            &self.schemas,
        )
        .ok_or("private composition has unsupported shape relations")?;
        let metadata = self
            .schemas
            .clone_allocation_bound_bytes()
            .and_then(|n| n.checked_add(self.binding_bytes))
            .and_then(|n| n.checked_add((leaves.sources.len() as u64).checked_mul(1024)?))
            .and_then(|n| n.checked_add(4096))
            .ok_or("private composition metadata overflow")?;
        self.frame_composition = Some(Box::new(Composition {
            admission: super::frame_admission::FrameAdmissionRecipe::derive(
                &leaves.schemas,
                &self.schemas,
            ),
            shape,
            sources: leaves.sources.into(),
            schemas: leaves.schemas.into(),
            metadata,
            remove_wrappers: leaves.remove,
            add_containers: leaves.add,
            elements,
        }));
        Ok(())
    }

    /// Reads only this invocation's current scalar/managed lanes. This grants
    /// neither source-site entry nor publication and retains no previous visit.
    pub fn admit_buffer_frame_composition<'a>(
        &self,
        inputs: &'a [Option<Value>],
        words: &[u64],
        current: impl Fn(Field) -> Option<(Option<&'a Value>, u64)>,
        selector: Option<(&Value, usize)>,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        coexistence: ResourceDemand,
    ) -> Result<(), InvokeError> {
        finite_limits(self.template.origin().node(), limits).map_err(InvokeError::Binding)?;
        let recipe = self
            .frame_composition
            .as_ref()
            .ok_or(InvokeError::InvalidInput)?;
        if inputs.len() != words.len()
            || inputs.len() != self.template.inputs().len()
            || (selector.is_some() != (self.template.primitive() == Primitive::RecordFieldAccess))
        {
            return Err(InvokeError::InvalidInput);
        }
        let read = |index: usize| match recipe.sources[index] {
            Source::Input(input) => Some((inputs[input].as_ref(), words[input])),
            Source::Leaf(field) => current(field),
        };
        work.with_call_limits(
            limits.max_compute_work.unwrap(),
            limits.max_comparison_work.unwrap(),
            || {
                let memory = work.memory_budget();
                let context = NativeProjectionIndexContext::new(work, limits, coexistence);
                context.charge(
                    recipe
                        .metadata
                        .checked_mul(8)
                        .ok_or(InvokeError::InvalidInput)?,
                    0,
                )?;
                context.check_capacity(recipe.metadata, recipe.sources.len() as u64 + 8)?;
                let _memory = memory.reserve_capacity(recipe.metadata)?;
                let _nodes = memory.reserve_nodes(recipe.sources.len() as u64 + 8)?;
                if let Some((selector, ordinal)) = selector {
                    self.admit_composed_selector(selector, ordinal, &memory, &context)?;
                }
                let mut meter = ResidentBudgetMeter::for_native_projection(work, limits)?;
                let measured = match &recipe.admission {
                    Some(admission) => {
                        admission.try_admit_current(&self.schemas, read, &memory, &mut meter)?
                    }
                    None => None,
                };
                let mut logical = measured.unwrap_or(0);
                if measured.is_none() {
                    for (index, schema) in recipe.schemas.iter().enumerate() {
                        let (value, bits) = read(index).ok_or(InvokeError::InvalidInput)?;
                        let definition =
                            self.schemas.get(*schema).ok_or(InvokeError::InvalidInput)?;
                        let footprint = if let Some(value) = value {
                            if bits != 0
                                || value.schema() != *schema
                                || value.schema_key() != definition.key()
                                || value.memory_budget_retained_bytes(&memory).is_none()
                            {
                                return Err(InvokeError::InvalidInput);
                            }
                            value
                                .validate_against(&self.schemas)
                                .map_err(|_| InvokeError::InvalidInput)?;
                            measure_borrowed_canonical_value_footprint(
                                &mut meter,
                                value,
                                &self.schemas,
                            )?
                        } else {
                            if !definition.dimension_parameters().is_empty() {
                                return Err(InvokeError::InvalidInput);
                            }
                            let kind = NativeScalarKind::from_schema_body(definition.body())
                                .ok_or(InvokeError::InvalidInput)?;
                            let value = NativeScalarValue::from_bits(kind, bits)
                                .ok_or(InvokeError::InvalidInput)?;
                            measure_native_scalar_frame_footprint(
                                &mut meter,
                                definition.body(),
                                value,
                            )?
                        };
                        logical = logical
                            .checked_add(footprint.retained_bytes)
                            .ok_or(InvokeError::InvalidInput)?;
                    }
                }
                if measured.is_none()
                    || !recipe
                        .shape
                        .matches_current(|i| read(i).and_then(|(v, _)| v.map(Value::shape)))
                {
                    recipe.shape.validate(&self.schemas, |i| {
                        read(i).and_then(|(v, _)| v.map(Value::shape))
                    })?;
                }
                // A composed root has one canonical logical wrapper and one DATA
                // container per structural node, rather than a wrapper per leaf.
                logical = logical
                    .checked_sub(recipe.remove_wrappers)
                    .and_then(|n| n.checked_add(recipe.add_containers))
                    .ok_or(InvokeError::InvalidInput)?;
                if let Some(violation) = mech_core::evaluate_call_memory_budget(
                    mech_core::MemoryObjectOwner::NodeOutput {
                        node: self.template.origin().node(),
                        port: 0,
                    },
                    ResourceDemand {
                        output_elements: recipe.elements,
                        ..ResourceDemand::default()
                    },
                    logical,
                    0,
                    limits,
                )
                .first()
                {
                    return Err(InvokeError::Plan(
                        mech_core::MemoryPlanError::TargetLimitExceeded {
                            violation: violation.clone(),
                        },
                    ));
                }
                Ok(())
            },
        )?
    }

    fn admit_composed_selector(
        &self,
        selector: &Value,
        ordinal: usize,
        memory: &mech_core::ManagedMemoryBudget,
        context: &NativeProjectionIndexContext<'_>,
    ) -> Result<(), InvokeError> {
        let mech_core::ResolvedOperationContract::Declared(contract) = self.template.contract()
        else {
            return Err(InvokeError::InvalidInput);
        };
        if selector.schema() != contract.inputs[1].schema
            || selector.schema_key() != self.input_keys[1]
        {
            return Err(InvokeError::InvalidInput);
        }
        selector
            .validate_against(&self.schemas)
            .map_err(|_| InvokeError::InvalidInput)?;
        let bytes = selector
            .memory_budget_retained_bytes(memory)
            .ok_or(InvokeError::UnadmittedValue)?;
        let nodes = selector
            .memory_budget_retained_nodes(memory)
            .ok_or(InvokeError::UnadmittedValue)?;
        let context = context.with_added_capacity(bytes, nodes)?;
        context.check_capacity(0, 0)?;
        self.check_buffer_record_ordinal(selector, ordinal, &context)
    }
}
