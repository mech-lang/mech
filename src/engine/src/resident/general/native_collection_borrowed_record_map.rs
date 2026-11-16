//! Selected immutable Record→Map view. The canonical parent remains the only
//! owner; no child Value, draft, binding, turn plan or pointer escapes this call.
use super::{InvokeError, NativePreparedLibraryCall, finite_limits};
use crate::{
    native_artifact::NativeCollectionPrimitive as Primitive,
    resident::{
        budget::{
            NativeLibraryWorkBudget, ResidentBudgetMeter,
            measure_borrowed_canonical_component_footprint,
            measure_borrowed_canonical_value_footprint,
            native_turn_plan_cache::NativeProjectionIndexContext,
        },
        numeric::string_map_access::{self, Budget},
    },
};
use mech_core::{
    IntegerWidth, MemoryBudgetLimits, ResidentKernelError, ResourceDemand, SchemaBody, SchemaId,
    Value, ValueData, snapshot::MapValue,
};

fn string_u64(body: &SchemaBody) -> bool {
    matches!(body, SchemaBody::Map { key, value, .. }
        if key.as_ref() == &SchemaBody::String
            && value.as_ref() == &SchemaBody::UnsignedInteger(IntegerWidth::W64))
}

fn record_map<'a>(
    source: &'a Value,
    schemas: &mech_core::SchemaTable,
    schema: SchemaId,
    ordinal: usize,
    expected: &SchemaBody,
) -> Result<(&'a MapValue, &'a ValueData), InvokeError> {
    let definition = schemas.get(schema).ok_or(InvokeError::InvalidInput)?;
    let entry = schemas.entry(schema).ok_or(InvokeError::InvalidInput)?;
    if source.schema() != schema
        || source.schema_key() != entry.key()
        || !source.shape().parameter_values().is_empty()
        || !definition.dimension_parameters().is_empty()
    {
        return Err(InvokeError::InvalidInput);
    }
    source
        .validate_against(schemas)
        .map_err(|_| InvokeError::InvalidInput)?;
    let (SchemaBody::Record(fields), ValueData::Record(record)) =
        (definition.body(), source.data())
    else {
        return Err(InvokeError::InvalidInput);
    };
    if fields.is_empty()
        || fields.len() > 32
        || fields.len() != record.fields().len()
        || fields.iter().filter(|f| string_u64(&f.schema)).count() != 1
        || fields.iter().any(|f| {
            !string_u64(&f.schema) && f.schema != SchemaBody::UnsignedInteger(IntegerWidth::W64)
        })
        || fields
            .get(ordinal)
            .is_none_or(|field| &field.schema != expected)
    {
        return Err(InvokeError::InvalidInput);
    }
    match record.fields().get(ordinal) {
        Some(data @ ValueData::Map(map)) => Ok((map, data)),
        _ => Err(InvokeError::InvalidInput),
    }
}

fn admitted(value: &Value, work: &NativeLibraryWorkBudget) -> Result<(u64, u64), InvokeError> {
    let memory = work.memory_budget();
    Ok((
        value
            .memory_budget_retained_bytes(&memory)
            .ok_or(InvokeError::UnadmittedValue)?,
        value
            .memory_budget_retained_nodes(&memory)
            .ok_or(InvokeError::UnadmittedValue)?,
    ))
}

impl NativePreparedLibraryCall {
    pub fn supports_borrowed_record_map_projection(&self) -> bool {
        self.template.primitive() == Primitive::RecordFieldAccess
            && self.template.inputs().len() == 2
            && self.template.inputs()[1].body() == &SchemaBody::Id
            && self
                .template
                .inputs()
                .iter()
                .all(|s| s.dimension_parameters().is_empty())
            && self.template.output().dimension_parameters().is_empty()
            && string_u64(self.template.output().body())
    }

