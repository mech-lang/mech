//! Checked layout and capacity derivation.

use crate::{
    CardinalitySpec, DimensionExpr, DimensionLifetime, DimensionParameterId, ExtentEvolution,
    FloatWidth, IntegerWidth, MemoryTopology, Schema, SchemaBody, ShapeInstance,
    check_schema_storage_compatibility,
};

#[cfg(all(feature = "no_std", feature = "functions"))]
use alloc::collections::BTreeMap;
#[cfg(feature = "no_std")]
use alloc::collections::BTreeSet;
#[cfg(feature = "no_std")]
use alloc::{vec, vec::Vec};
#[cfg(all(not(feature = "no_std"), feature = "functions"))]
use std::collections::BTreeMap;
#[cfg(not(feature = "no_std"))]
use std::{collections::BTreeSet, vec::Vec};

#[cfg(feature = "functions")]
use super::{
    AliasDecision, AllocationPlan, AllocationRole, ArenaPlacement, CallMemoryPlan,
    ImplementationMemoryClass, MemoryArenaId, MemoryLifetime, MemoryObjectId, MemoryObjectOwner,
    MemorySpace, PortMemoryPlan, RegionAccessPlan, ResourceDemand, TransactionRequirement,
    evaluate_memory_budget,
};
use super::{
    AxisCapacityPlan, CapacityAuthority, CapacityRequirement, CurrentMemoryFootprint,
    DimensionCapacity, GrowthPolicy, MemoryFootprintWitness, MemoryPlanError, MemoryTargetKind,
    PayloadCapacityPlan, PhysicalStorageDescriptor, PlannedSlotKind, SlotLayout,
    StorageLayoutClass, TargetMemoryProfile, ValueLayoutPlan,
};
#[cfg(feature = "functions")]
use crate::{
    AliasPolicy, BoundCall, ChangeDetectionPolicy, ExecutionTarget, OutputConstruction,
    PortDirection, PublicationRequirement, check_port_storage_compatibility,
};

#[cfg(feature = "functions")]
pub struct CallMemoryPlanningRequest<'a> {
    pub bound_call: &'a BoundCall,
    pub input_storage: &'a [PhysicalStorageDescriptor],
    pub output_storage: &'a [PhysicalStorageDescriptor],
    pub input_witnesses: &'a [MemoryFootprintWitness],
    pub output_witnesses: &'a [MemoryFootprintWitness],
    pub implementation_memory: ImplementationMemoryClass,
    pub target: &'a TargetMemoryProfile,
    pub regions: &'a [RegionAccessPlan],
}

#[cfg(feature = "functions")]
pub fn plan_call_memory(
    request: CallMemoryPlanningRequest<'_>,
) -> Result<CallMemoryPlan, MemoryPlanError> {
    request
        .bound_call
        .operation_descriptor()
        .validate()
        .map_err(|_| MemoryPlanError::DescriptorMismatch)?;
    validate_call_arities(&request)?;
    validate_call_target(request.bound_call, request.target)?;
    let requirements = request
        .bound_call
        .operation_descriptor()
        .contract
        .memory_requirements(request.bound_call.inputs().len())
        .map_err(|_| MemoryPlanError::DescriptorMismatch)?;

    let mut next_object = 0_u32;
    let mut inputs = Vec::with_capacity(request.bound_call.inputs().len());
    let mut outputs = Vec::with_capacity(request.bound_call.outputs().len());
    let mut allocations = Vec::new();
    let mut arena_offsets = BTreeMap::new();

    for (ordinal, (((descriptor, storage), witness), requirement)) in request
        .bound_call
        .inputs()
        .iter()
        .zip(request.input_storage)
        .zip(request.input_witnesses)
        .zip(requirements.inputs.iter())
        .enumerate()
    {
        check_port_storage_compatibility(
            descriptor.schema(),
            descriptor.shape(),
            requirement,
            &storage.capabilities,
        )
        .map_err(|_| MemoryPlanError::DescriptorMismatch)?;
        let value = plan_value_layout(ValueLayoutPlanningRequest {
            descriptor,
            storage,
            witness: *witness,
            target: request.target,
        })?;
        let owner = MemoryObjectOwner::DirectCallPort {
            call: 0,
            direction: PortDirection::Input,
            port: checked_u16(ordinal, "input port ordinal")?,
        };
        let object = MemoryObjectId::new(next_object);
        next_object = checked_next_object(next_object)?;
        push_value_allocations(
            &mut allocations,
            &mut arena_offsets,
            object,
            owner,
            storage,
            &value,
            &mut next_object,
        )?;
        inputs.push(PortMemoryPlan {
            descriptor: descriptor.clone(),
            value,
            region: RegionAccessPlan::WholeValue,
            object,
        });
    }

    for (ordinal, ((((descriptor, storage), witness), requirement), region)) in request
        .bound_call
        .outputs()
        .iter()
        .zip(request.output_storage)
        .zip(request.output_witnesses)
        .zip(requirements.outputs.iter())
        .zip(request.regions)
        .enumerate()
    {
        check_port_storage_compatibility(
            descriptor.schema(),
            descriptor.shape(),
            requirement,
            &storage.capabilities,
        )
        .map_err(|_| MemoryPlanError::DescriptorMismatch)?;
        let value = plan_value_layout(ValueLayoutPlanningRequest {
            descriptor,
            storage,
            witness: *witness,
            target: request.target,
        })?;
        let owner = MemoryObjectOwner::DirectCallPort {
            call: 0,
            direction: PortDirection::Output,
            port: checked_u16(ordinal, "output port ordinal")?,
        };
        let object = MemoryObjectId::new(next_object);
        next_object = checked_next_object(next_object)?;
        push_value_allocations(
            &mut allocations,
            &mut arena_offsets,
            object,
            owner,
            storage,
            &value,
            &mut next_object,
        )?;
        outputs.push(PortMemoryPlan {
            descriptor: descriptor.clone(),
            value,
            region: region.clone(),
            object,
        });
    }

    let aliases = derive_aliases(&request, &requirements, &inputs, &outputs)?;
    let (transactions, transaction_bytes) = derive_transactions(
        &request,
        &requirements,
        &inputs,
        &outputs,
        &mut allocations,
        &mut arena_offsets,
        &mut next_object,
    )?;
    let demand = derive_call_demand(
        &request,
        &requirements,
        &inputs,
        &outputs,
        transaction_bytes,
    )?;
    let output_bytes = outputs.iter().try_fold(0_u64, |total, output| {
        total
            .checked_add(value_required_bytes(&output.value)?)
            .ok_or(MemoryPlanError::ArithmeticOverflow {
                field: "call output bytes",
            })
    })?;
    let owner = outputs
        .first()
        .map(|output| {
            allocations
                .iter()
                .find(|allocation| allocation.id == output.object)
                .map(|allocation| allocation.owner.clone())
        })
        .flatten()
        .unwrap_or(MemoryObjectOwner::DirectCallPort {
            call: 0,
            direction: PortDirection::Output,
            port: 0,
        });
    let storage_buffer_bytes = if request.target.kind == MemoryTargetKind::Gpu {
        allocations
            .iter()
            .map(|allocation| allocation.capacity_bytes)
            .max()
            .unwrap_or(0)
    } else {
        0
    };
    if let Some(violation) = evaluate_memory_budget(
        owner,
        demand,
        output_bytes,
        storage_buffer_bytes,
        request.target.limits,
    )
    .first()
    .cloned()
    {
        return Err(MemoryPlanError::TargetLimitExceeded { violation });
    }
    allocations.sort_by_key(|allocation| allocation.id);
    Ok(CallMemoryPlan {
        bound_call: request.bound_call.clone(),
        inputs: inputs.into_boxed_slice(),
        outputs: outputs.into_boxed_slice(),
        allocations: allocations.into_boxed_slice(),
        aliases: aliases.into_boxed_slice(),
        transactions: transactions.into_boxed_slice(),
        implementation_memory: request.implementation_memory,
        demand,
        deferred_witnesses: request
            .input_witnesses
            .iter()
            .chain(request.output_witnesses)
            .filter_map(|witness| match witness {
                MemoryFootprintWitness::Known(_) => None,
                MemoryFootprintWitness::Deferred(stage) => Some(*stage),
            })
            .collect::<Vec<_>>()
            .into_boxed_slice(),
    })
}

