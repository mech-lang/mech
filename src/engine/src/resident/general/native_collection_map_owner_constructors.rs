//! Selected constructor admission with one currently owned String/U64 Map.
//! Statistics describe a private owner; they grant no execution authority.
//! The native caller must validate its exact plan/site/token and same account.
use super::{InvokeError, NativePreparedLibraryCall, finite_limits};
use crate::native_artifact::{NativeCollectionPrimitive as Primitive, NativeLibraryWorkBudget};
use crate::resident::{
    budget::native_turn_plan_cache::NativeProjectionIndexContext,
    composite::{constructor_child_clone_bytes, constructor_output_envelope},
};
use mech_core::{IntegerWidth, MemoryBudgetLimits, ResourceDemand, SchemaBody, Value, ValueData};

pub(super) fn normalization(
    template: &super::NativeProgramLibraryTemplate,
    schemas: &mech_core::SchemaTable,
) -> Option<mech_core::snapshot::CompositeBindingCost> {
    if !matches!(template.primitive(), Primitive::CompositePack(_)) {
        return None;
    }
    let mech_core::ResolvedOperationContract::Declared(contract) = template.contract() else {
        return None;
    };
    mech_core::snapshot::CompositeSnapshotConstructor::binding_cost_for_inputs(
        contract.outputs.first()?.schema,
        contract.inputs.len(),
        |index| contract.inputs.get(index).map(|input| input.schema),
        schemas,
    )
}

fn add(a: u64, b: u64) -> Result<u64, InvokeError> {
    a.checked_add(b).ok_or(InvokeError::InvalidInput)
}
fn mul(a: u64, b: u64) -> Result<u64, InvokeError> {
    a.checked_mul(b).ok_or(InvokeError::InvalidInput)
}
fn map(body: &SchemaBody) -> bool {
    matches!(body, SchemaBody::Map { key, value, .. }
        if key.as_ref() == &SchemaBody::String
        && value.as_ref() == &SchemaBody::UnsignedInteger(IntegerWidth::W64))
}
fn data_bytes(body: &SchemaBody, map_data: u64, maps: &mut usize) -> Result<u64, InvokeError> {
    let data = core::mem::size_of::<ValueData>() as u64;
    match body {
        SchemaBody::UnsignedInteger(IntegerWidth::W64) => Ok(data),
        _ if map(body) => {
            *maps += 1;
            Ok(map_data)
        }
        SchemaBody::Tuple(items) if !items.is_empty() && items.len() <= 32 => {
            items.iter().try_fold(data, |n, item| {
                if !map(item) && *item != SchemaBody::UnsignedInteger(IntegerWidth::W64) {
                    return Err(InvokeError::InvalidInput);
                }
                add(n, data_bytes(item, map_data, maps)?)
            })
        }
        SchemaBody::Record(fields) if !fields.is_empty() && fields.len() <= 32 => {
            fields.iter().try_fold(data, |n, field| {
                if !map(&field.schema)
                    && field.schema != SchemaBody::UnsignedInteger(IntegerWidth::W64)
                {
                    return Err(InvokeError::InvalidInput);
                }
                add(n, data_bytes(&field.schema, map_data, maps)?)
            })
        }
        _ => Err(InvokeError::InvalidInput),
    }
}