    /// Admit the genuine selected field operation over a current canonical
    /// parent. The caller separately proves its exact source generation and
    /// complete read-only use set. Coexistence excludes source/selector, which
    /// are included here once. Logical output/clone caps remain canonical.
    pub fn admit_borrowed_record_map(
        &self,
        source: &Value,
        selector: &Value,
        ordinal: usize,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        coexistence: ResourceDemand,
    ) -> Result<(), InvokeError> {
        finite_limits(self.template.origin().node(), limits).map_err(InvokeError::Binding)?;
        if !self.supports_borrowed_record_map_projection() {
            return Err(InvokeError::InvalidInput);
        }
        work.with_call_limits(
            limits.max_compute_work.unwrap(),
            limits.max_comparison_work.unwrap(),
            || {
                let mech_core::ResolvedOperationContract::Declared(contract) =
                    self.template.contract()
                else {
                    return Err(InvokeError::InvalidInput);
                };
                if selector.schema() != contract.inputs[1].schema
                    || selector.schema_key() != self.input_keys[1]
                    || !selector.shape().parameter_values().is_empty()
                {
                    return Err(InvokeError::InvalidInput);
                }
                selector
                    .validate_against(&self.schemas)
                    .map_err(|_| InvokeError::InvalidInput)?;
                let (_, component) = record_map(
                    source,
                    &self.schemas,
                    contract.inputs[0].schema,
                    ordinal,
                    self.template.output().body(),
                )?;
                let (source_bytes, source_nodes) = admitted(source, work)?;
                let (selector_bytes, selector_nodes) = admitted(selector, work)?;
                let context = NativeProjectionIndexContext::new(work, limits, coexistence)
                    .with_added_capacity(
                        source_bytes
                            .checked_add(selector_bytes)
                            .ok_or(InvokeError::InvalidInput)?,
                        source_nodes
                            .checked_add(selector_nodes)
                            .ok_or(InvokeError::InvalidInput)?,
                    )?;
                context.check_capacity(0, 0)?;
                self.check_buffer_record_ordinal(selector, ordinal, &context)?;
                let mut meter = ResidentBudgetMeter::for_native_projection(work, limits)?;
                let selector_footprint = measure_borrowed_canonical_value_footprint(
                    &mut meter,
                    selector,
                    &self.schemas,
                )?;
                let footprint = measure_borrowed_canonical_component_footprint(
                    &mut meter,
                    self.template.output().body(),
                    component,
                )?;
                let cloned = footprint
                    .retained_bytes
                    .checked_sub(core::mem::size_of::<Value>() as u64)
                    .and_then(|n| n.checked_add(selector_footprint.retained_bytes))
                    .ok_or(InvokeError::InvalidInput)?;
                if let Some(violation) = mech_core::first_call_memory_budget_violation(
                    mech_core::MemoryObjectOwner::NodeOutput {
                        node: self.template.origin().node(),
                        port: 0,
                    },
                    ResourceDemand {
                        output_elements: 1,
                        cloned_bytes: cloned,
                        ..ResourceDemand::default()
                    },
                    footprint.retained_bytes,
                    0,
                    limits,
                ) {
                    return Err(InvokeError::Plan(
                        mech_core::MemoryPlanError::TargetLimitExceeded { violation },
                    ));
                }
                Ok(())
            },
        )?
    }

    pub fn supports_borrowed_record_map_read(&self) -> bool {
        let [source, key] = self.template.inputs() else {
            return false;
        };
        matches!(
            self.template.primitive(),
            Primitive::MapKeyAccess | Primitive::MapContainsKey
        ) && string_u64(source.body())
            && key.body() == &SchemaBody::String
            && self
                .template
                .inputs()
                .iter()
                .all(|s| s.dimension_parameters().is_empty())
            && self.template.output().dimension_parameters().is_empty()
            && self.template.output().body()
                == match self.template.primitive() {
                    Primitive::MapKeyAccess => &SchemaBody::UnsignedInteger(IntegerWidth::W64),
                    Primitive::MapContainsKey => &SchemaBody::Bool,
                    _ => return false,
                }
    }