pub struct ValueLayoutPlanningRequest<'a> {
    pub descriptor: &'a crate::ResolvedValueDescriptor,
    pub storage: &'a PhysicalStorageDescriptor,
    pub witness: MemoryFootprintWitness,
    pub target: &'a TargetMemoryProfile,
}

pub fn derive_dimension_capacity(
    schema: &Schema,
    shape: &ShapeInstance,
    expression: &DimensionExpr,
) -> Result<DimensionCapacity, MemoryPlanError> {
    schema
        .instantiate_shape(shape.parameter_values().to_vec().into_boxed_slice())
        .map_err(|_| MemoryPlanError::DescriptorMismatch)?;
    evaluate_dimension_capacity(schema, shape, expression, &mut BTreeSet::new())
}

pub fn plan_value_layout(
    request: ValueLayoutPlanningRequest<'_>,
) -> Result<ValueLayoutPlan, MemoryPlanError> {
    let descriptor = request.descriptor;
    check_schema_storage_compatibility(
        descriptor.schema(),
        descriptor.shape(),
        &request.storage.capabilities,
    )
    .map_err(|_| MemoryPlanError::DescriptorMismatch)?;
    let contract = descriptor
        .schema()
        .resolved_type_memory_contract(descriptor.shape())
        .map_err(|_| MemoryPlanError::DescriptorMismatch)?;
    let footprint = match request.witness {
        MemoryFootprintWitness::Known(footprint) => Some(footprint),
        MemoryFootprintWitness::Deferred(stage) => {
            if needs_footprint(&request.storage.slot, contract.topology) {
                return Err(MemoryPlanError::MissingFootprintWitness { stage });
            }
            None
        }
    };
    let slot = target_slot_layout(request.target, request.storage.slot)?;
    super::validate_alignment(slot.alignment)?;
    let storage = storage_layout(contract.topology, request.storage.slot)?;
    let axes = derive_axes(descriptor, footprint)?;
    let (current_elements, capacity_elements) =
        derive_element_capacity(descriptor, contract.topology, &axes, footprint)?;
    let payload = derive_payload_capacity(request.storage.slot, storage, footprint)?;
    let element_stride = align_up(slot.bytes, slot.alignment)?;
    let (strides_bytes, current_address_span_bytes, capacity_bytes) = match storage {
        StorageLayoutClass::DenseColumnMajor { .. } => {
            let [rows, columns] = axes.as_slice() else {
                return Err(MemoryPlanError::UnsupportedDenseRank {
                    rank: axes.len() as u64,
                });
            };
            let column_stride = rows.capacity.required.checked_mul(element_stride).ok_or(
                MemoryPlanError::ArithmeticOverflow {
                    field: "dense column stride",
                },
            )?;
            let current_span = if rows.current == 0 || columns.current == 0 {
                0
            } else {
                columns
                    .current
                    .checked_sub(1)
                    .and_then(|columns| columns.checked_mul(column_stride))
                    .and_then(|prefix| {
                        rows.current
                            .checked_mul(element_stride)
                            .and_then(|rows| prefix.checked_add(rows))
                    })
                    .ok_or(MemoryPlanError::ArithmeticOverflow {
                        field: "dense current address span",
                    })?
            };
            let capacity = rows
                .capacity
                .required
                .checked_mul(columns.capacity.required)
                .and_then(|elements| elements.checked_mul(element_stride))
                .ok_or(MemoryPlanError::ArithmeticOverflow {
                    field: "dense capacity bytes",
                })?;
            (
                vec![element_stride, column_stride].into_boxed_slice(),
                current_span,
                capacity,
            )
        }
        StorageLayoutClass::Scalar { .. } | StorageLayoutClass::CanonicalSnapshot { .. } => (
            Vec::new().into_boxed_slice(),
            element_stride,
            element_stride,
        ),
    };
    if request.target.kind == MemoryTargetKind::Gpu && capacity_bytes == 0 {
        return Err(MemoryPlanError::ZeroSizedGpuBinding);
    }
    let addressed = capacity_bytes.checked_add(payload.required_bytes).ok_or(
        MemoryPlanError::ArithmeticOverflow {
            field: "value capacity and payload bytes",
        },
    )?;
    if addressed > request.target.maximum_addressable_bytes {
        return Err(MemoryPlanError::TargetAddressOverflow);
    }
    Ok(ValueLayoutPlan {
        storage,
        axes: axes.into_boxed_slice(),
        current_elements,
        capacity_elements,
        slot,
        strides_bytes,
        current_address_span_bytes,
        capacity_bytes,
        payload,
    })
}