impl NativePreparedLibraryCall {
    /// Check the maintained logical element/output/clone envelope before the
    /// selected constructor's native word stores. No Map traversal, canonical
    /// child, draft, output allocation, or library kernel is performed here.
    pub fn admit_map_owner_constructor(
        &self,
        map_entries: u64,
        map_key_bytes: u64,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        coexistence: ResourceDemand,
    ) -> Result<(), InvokeError> {
        finite_limits(self.template.origin().node(), limits).map_err(InvokeError::Binding)?;
        if !self.template.output().dimension_parameters().is_empty()
            || self
                .template
                .inputs()
                .iter()
                .any(|schema| !schema.dimension_parameters().is_empty())
        {
            return Err(InvokeError::InvalidInput);
        }
        work.with_call_limits(
            limits.max_compute_work.unwrap(),
            limits.max_comparison_work.unwrap(),
            || {
                let context = NativeProjectionIndexContext::new(work, limits, coexistence);
                context.charge(
                    (self.template.inputs().len() as u64)
                        .checked_mul(64)
                        .and_then(|n| n.checked_add(512))
                        .ok_or(InvokeError::InvalidInput)?,
                    0,
                )?;
                // Map retained_data_footprint is one root ValueData, one
                // MapEntryValue plus key/value ValueDatas per entry, and UTF8 bytes.
                let map_data = add(
                    add(
                        core::mem::size_of::<ValueData>() as u64,
                        mul(
                            map_entries,
                            (core::mem::size_of::<mech_core::snapshot::MapEntryValue>()
                                + 2 * core::mem::size_of::<ValueData>())
                                as u64,
                        )?,
                    )?,
                    map_key_bytes,
                )?;
                let wrapper = core::mem::size_of::<Value>() as u64;
                let slot = core::mem::size_of::<ValueData>();
                let root =
                    mech_core::snapshot::CompositeSnapshotConstructor::value_container_bytes(0)
                        .ok_or(InvokeError::InvalidInput)?;
                let mut maps = 0;
                let (elements, root, children) = match self.template.primitive() {
                    Primitive::CompositePack(arity)
                        if arity == self.template.inputs().len() && arity <= 32 =>
                    {
                        let fields = match self.template.output().body() {
                            SchemaBody::Tuple(items) => items.len(),
                            SchemaBody::Record(fields) => fields.len(),
                            _ => return Err(InvokeError::InvalidInput),
                        };
                        if fields != arity {
                            return Err(InvokeError::InvalidInput);
                        }
                        let mut children = 0usize;
                        for (ordinal, input) in self.template.inputs().iter().enumerate() {
                            let expected = match self.template.output().body() {
                                SchemaBody::Tuple(items) => &items[ordinal],
                                SchemaBody::Record(fields) => &fields[ordinal].schema,
                                _ => return Err(InvokeError::InvalidInput),
                            };
                            if input.body() != expected {
                                return Err(InvokeError::InvalidInput);
                            }
                            let footprint =
                                add(wrapper, data_bytes(input.body(), map_data, &mut maps)?)?;
                            children = children
                                .checked_add(
                                    constructor_child_clone_bytes(
                                        slot,
                                        usize::try_from(footprint)
                                            .map_err(|_| InvokeError::InvalidInput)?,
                                    )
                                    .ok_or(InvokeError::InvalidInput)?,
                                )
                                .ok_or(InvokeError::InvalidInput)?;
                        }
                        (arity as u64, root, children)
                    }
                    Primitive::EnumPack
                        if self.template.inputs().len() == 2
                            && self.template.inputs()[0].body() == &SchemaBody::Index =>
                    {
                        let payload = self.template.inputs()[1].body();
                        let SchemaBody::Enum { variants, .. } = self.template.output().body()
                        else {
                            return Err(InvokeError::InvalidInput);
                        };
                        if !variants
                            .iter()
                            .any(|variant| variant.payload.as_ref() == Some(payload))
                        {
                            return Err(InvokeError::InvalidInput);
                        }
                        let footprint = add(wrapper, data_bytes(payload, map_data, &mut maps)?)?;
                        let children = constructor_child_clone_bytes(
                            slot,
                            usize::try_from(footprint).map_err(|_| InvokeError::InvalidInput)?,
                        )
                        .ok_or(InvokeError::InvalidInput)?;
                        (
                            1,
                            root.checked_add(slot).ok_or(InvokeError::InvalidInput)?,
                            children,
                        )
                    }
                    _ => return Err(InvokeError::InvalidInput),
                };
                if maps != 1 {
                    return Err(InvokeError::InvalidInput);
                }
                let (output, cloned) =
                    constructor_output_envelope(root, children).ok_or(InvokeError::InvalidInput)?;
                let cloned = if matches!(self.template.primitive(), Primitive::CompositePack(_)) {
                    // NativeCollectionCall's maintained binding deliberately
                    // uses activation_fixed_shape=false. Its normalization
                    // witness is part of the original clone admission even
                    // though these private words require no cloned metadata.
                    let normalization = self
                        .constructor_normalization
                        .ok_or(InvokeError::InvalidInput)?;
                    add(cloned as u64, normalization.temporary_bytes)?
                } else {
                    cloned as u64
                };
                if let Some(violation) = mech_core::first_call_memory_budget_violation(
                    mech_core::MemoryObjectOwner::NodeOutput {
                        node: self.template.origin().node(),
                        port: 0,
                    },
                    ResourceDemand {
                        output_elements: elements,
                        cloned_bytes: cloned,
                        ..ResourceDemand::default()
                    },
                    output as u64,
                    0,
                    limits,
                ) {
                    return Err(InvokeError::Kernel(
                        crate::resident::budget::resource_budget_error(&violation),
                    ));
                }
                context.check_capacity(0, 0)?;
                Ok(())
            },
        )?
    }
}