    /// Read ONE selected maintained Map operation through the admitted parent.
    /// Root/ordinal are descriptive; the genuine factory, current account and
    /// schema checks remain mandatory. No borrowed pointer leaves this method.
    /// Coexistence excludes source/key and already admitted scalar output.
    pub fn read_borrowed_record_map(
        &self,
        source: &Value,
        record_schema: SchemaId,
        ordinal: usize,
        key: &Value,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        coexistence: ResourceDemand,
    ) -> Result<u64, InvokeError> {
        finite_limits(self.template.origin().node(), limits).map_err(InvokeError::Binding)?;
        if !self.supports_borrowed_record_map_read() {
            return Err(InvokeError::InvalidInput);
        }
        work.with_call_limits(
            limits.max_compute_work.unwrap(),
            limits.max_comparison_work.unwrap(),
            || {
                let contains = self.template.primitive() == Primitive::MapContainsKey;
                let output_bytes = if contains { 1 } else { 8 };
                if let Some(violation) = mech_core::first_call_memory_budget_violation(
                    mech_core::MemoryObjectOwner::NodeOutput {
                        node: self.template.origin().node(),
                        port: 0,
                    },
                    ResourceDemand {
                        output_elements: 1,
                        ..ResourceDemand::default()
                    },
                    output_bytes,
                    0,
                    limits,
                ) {
                    return Err(InvokeError::Plan(
                        mech_core::MemoryPlanError::TargetLimitExceeded { violation },
                    ));
                }
                let context = NativeProjectionIndexContext::new(work, limits, coexistence);
                context.check_capacity(0, 0)?;
                context.charge(self.lookup_work, 0)?;
                let mech_core::ResolvedOperationContract::Declared(contract) =
                    self.template.contract()
                else {
                    return Err(InvokeError::InvalidInput);
                };
                if key.schema() != contract.inputs[1].schema
                    || key.schema_key() != self.input_keys[1]
                    || !key.shape().parameter_values().is_empty()
                {
                    return Err(InvokeError::InvalidInput);
                }
                key.validate_against(&self.schemas)
                    .map_err(|_| InvokeError::InvalidInput)?;
                let (map, _) = record_map(
                    source,
                    &self.schemas,
                    record_schema,
                    ordinal,
                    self.template.inputs()[0].body(),
                )?;
                let (source_bytes, source_nodes) = admitted(source, work)?;
                let (key_bytes, key_nodes) = admitted(key, work)?;
                let context = context.with_added_capacity(
                    source_bytes
                        .checked_add(key_bytes)
                        .and_then(|n| n.checked_add(output_bytes))
                        .ok_or(InvokeError::InvalidInput)?,
                    source_nodes
                        .checked_add(key_nodes)
                        .and_then(|n| n.checked_add(1))
                        .ok_or(InvokeError::InvalidInput)?,
                )?;
                context.check_capacity(0, 0)?;
                context.charge(128, 0)?;
                let ValueData::String(text) = key.data() else {
                    return Err(InvokeError::InvalidInput);
                };
                let mut budget = ReadBudget {
                    context: &context,
                    bytes: 0,
                    nodes: 0,
                };
                let candidate = string_map_access::candidate_work(text, &mut budget)?;
                let found = string_map_access::find_optional_with_compare(
                    map,
                    candidate,
                    &mut budget,
                    |value| {
                        let ValueData::String(candidate) = value else {
                            return Err(ResidentKernelError::InvalidInput);
                        };
                        Ok(text.as_ref().cmp(candidate.as_ref()))
                    },
                )?;
                if contains {
                    return Ok(u64::from(found.is_some()));
                }
                let at = found.ok_or(ResidentKernelError::InvalidInput)?;
                match map.entries().get(at).map(|entry| entry.value()) {
                    Some(ValueData::U64(value)) => Ok(*value),
                    _ => Err(InvokeError::InvalidInput),
                }
            },
        )?
    }
}

struct ReadBudget<'a, 'b> {
    context: &'a NativeProjectionIndexContext<'b>,
    bytes: u64,
    nodes: u64,
}
impl Budget for ReadBudget<'_, '_> {
    fn compute(&mut self, n: u64) -> Result<(), ResidentKernelError> {
        self.context.charge(n, 0)
    }
    fn comparison(&mut self, n: u64) -> Result<(), ResidentKernelError> {
        self.context.charge(n, n)
    }
    fn nodes(&mut self, n: u64) -> Result<(), ResidentKernelError> {
        self.nodes = self
            .nodes
            .checked_add(n)
            .ok_or(ResidentKernelError::InvalidShape)?;
        self.context.check_capacity(self.bytes, self.nodes)
    }
    fn selector_bytes(&mut self, n: u64) -> Result<(), ResidentKernelError> {
        self.bytes = self
            .bytes
            .checked_add(n)
            .ok_or(ResidentKernelError::InvalidShape)?;
        self.context.check_capacity(self.bytes, self.nodes)
    }
    fn admit(&mut self) -> Result<(), ResidentKernelError> {
        self.context.check_capacity(self.bytes, self.nodes)
    }
}

#[cfg(all(test, any(feature = "full_source", feature = "full_compiler")))]
#[path = "native_collection_borrowed_record_map_tests.rs"]
mod tests;