fn evaluate_dimension_capacity(
    schema: &Schema,
    shape: &ShapeInstance,
    expression: &DimensionExpr,
    visiting: &mut BTreeSet<DimensionParameterId>,
) -> Result<DimensionCapacity, MemoryPlanError> {
    let current = shape
        .resolve_dimension(expression)
        .map_err(|_| MemoryPlanError::DescriptorMismatch)?;
    let (maximum, evolution) = match expression {
        DimensionExpr::Hole => return Err(MemoryPlanError::DescriptorMismatch),
        DimensionExpr::Constant(value) => (Some(*value), ExtentEvolution::Fixed),
        DimensionExpr::Parameter(id) => {
            let parameter = schema
                .dimension_parameters()
                .get(id.get() as usize)
                .ok_or(MemoryPlanError::DescriptorMismatch)?;
            match parameter.lifetime() {
                DimensionLifetime::CompileTime => {
                    return Err(MemoryPlanError::DescriptorMismatch);
                }
                DimensionLifetime::Activation => (Some(current), ExtentEvolution::ActivationFixed),
                DimensionLifetime::Turn => {
                    let Some(bound) = parameter.upper_bound() else {
                        return Ok(DimensionCapacity {
                            current,
                            maximum: None,
                            evolution: ExtentEvolution::TurnUnbounded,
                        });
                    };
                    if !visiting.insert(*id) {
                        return Err(MemoryPlanError::CyclicDimensionUpperBound);
                    }
                    let result = evaluate_dimension_capacity(schema, shape, bound, visiting)?;
                    visiting.remove(id);
                    (
                        result.maximum,
                        if result.maximum.is_some() {
                            ExtentEvolution::TurnBounded
                        } else {
                            ExtentEvolution::TurnUnbounded
                        },
                    )
                }
            }
        }
        DimensionExpr::Add(operands) => (
            combine_all_maxima(schema, shape, operands, visiting, 0, u64::checked_add)?,
            compound_evolution(schema, shape, operands, visiting)?,
        ),
        DimensionExpr::Multiply(operands) => (
            combine_all_maxima(schema, shape, operands, visiting, 1, u64::checked_mul)?,
            compound_evolution(schema, shape, operands, visiting)?,
        ),
        DimensionExpr::Min(operands) => {
            let capacities = evaluate_operands(schema, shape, operands, visiting)?;
            (
                capacities.iter().filter_map(|value| value.maximum).min(),
                joined_evolution(&capacities),
            )
        }
        DimensionExpr::Max(operands) => {
            let capacities = evaluate_operands(schema, shape, operands, visiting)?;
            let maximum = capacities
                .iter()
                .map(|value| value.maximum)
                .collect::<Option<Vec<_>>>()
                .and_then(|values| values.into_iter().max());
            (maximum, joined_evolution(&capacities))
        }
    };
    if let Some(maximum) = maximum
        && maximum < current
    {
        return Err(MemoryPlanError::CapacityBelowCurrent { current, maximum });
    }
    Ok(DimensionCapacity {
        current,
        maximum,
        evolution,
    })
}

fn evaluate_operands(
    schema: &Schema,
    shape: &ShapeInstance,
    operands: &[DimensionExpr],
    visiting: &mut BTreeSet<DimensionParameterId>,
) -> Result<Vec<DimensionCapacity>, MemoryPlanError> {
    operands
        .iter()
        .map(|operand| evaluate_dimension_capacity(schema, shape, operand, visiting))
        .collect()
}

fn combine_all_maxima(
    schema: &Schema,
    shape: &ShapeInstance,
    operands: &[DimensionExpr],
    visiting: &mut BTreeSet<DimensionParameterId>,
    identity: u64,
    operation: fn(u64, u64) -> Option<u64>,
) -> Result<Option<u64>, MemoryPlanError> {
    let mut result = identity;
    for operand in operands {
        let Some(maximum) = evaluate_dimension_capacity(schema, shape, operand, visiting)?.maximum
        else {
            return Ok(None);
        };
        result = operation(result, maximum).ok_or(MemoryPlanError::ArithmeticOverflow {
            field: "dimension upper bound",
        })?;
    }
    Ok(Some(result))
}

fn compound_evolution(
    schema: &Schema,
    shape: &ShapeInstance,
    operands: &[DimensionExpr],
    visiting: &mut BTreeSet<DimensionParameterId>,
) -> Result<ExtentEvolution, MemoryPlanError> {
    Ok(joined_evolution(&evaluate_operands(
        schema, shape, operands, visiting,
    )?))
}

fn joined_evolution(capacities: &[DimensionCapacity]) -> ExtentEvolution {
    capacities
        .iter()
        .fold(ExtentEvolution::Fixed, |left, right| {
            join_evolution(left, right.evolution)
        })
}

fn join_evolution(left: ExtentEvolution, right: ExtentEvolution) -> ExtentEvolution {
    use ExtentEvolution::{ActivationFixed, Fixed, TurnBounded, TurnUnbounded};
    match (left, right) {
        (TurnUnbounded, _) | (_, TurnUnbounded) => TurnUnbounded,
        (TurnBounded, _) | (_, TurnBounded) => TurnBounded,
        (ActivationFixed, _) | (_, ActivationFixed) => ActivationFixed,
        (Fixed, Fixed) => Fixed,
    }
}

fn derive_axes(
    descriptor: &crate::ResolvedValueDescriptor,
    footprint: Option<CurrentMemoryFootprint>,
) -> Result<Vec<AxisCapacityPlan>, MemoryPlanError> {
    let schema = descriptor.schema();
    let shape = descriptor.shape();
    let expressions = match schema.body() {
        SchemaBody::Matrix { dimensions, .. } => {
            return dimensions
                .iter()
                .map(|expression| derive_axis(schema, shape, expression))
                .collect();
        }
        SchemaBody::Table { rows, .. }
        | SchemaBody::Set {
            cardinality: rows, ..
        }
        | SchemaBody::Map {
            cardinality: rows, ..
        } => rows,
        _ => return Ok(Vec::new()),
    };
    match expressions {
        CardinalitySpec::Exact(expression) => Ok(vec![derive_axis(schema, shape, expression)?]),
        CardinalitySpec::Dynamic { upper_bound } => {
            let current = footprint
                .ok_or(MemoryPlanError::MissingFootprintWitness {
                    stage: super::MemoryWitnessStage::Activation,
                })?
                .logical_elements;
            let maximum = upper_bound
                .as_ref()
                .map(|bound| derive_dimension_capacity(schema, shape, bound))
                .transpose()?
                .and_then(|bound| bound.maximum);
            if let Some(maximum) = maximum
                && current > maximum
            {
                return Err(MemoryPlanError::DynamicCardinalityExceedsBound { current, maximum });
            }
            let evolution = if maximum.is_some() {
                ExtentEvolution::TurnBounded
            } else {
                ExtentEvolution::TurnUnbounded
            };
            Ok(vec![AxisCapacityPlan {
                current,
                capacity: capacity_for(current, maximum, evolution),
                evolution,
            }])
        }
    }
}

fn derive_axis(
    schema: &Schema,
    shape: &ShapeInstance,
    expression: &DimensionExpr,
) -> Result<AxisCapacityPlan, MemoryPlanError> {
    let dimension = derive_dimension_capacity(schema, shape, expression)?;
    Ok(AxisCapacityPlan {
        current: dimension.current,
        capacity: capacity_for(dimension.current, dimension.maximum, dimension.evolution),
        evolution: dimension.evolution,
    })
}

fn capacity_for(
    current: u64,
    maximum: Option<u64>,
    evolution: ExtentEvolution,
) -> CapacityRequirement {
    match evolution {
        ExtentEvolution::Fixed => CapacityRequirement {
            current,
            required: current,
            maximum: Some(current),
            authority: CapacityAuthority::ExactSemantic,
            growth: GrowthPolicy::Fixed,
        },
        ExtentEvolution::ActivationFixed => CapacityRequirement {
            current,
            required: current,
            maximum: Some(current),
            authority: CapacityAuthority::ActivationSemantic,
            growth: GrowthPolicy::Fixed,
        },
        ExtentEvolution::TurnBounded => CapacityRequirement {
            current,
            required: maximum.unwrap_or(current),
            maximum,
            authority: CapacityAuthority::SemanticUpperBound,
            growth: GrowthPolicy::ReservedToBound,
        },
        ExtentEvolution::TurnUnbounded => CapacityRequirement {
            current,
            required: current,
            maximum: None,
            authority: CapacityAuthority::CurrentValueWitness,
            growth: GrowthPolicy::ReplanBeforeGrowth,
        },
    }
}

fn derive_element_capacity(
    _descriptor: &crate::ResolvedValueDescriptor,
    topology: MemoryTopology,
    axes: &[AxisCapacityPlan],
    footprint: Option<CurrentMemoryFootprint>,
) -> Result<(u64, CapacityRequirement), MemoryPlanError> {
    if !axes.is_empty() {
        let current = checked_axis_product(axes.iter().map(|axis| axis.current))?;
        let required = checked_axis_product(axes.iter().map(|axis| axis.capacity.required))?;
        let maximum = axes
            .iter()
            .map(|axis| axis.capacity.maximum)
            .collect::<Option<Vec<_>>>()
            .map(|values| checked_axis_product(values.into_iter()))
            .transpose()?;
        let evolution = axes.iter().fold(ExtentEvolution::Fixed, |combined, axis| {
            join_evolution(combined, axis.evolution)
        });
        let mut capacity = capacity_for(current, maximum, evolution);
        capacity.required = required;
        return Ok((current, capacity));
    }
    let current = if matches!(
        topology,
        MemoryTopology::Tagged { .. }
            | MemoryTopology::Product { .. }
            | MemoryTopology::Columnar { .. }
            | MemoryTopology::OrderedSet
            | MemoryTopology::OrderedMap
    ) {
        footprint.map_or(1, |footprint| footprint.logical_elements.max(1))
    } else {
        1
    };
    Ok((
        current,
        CapacityRequirement {
            current,
            required: current,
            maximum: Some(current),
            authority: CapacityAuthority::ExactSemantic,
            growth: GrowthPolicy::Fixed,
        },
    ))
}

fn checked_axis_product(values: impl IntoIterator<Item = u64>) -> Result<u64, MemoryPlanError> {
    values.into_iter().try_fold(1_u64, |product, value| {
        product
            .checked_mul(value)
            .ok_or(MemoryPlanError::ArithmeticOverflow {
                field: "capacity elements",
            })
    })
}

fn needs_footprint(slot: &PlannedSlotKind, topology: MemoryTopology) -> bool {
    matches!(
        slot,
        PlannedSlotKind::StringHeader | PlannedSlotKind::CanonicalValueHandle
    ) || !matches!(topology, MemoryTopology::Scalar(_))
}

fn storage_layout(
    topology: MemoryTopology,
    slot: PlannedSlotKind,
) -> Result<StorageLayoutClass, MemoryPlanError> {
    Ok(match topology {
        MemoryTopology::Scalar(_) => StorageLayoutClass::Scalar { slot },
        MemoryTopology::DenseSequence { rank: 2 } => StorageLayoutClass::DenseColumnMajor { slot },
        MemoryTopology::DenseSequence { rank } => {
            return Err(MemoryPlanError::UnsupportedDenseRank { rank });
        }
        topology => StorageLayoutClass::CanonicalSnapshot { topology },
    })
}

fn derive_payload_capacity(
    slot: PlannedSlotKind,
    storage: StorageLayoutClass,
    footprint: Option<CurrentMemoryFootprint>,
) -> Result<PayloadCapacityPlan, MemoryPlanError> {
    let variable = matches!(
        slot,
        PlannedSlotKind::StringHeader | PlannedSlotKind::CanonicalValueHandle
    ) || matches!(storage, StorageLayoutClass::CanonicalSnapshot { .. });
    if !variable {
        return Ok(PayloadCapacityPlan {
            current_bytes: 0,
            required_bytes: 0,
            maximum_bytes: Some(0),
            current_nodes: 0,
            required_nodes: 0,
            maximum_nodes: Some(0),
            authority: CapacityAuthority::ExactSemantic,
            growth: GrowthPolicy::Fixed,
        });
    }
    let footprint = footprint.ok_or(MemoryPlanError::MissingFootprintWitness {
        stage: super::MemoryWitnessStage::Activation,
    })?;
    Ok(PayloadCapacityPlan {
        current_bytes: footprint.payload_bytes,
        required_bytes: footprint.payload_bytes,
        maximum_bytes: None,
        current_nodes: footprint.retained_nodes,
        required_nodes: footprint.retained_nodes,
        maximum_nodes: None,
        authority: CapacityAuthority::CurrentValueWitness,
        growth: GrowthPolicy::ReplanBeforeGrowth,
    })
}

fn target_slot_layout(
    target: &TargetMemoryProfile,
    slot: PlannedSlotKind,
) -> Result<SlotLayout, MemoryPlanError> {
    use crate::ScalarMemoryKind::{
        Atom, Bool, Complex, Floating, Id, Index, Rational64, Signed, String as StringKind,
        Unsigned,
    };
    let layouts = &target.primitives;
    if target.kind == MemoryTargetKind::Gpu
        && !matches!(
            slot,
            PlannedSlotKind::FixedScalar(Floating(FloatWidth::W32))
                | PlannedSlotKind::FixedScalar(Unsigned(IntegerWidth::W32))
        )
    {
        return Err(MemoryPlanError::UnsupportedStorageLayout);
    }
    if target.kind == MemoryTargetKind::ResidentCpu
        && !matches!(
            slot,
            PlannedSlotKind::FixedScalar(Bool)
                | PlannedSlotKind::FixedScalar(Index)
                | PlannedSlotKind::FixedScalar(Floating(FloatWidth::W64))
                | PlannedSlotKind::StringHeader
                | PlannedSlotKind::CanonicalValueHandle
        )
    {
        return Err(MemoryPlanError::UnsupportedStorageLayout);
    }
    match slot {
        PlannedSlotKind::StringHeader => Ok(layouts.string_header),
        PlannedSlotKind::CanonicalValueHandle => Ok(layouts.canonical_value_handle),
        PlannedSlotKind::FixedScalar(kind) => Ok(match kind {
            Bool => layouts.bool_slot,
            Unsigned(IntegerWidth::W8) => layouts.u8_slot,
            Unsigned(IntegerWidth::W16) => layouts.u16_slot,
            Unsigned(IntegerWidth::W32) => layouts.u32_slot,
            Unsigned(IntegerWidth::W64) => layouts.u64_slot,
            Unsigned(IntegerWidth::W128) => layouts.u128_slot,
            Signed(IntegerWidth::W8) => layouts.i8_slot,
            Signed(IntegerWidth::W16) => layouts.i16_slot,
            Signed(IntegerWidth::W32) => layouts.i32_slot,
            Signed(IntegerWidth::W64) => layouts.i64_slot,
            Signed(IntegerWidth::W128) => layouts.i128_slot,
            Floating(FloatWidth::W32) => layouts.f32_slot,
            Floating(FloatWidth::W64) => layouts.f64_slot,
            Complex(FloatWidth::W32) => layouts.c64_slot,
            Complex(FloatWidth::W64) => {
                return Err(MemoryPlanError::UnsupportedStorageLayout);
            }
            Rational64 => layouts.r64_slot,
            StringKind => layouts.string_header,
            Id => layouts.id_slot,
            Index => layouts.index_slot,
            Atom => layouts.atom_slot,
        }),
    }
}

fn align_up(bytes: u64, alignment: u32) -> Result<u64, MemoryPlanError> {
    super::validate_alignment(alignment)?;
    let mask = u64::from(alignment) - 1;
    bytes
        .checked_add(mask)
        .map(|value| value & !mask)
        .ok_or(MemoryPlanError::ArithmeticOverflow {
            field: "aligned slot bytes",
        })
}

#[cfg(feature = "functions")]
fn validate_call_arities(request: &CallMemoryPlanningRequest<'_>) -> Result<(), MemoryPlanError> {
    if request.bound_call.inputs().len() != request.input_storage.len()
        || request.bound_call.inputs().len() != request.input_witnesses.len()
        || request.bound_call.outputs().len() != request.output_storage.len()
        || request.bound_call.outputs().len() != request.output_witnesses.len()
        || request.bound_call.outputs().len() != request.regions.len()
    {
        return Err(MemoryPlanError::DescriptorArityMismatch);
    }
    Ok(())
}

#[cfg(feature = "functions")]
fn validate_call_target(
    call: &BoundCall,
    target: &TargetMemoryProfile,
) -> Result<(), MemoryPlanError> {
    let compatible = match call.target() {
        ExecutionTarget::DirectRuntime => matches!(
            target.kind,
            MemoryTargetKind::DirectHost | MemoryTargetKind::WasmHost
        ),
        ExecutionTarget::ResidentCpu => target.kind == MemoryTargetKind::ResidentCpu,
        ExecutionTarget::Native => target.kind == MemoryTargetKind::NativeHost,
        ExecutionTarget::GpuBatch => target.kind == MemoryTargetKind::Gpu,
    };
    compatible
        .then_some(())
        .ok_or(MemoryPlanError::DescriptorMismatch)
}

#[cfg(feature = "functions")]
fn checked_u16(value: usize, field: &'static str) -> Result<u16, MemoryPlanError> {
    u16::try_from(value).map_err(|_| MemoryPlanError::ArithmeticOverflow { field })
}

#[cfg(feature = "functions")]
fn checked_next_object(current: u32) -> Result<u32, MemoryPlanError> {
    current
        .checked_add(1)
        .ok_or(MemoryPlanError::ArithmeticOverflow {
            field: "memory object identity",
        })
}

#[cfg(feature = "functions")]
fn arena_for_space(space: MemorySpace) -> MemoryArenaId {
    MemoryArenaId::new(match space {
        MemorySpace::Host => 0,
        MemorySpace::ResidentCpu => 1,
        MemorySpace::Device { region } => region.saturating_add(2),
    })
}

#[cfg(feature = "functions")]
fn allocate_offset(
    offsets: &mut BTreeMap<MemoryArenaId, u64>,
    arena: MemoryArenaId,
    bytes: u64,
    alignment: u32,
) -> Result<ArenaPlacement, MemoryPlanError> {
    let start = align_up(offsets.get(&arena).copied().unwrap_or(0), alignment)?;
    let end = start
        .checked_add(bytes)
        .ok_or(MemoryPlanError::ArithmeticOverflow {
            field: "arena placement",
        })?;
    offsets.insert(arena, end);
    Ok(ArenaPlacement {
        arena,
        offset: start,
    })
}

#[cfg(feature = "functions")]
fn push_value_allocations(
    allocations: &mut Vec<AllocationPlan>,
    offsets: &mut BTreeMap<MemoryArenaId, u64>,
    object: MemoryObjectId,
    owner: MemoryObjectOwner,
    storage: &PhysicalStorageDescriptor,
    value: &ValueLayoutPlan,
    next_object: &mut u32,
) -> Result<(), MemoryPlanError> {
    let arena = arena_for_space(storage.space);
    allocations.push(AllocationPlan {
        id: object,
        owner: owner.clone(),
        role: AllocationRole::FixedStorage,
        space: storage.space,
        current_bytes: value.current_address_span_bytes,
        capacity_bytes: value.capacity_bytes,
        alignment: value.slot.alignment,
        lifetime: storage.lifetime,
        placement: allocate_offset(offsets, arena, value.capacity_bytes, value.slot.alignment)?,
        reuse_group: None,
    });
    if value.payload.required_bytes != 0 || value.payload.maximum_bytes.is_none() {
        let payload = MemoryObjectId::new(*next_object);
        *next_object = checked_next_object(*next_object)?;
        allocations.push(AllocationPlan {
            id: payload,
            owner,
            role: AllocationRole::VariablePayload,
            space: storage.space,
            current_bytes: value.payload.current_bytes,
            capacity_bytes: value.payload.required_bytes,
            alignment: 1,
            lifetime: storage.lifetime,
            placement: allocate_offset(offsets, arena, value.payload.required_bytes, 1)?,
            reuse_group: None,
        });
    }
    Ok(())
}

#[cfg(feature = "functions")]
fn derive_aliases(
    request: &CallMemoryPlanningRequest<'_>,
    requirements: &crate::OperationMemoryRequirements,
    inputs: &[PortMemoryPlan],
    outputs: &[PortMemoryPlan],
) -> Result<Vec<AliasDecision>, MemoryPlanError> {
    requirements
        .outputs
        .iter()
        .zip(outputs)
        .zip(request.output_storage)
        .map(
            |((requirement, output), output_storage)| match requirement.alias {
                Some(AliasPolicy::NoAlias) | None => {
                    if requirement.publication == PublicationRequirement::AtomicReplace {
                        Ok(AliasDecision::StageThenPublish { input: None })
                    } else {
                        Ok(AliasDecision::Disjoint)
                    }
                }
                Some(AliasPolicy::MayAlias { input }) => {
                    let index = input as usize;
                    let compatible = inputs.get(index).zip(request.input_storage.get(index));
                    if compatible.is_some_and(|(candidate, storage)| {
                        candidate.descriptor == output.descriptor
                            && candidate.value.storage == output.value.storage
                            && candidate.value.slot == output.value.slot
                            && candidate.value.capacity_bytes >= output.value.capacity_bytes
                            && storage.space == output_storage.space
                            && storage.reusable_turn_temporary
                            && matches!(storage.lifetime, MemoryLifetime::Turn { .. })
                    }) {
                        Ok(AliasDecision::ReuseInput { input })
                    } else {
                        Ok(AliasDecision::StageThenPublish { input: Some(input) })
                    }
                }
                Some(AliasPolicy::InPlaceRequired { input }) => {
                    let index = input as usize;
                    let Some((candidate, storage)) =
                        inputs.get(index).zip(request.input_storage.get(index))
                    else {
                        return Err(MemoryPlanError::RequiredInPlaceAliasUnavailable { input });
                    };
                    if candidate.descriptor != output.descriptor
                        || candidate.value.storage != output.value.storage
                        || candidate.value.slot != output.value.slot
                        || candidate.value.capacity_bytes < output.value.capacity_bytes
                        || storage.space != output_storage.space
                        || !storage.capabilities.access.writable
                    {
                        return Err(MemoryPlanError::IncompatibleAlias {
                            input,
                            reason: "semantic descriptor or physical storage is incompatible"
                                .into(),
                        });
                    }
                    Ok(AliasDecision::InPlaceRequired { input })
                }
            },
        )
        .collect()
}

#[cfg(feature = "functions")]
fn derive_transactions(
    request: &CallMemoryPlanningRequest<'_>,
    requirements: &crate::OperationMemoryRequirements,
    inputs: &[PortMemoryPlan],
    outputs: &[PortMemoryPlan],
    allocations: &mut Vec<AllocationPlan>,
    offsets: &mut BTreeMap<MemoryArenaId, u64>,
    next_object: &mut u32,
) -> Result<(Vec<TransactionRequirement>, u64), MemoryPlanError> {
    let mut transactions = Vec::with_capacity(outputs.len());
    let mut bytes = 0_u64;
    for (ordinal, ((output, storage), requirement)) in outputs
        .iter()
        .zip(request.output_storage)
        .zip(requirements.outputs.iter())
        .enumerate()
    {
        if requirement.construction.is_none() {
            transactions.push(TransactionRequirement::None);
            continue;
        }
        let staged = MemoryObjectId::new(*next_object);
        *next_object = checked_next_object(*next_object)?;
        let staged_bytes = value_required_bytes(&output.value)?;
        bytes = bytes
            .checked_add(staged_bytes)
            .ok_or(MemoryPlanError::ArithmeticOverflow {
                field: "transaction bytes",
            })?;
        let arena = arena_for_space(storage.space);
        allocations.push(AllocationPlan {
            id: staged,
            owner: MemoryObjectOwner::DirectCallPort {
                call: 0,
                direction: PortDirection::Output,
                port: checked_u16(ordinal, "transaction output ordinal")?,
            },
            role: AllocationRole::TransactionStage,
            space: storage.space,
            current_bytes: staged_bytes,
            capacity_bytes: staged_bytes,
            alignment: output.value.slot.alignment,
            lifetime: MemoryLifetime::Transaction {
                first: super::MemoryPlanPoint::new(0),
                last: super::MemoryPlanPoint::new(0),
            },
            placement: allocate_offset(offsets, arena, staged_bytes, output.value.slot.alignment)?,
            reuse_group: None,
        });
        let transaction = match requirement.alias {
            Some(AliasPolicy::InPlaceRequired { input }) => {
                let target = inputs
                    .get(input as usize)
                    .map(|input| input.object)
                    .ok_or(MemoryPlanError::RequiredInPlaceAliasUnavailable { input })?;
                TransactionRequirement::UndoSnapshot {
                    target,
                    undo: staged,
                }
            }
            _ if matches!(
                request.target.kind,
                MemoryTargetKind::ResidentCpu | MemoryTargetKind::Gpu
            ) && storage.lifetime == MemoryLifetime::Activation =>
            {
                TransactionRequirement::DoubleBuffer {
                    current: output.object,
                    next: staged,
                }
            }
            _ => TransactionRequirement::StageAndSwap {
                current: output.object,
                staged,
            },
        };
        transactions.push(transaction);
    }
    Ok((transactions, bytes))
}

#[cfg(feature = "functions")]
fn derive_call_demand(
    request: &CallMemoryPlanningRequest<'_>,
    requirements: &crate::OperationMemoryRequirements,
    inputs: &[PortMemoryPlan],
    outputs: &[PortMemoryPlan],
    transaction_bytes: u64,
) -> Result<ResourceDemand, MemoryPlanError> {
    let mut demand = ResourceDemand {
        transaction_peak_bytes: transaction_bytes,
        turn_peak_bytes: transaction_bytes,
        storage_bindings: if request.target.kind == MemoryTargetKind::Gpu {
            u32::try_from(inputs.len().saturating_add(outputs.len())).map_err(|_| {
                MemoryPlanError::ArithmeticOverflow {
                    field: "storage bindings",
                }
            })?
        } else {
            0
        },
        ..ResourceDemand::default()
    };
    for (port, storage) in inputs
        .iter()
        .zip(request.input_storage)
        .chain(outputs.iter().zip(request.output_storage))
    {
        let bytes = value_current_bytes(&port.value)?;
        match storage.lifetime {
            MemoryLifetime::Program => {
                demand.persistent_bytes =
                    checked_add(demand.persistent_bytes, bytes, "persistent call bytes")?
            }
            MemoryLifetime::Activation => {
                demand.activation_bytes =
                    checked_add(demand.activation_bytes, bytes, "activation call bytes")?
            }
            MemoryLifetime::Turn { .. }
            | MemoryLifetime::Transaction { .. }
            | MemoryLifetime::Transfer { .. } => {
                demand.turn_peak_bytes =
                    checked_add(demand.turn_peak_bytes, bytes, "turn call bytes")?
            }
        }
    }
    for (ordinal, (output, requirement)) in outputs.iter().zip(&requirements.outputs).enumerate() {
        demand.output_elements = checked_add(
            demand.output_elements,
            output.value.current_elements,
            "output elements",
        )?;
        if let Some(OutputConstruction::ReadModifyWrite { base_input, .. }) =
            requirement.construction.as_ref()
        {
            let input = inputs
                .get(*base_input as usize)
                .ok_or(MemoryPlanError::DescriptorArityMismatch)?;
            let cloned = value_current_bytes(&input.value)?;
            demand.cloned_bytes = checked_add(demand.cloned_bytes, cloned, "rmw clone bytes")?;
            demand.turn_peak_bytes =
                checked_add(demand.turn_peak_bytes, cloned, "rmw temporary bytes")?;
        }
        match requirement.change_detection {
            None
            | Some(ChangeDetectionPolicy::KernelReported)
            | Some(ChangeDetectionPolicy::AlwaysChanged) => {}
            Some(ChangeDetectionPolicy::ExactScalar) => {
                demand.work.comparison =
                    checked_add(demand.work.comparison, 1, "exact scalar comparison")?;
            }
            Some(ChangeDetectionPolicy::SemanticHash) => {
                let footprint = known_footprint(request.output_witnesses[ordinal])?;
                let one_side = checked_add(
                    checked_add(
                        footprint.schema_bytes,
                        footprint.encoded_bytes,
                        "semantic hash bytes",
                    )?,
                    footprint.shape_parameter_count.checked_mul(8).ok_or(
                        MemoryPlanError::ArithmeticOverflow {
                            field: "semantic hash shape bytes",
                        },
                    )?,
                    "semantic hash shape work",
                )?;
                demand.work.comparison = checked_add(
                    demand.work.comparison,
                    one_side
                        .checked_mul(2)
                        .ok_or(MemoryPlanError::ArithmeticOverflow {
                            field: "semantic hash current and candidate",
                        })?,
                    "semantic hash comparison work",
                )?;
            }
        }
    }
    apply_implementation_demand(request, inputs, outputs, &mut demand)?;
    Ok(demand)
}

#[cfg(feature = "functions")]
fn apply_implementation_demand(
    request: &CallMemoryPlanningRequest<'_>,
    inputs: &[PortMemoryPlan],
    outputs: &[PortMemoryPlan],
    demand: &mut ResourceDemand,
) -> Result<(), MemoryPlanError> {
    match request.implementation_memory {
        ImplementationMemoryClass::NoAdditionalScratch => {}
        ImplementationMemoryClass::CloneInput { input } => {
            let input = inputs
                .get(input as usize)
                .ok_or(MemoryPlanError::DescriptorArityMismatch)?;
            let bytes = value_current_bytes(&input.value)?;
            demand.cloned_bytes = checked_add(demand.cloned_bytes, bytes, "input clone bytes")?;
            demand.turn_peak_bytes =
                checked_add(demand.turn_peak_bytes, bytes, "input clone temporary bytes")?;
        }
        ImplementationMemoryClass::MatrixSolve => {
            let [coefficients, rhs] = inputs else {
                return Err(MemoryPlanError::MatrixSolveLayoutInvalid);
            };
            let [rows, columns] = coefficients.value.axes.as_ref() else {
                return Err(MemoryPlanError::MatrixSolveLayoutInvalid);
            };
            if rows.current != columns.current {
                return Err(MemoryPlanError::MatrixSolveLayoutInvalid);
            }
            let rhs_columns = rhs.value.axes.get(1).map_or(1, |axis| axis.current);
            let coefficient_bytes = value_current_bytes(&coefficients.value)?;
            let solution_bytes = outputs
                .first()
                .map(|output| value_required_bytes(&output.value))
                .transpose()?
                .unwrap_or(0);
            let pivot_stride = target_slot_layout(
                request.target,
                PlannedSlotKind::FixedScalar(crate::ScalarMemoryKind::Index),
            )?
            .bytes;
            let pivot_bytes = rows.current.checked_mul(pivot_stride).ok_or(
                MemoryPlanError::ArithmeticOverflow {
                    field: "matrix solve pivot bytes",
                },
            )?;
            demand.cloned_bytes = checked_add(
                demand.cloned_bytes,
                coefficient_bytes,
                "matrix solve coefficient clone",
            )?;
            demand.turn_peak_bytes = checked_add(
                demand.turn_peak_bytes,
                checked_add(
                    coefficient_bytes,
                    checked_add(solution_bytes, pivot_bytes, "matrix solve stage and pivot")?,
                    "matrix solve scratch",
                )?,
                "matrix solve turn peak",
            )?;
            let square = rows.current.checked_mul(rows.current).ok_or(
                MemoryPlanError::ArithmeticOverflow {
                    field: "matrix solve square work",
                },
            )?;
            let work = square
                .checked_mul(rows.current)
                .and_then(|cube| {
                    square
                        .checked_mul(rhs_columns)
                        .and_then(|rhs| cube.checked_add(rhs))
                })
                .ok_or(MemoryPlanError::ArithmeticOverflow {
                    field: "matrix solve compute work",
                })?;
            demand.work.compute =
                checked_add(demand.work.compute, work, "matrix solve compute work")?;
        }
        ImplementationMemoryClass::CanonicalFinalize
        | ImplementationMemoryClass::CanonicalSortUnique => {
            let mut encoded = 0_u64;
            let mut nodes = 0_u64;
            for witness in request.output_witnesses {
                let footprint = known_footprint(*witness)?;
                encoded = checked_add(encoded, footprint.encoded_bytes, "canonical encoded bytes")?;
                nodes = checked_add(nodes, footprint.retained_nodes, "canonical retained nodes")?;
            }
            let traversal = encoded.max(nodes).max(1);
            demand.work.canonicalization = checked_add(
                demand.work.canonicalization,
                traversal
                    .checked_mul(2)
                    .ok_or(MemoryPlanError::ArithmeticOverflow {
                        field: "canonical draft and finalization work",
                    })?,
                "canonicalization work",
            )?;
            demand.retained_nodes = checked_add(demand.retained_nodes, nodes, "retained nodes")?;
            demand.turn_peak_bytes = checked_add(
                demand.turn_peak_bytes,
                encoded
                    .checked_mul(2)
                    .ok_or(MemoryPlanError::ArithmeticOverflow {
                        field: "canonical draft and candidate bytes",
                    })?,
                "canonical temporary bytes",
            )?;
            if request.implementation_memory == ImplementationMemoryClass::CanonicalSortUnique {
                let entries = outputs.iter().try_fold(0_u64, |total, output| {
                    checked_add(
                        total,
                        output.value.capacity_elements.required,
                        "canonical output entries",
                    )
                })?;
                let sort_work = ceil_log2(entries.max(1))
                    .checked_mul(encoded.max(nodes).max(entries))
                    .ok_or(MemoryPlanError::ArithmeticOverflow {
                        field: "canonical sorting work",
                    })?;
                demand.work.comparison =
                    checked_add(demand.work.comparison, sort_work, "canonical sorting work")?;
            }
        }
    }
    Ok(())
}

#[cfg(feature = "functions")]
fn known_footprint(
    witness: MemoryFootprintWitness,
) -> Result<CurrentMemoryFootprint, MemoryPlanError> {
    match witness {
        MemoryFootprintWitness::Known(footprint) => Ok(footprint),
        MemoryFootprintWitness::Deferred(stage) => {
            Err(MemoryPlanError::MissingFootprintWitness { stage })
        }
    }
}

#[cfg(feature = "functions")]
fn value_current_bytes(value: &ValueLayoutPlan) -> Result<u64, MemoryPlanError> {
    checked_add(
        value.current_address_span_bytes,
        value.payload.current_bytes,
        "current value bytes",
    )
}

#[cfg(feature = "functions")]
fn value_required_bytes(value: &ValueLayoutPlan) -> Result<u64, MemoryPlanError> {
    checked_add(
        value.capacity_bytes,
        value.payload.required_bytes,
        "required value bytes",
    )
}

#[cfg(feature = "functions")]
fn checked_add(left: u64, right: u64, field: &'static str) -> Result<u64, MemoryPlanError> {
    left.checked_add(right)
        .ok_or(MemoryPlanError::ArithmeticOverflow { field })
}

#[cfg(feature = "functions")]
fn ceil_log2(value: u64) -> u64 {
    if value <= 1 {
        0
    } else {
        u64::from(u64::BITS - (value - 1).leading_zeros())
    }
}
