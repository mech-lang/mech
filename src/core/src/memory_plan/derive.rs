//! Checked layout and capacity derivation.

use crate::{
    CardinalitySpec, DimensionExpr, DimensionLifetime, DimensionParameterId, ExtentEvolution,
    FloatWidth, IntegerWidth, MemoryTopology, Schema, SchemaBody, ShapeInstance, StorageTopology,
    check_schema_storage_compatibility,
};

#[cfg(all(feature = "no_std", feature = "functions"))]
use alloc::collections::BTreeMap;
#[cfg(feature = "no_std")]
use alloc::collections::BTreeSet;
#[cfg(feature = "no_std")]
use alloc::{boxed::Box, vec, vec::Vec};
#[cfg(all(not(feature = "no_std"), feature = "functions"))]
use std::collections::BTreeMap;
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, collections::BTreeSet, vec::Vec};

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
    /// Currently published outputs remain live through candidate staging and
    /// participate independently in comparison/work accounting.
    pub published_output_witnesses: &'a [MemoryFootprintWitness],
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
    let scratch_bytes = derive_scratch_allocations(
        &request,
        &requirements,
        &inputs,
        &outputs,
        &mut allocations,
        &mut arena_offsets,
        &mut next_object,
    )?;
    let mut demand = derive_call_demand(
        &request,
        &requirements,
        &inputs,
        &outputs,
        transaction_bytes,
    )?;
    // Scratch is physical plan data, not a second demand-only estimate.
    demand.turn_peak_bytes = checked_add(
        demand.turn_peak_bytes,
        scratch_bytes,
        "call scratch allocations",
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
    let mut deferred_witnesses = Vec::new();
    for (direction, witnesses) in [
        (PortDirection::Input, request.input_witnesses),
        (PortDirection::Output, request.output_witnesses),
    ] {
        for (port, witness) in witnesses.iter().enumerate() {
            if let MemoryFootprintWitness::Deferred(stage) = witness {
                deferred_witnesses.push(super::DeferredMemoryWitness {
                    direction,
                    port: checked_u16(port, "deferred witness port ordinal")?,
                    stage: *stage,
                });
            }
        }
    }
    allocations.sort_by_key(|allocation| allocation.id);
    Ok(CallMemoryPlan {
        bound_call: request.bound_call.clone(),
        inputs: inputs.into_boxed_slice(),
        outputs: outputs.into_boxed_slice(),
        input_storage: request.input_storage.into(),
        output_storage: request.output_storage.into(),
        input_witnesses: request.input_witnesses.into(),
        output_witnesses: request.output_witnesses.into(),
        output_regions: request.regions.into(),
        input_lifetimes: request
            .input_storage
            .iter()
            .map(|storage| storage.lifetime)
            .collect::<Vec<_>>()
            .into_boxed_slice(),
        output_lifetimes: request
            .output_storage
            .iter()
            .map(|storage| storage.lifetime)
            .collect::<Vec<_>>()
            .into_boxed_slice(),
        allocations: allocations.into_boxed_slice(),
        aliases: aliases.into_boxed_slice(),
        transactions: transactions.into_boxed_slice(),
        implementation_memory: request.implementation_memory,
        target: request.target.clone(),
        demand,
        deferred_witnesses: deferred_witnesses.into_boxed_slice(),
    })
}

/// Re-runs the complete call planner when a provider supplies the final
/// semantic operation contract. No derived field survives by assignment.
#[cfg(feature = "functions")]
pub fn replan_call_memory(
    previous: &CallMemoryPlan,
    bound_call: &BoundCall,
) -> Result<CallMemoryPlan, MemoryPlanError> {
    plan_call_memory(CallMemoryPlanningRequest {
        bound_call,
        input_storage: &previous.input_storage,
        output_storage: &previous.output_storage,
        input_witnesses: &previous.input_witnesses,
        output_witnesses: &previous.output_witnesses,
        published_output_witnesses: &previous.output_witnesses,
        implementation_memory: previous.implementation_memory,
        target: &previous.target,
        regions: &previous.output_regions,
    })
}

/// Re-derives the complete fixed-width call demand and placement after live
/// dimensions change. No bytes or offsets are patched into an old placement.
#[cfg(feature = "functions")]
pub fn replan_fixed_call_geometry(
    previous: &CallMemoryPlan,
    current: &BoundCall,
) -> Result<CallMemoryPlan, MemoryPlanError> {
    fn witnesses(
        descriptors: &[crate::ResolvedValueDescriptor],
        storage: &[PhysicalStorageDescriptor],
    ) -> Result<Vec<MemoryFootprintWitness>, MemoryPlanError> {
        if descriptors.len() != storage.len() {
            return Err(MemoryPlanError::DescriptorMismatch);
        }
        descriptors
            .iter()
            .zip(storage)
            .map(|(descriptor, storage)| {
                if !matches!(storage.slot, PlannedSlotKind::FixedScalar(_)) {
                    return Err(MemoryPlanError::DescriptorMismatch);
                }
                let extents = descriptor
                    .current_extents()
                    .map_err(|_| MemoryPlanError::DescriptorMismatch)?;
                let logical_elements = extents
                    .iter()
                    .try_fold(1_u64, |total, extent| total.checked_mul(*extent))
                    .ok_or(MemoryPlanError::ArithmeticOverflow {
                        field: "current call elements",
                    })?;
                Ok(MemoryFootprintWitness::Known(CurrentMemoryFootprint {
                    logical_elements,
                    shape_parameter_count: descriptor.shape().parameter_values().len() as u64,
                    ..CurrentMemoryFootprint::default()
                }))
            })
            .collect()
    }
    let input_witnesses = witnesses(current.inputs(), &previous.input_storage)?;
    let output_witnesses = witnesses(current.outputs(), &previous.output_storage)?;
    plan_call_memory(CallMemoryPlanningRequest {
        bound_call: current,
        input_storage: &previous.input_storage,
        output_storage: &previous.output_storage,
        input_witnesses: &input_witnesses,
        output_witnesses: &output_witnesses,
        published_output_witnesses: &output_witnesses,
        implementation_memory: previous.implementation_memory,
        target: &previous.target,
        regions: &previous.output_regions,
    })
}

pub struct ValueLayoutPlanningRequest<'a> {
    pub descriptor: &'a crate::ResolvedValueDescriptor,
    pub storage: &'a PhysicalStorageDescriptor,
    pub witness: MemoryFootprintWitness,
    pub target: &'a TargetMemoryProfile,
}

/// The single-value form of the existing R5 layout and publication plan,
/// used by owned-value ingress before a program call has been bound.
/// Object IDs are local to this plan, just as for a standalone call.
#[derive(Clone, Debug)]
pub struct OwnedValueMemoryPlan {
    pub value: ValueLayoutPlan,
    pub storage: PhysicalStorageDescriptor,
    pub allocations: Box<[super::AllocationPlan]>,
    pub arenas: Box<[super::ArenaPlan]>,
    pub transactions: [super::TransactionRequirement; 1],
    pub target: TargetMemoryProfile,
    pub demand: super::ResourceDemand,
    pub output_bytes: u64,
}

pub fn plan_owned_value_memory(
    request: ValueLayoutPlanningRequest<'_>,
) -> Result<OwnedValueMemoryPlan, MemoryPlanError> {
    let value = plan_value_layout(ValueLayoutPlanningRequest {
        descriptor: request.descriptor,
        storage: request.storage,
        witness: request.witness,
        target: request.target,
    })?;
    let current = super::MemoryObjectId::new(0);
    let staged = super::MemoryObjectId::new(1);
    let arena = super::MemoryArenaId::new(0);
    let staged_offset = align_up(value.capacity_bytes, value.slot.alignment)?;
    let capacity = staged_offset.checked_add(value.capacity_bytes).ok_or(
        MemoryPlanError::ArithmeticOverflow {
            field: "owned value transaction arena",
        },
    )?;
    if capacity > request.target.maximum_addressable_bytes {
        return Err(MemoryPlanError::TargetAddressOverflow);
    }
    let owner = super::MemoryObjectOwner::DirectCallPort {
        call: 0,
        direction: crate::PortDirection::Output,
        port: 0,
    };
    let allocation = super::AllocationPlan {
        id: current,
        owner: owner.clone(),
        role: super::AllocationRole::FixedStorage,
        slot: Some(value.storage.planned_slot()),
        space: request.storage.space,
        current_bytes: value.current_address_span_bytes,
        capacity_bytes: value.capacity_bytes,
        payload_block_capacity: 0,
        alignment: value.slot.alignment,
        lifetime: super::MemoryLifetime::Activation,
        placement: super::ArenaPlacement { arena, offset: 0 },
        reuse_group: None,
    };
    let stage = super::AllocationPlan {
        id: staged,
        role: super::AllocationRole::TransactionStage,
        placement: super::ArenaPlacement {
            arena,
            offset: staged_offset,
        },
        lifetime: super::MemoryLifetime::Transaction {
            first: super::MemoryPlanPoint::new(0),
            last: super::MemoryPlanPoint::new(0),
        },
        ..allocation.clone()
    };
    let mut allocations = vec![allocation, stage];
    let mut arenas = vec![super::ArenaPlan {
        id: arena,
        space: request.storage.space,
        backing: super::ArenaBackingKind::ContiguousBytes,
        alignment: value.slot.alignment,
        capacity_bytes: capacity,
        members: vec![current, staged].into_boxed_slice(),
    }];
    let mut total_capacity = capacity;
    let mut transaction_peak = value.capacity_bytes;
    let mut cloned_bytes = value.current_address_span_bytes;
    if value.payload.required_bytes != 0 || value.payload.maximum_bytes.is_none() {
        let current_payload = super::MemoryObjectId::new(2);
        let staged_payload = super::MemoryObjectId::new(3);
        let payload_arena = super::MemoryArenaId::new(1);
        let staged_payload_offset = value.payload.required_bytes;
        let payload_capacity = staged_payload_offset
            .checked_add(value.payload.required_bytes)
            .ok_or(MemoryPlanError::ArithmeticOverflow {
                field: "owned value payload arena",
            })?;
        if payload_capacity > request.target.maximum_addressable_bytes {
            return Err(MemoryPlanError::TargetAddressOverflow);
        }
        allocations.push(super::AllocationPlan {
            id: current_payload,
            owner: owner.clone(),
            role: super::AllocationRole::VariablePayload,
            slot: None,
            space: request.storage.space,
            current_bytes: value.payload.current_bytes,
            capacity_bytes: value.payload.required_bytes,
            payload_block_capacity: value.payload.required_nodes.max(1),
            alignment: 1,
            lifetime: super::MemoryLifetime::Activation,
            placement: super::ArenaPlacement {
                arena: payload_arena,
                offset: 0,
            },
            reuse_group: None,
        });
        allocations.push(super::AllocationPlan {
            id: staged_payload,
            owner: owner.clone(),
            role: super::AllocationRole::VariablePayload,
            slot: None,
            space: request.storage.space,
            current_bytes: value.payload.current_bytes,
            capacity_bytes: value.payload.required_bytes,
            payload_block_capacity: value.payload.required_nodes.max(1),
            alignment: 1,
            lifetime: super::MemoryLifetime::Transaction {
                first: super::MemoryPlanPoint::new(0),
                last: super::MemoryPlanPoint::new(0),
            },
            placement: super::ArenaPlacement {
                arena: payload_arena,
                offset: staged_payload_offset,
            },
            reuse_group: None,
        });
        arenas.push(super::ArenaPlan {
            id: payload_arena,
            space: request.storage.space,
            backing: super::ArenaBackingKind::IndirectOwnedPayloads,
            alignment: 1,
            capacity_bytes: payload_capacity,
            members: vec![current_payload, staged_payload].into_boxed_slice(),
        });
        total_capacity = total_capacity.checked_add(payload_capacity).ok_or(
            MemoryPlanError::ArithmeticOverflow {
                field: "owned value total capacity",
            },
        )?;
        transaction_peak = transaction_peak
            .checked_add(value.payload.required_bytes)
            .ok_or(MemoryPlanError::ArithmeticOverflow {
                field: "owned value transaction peak",
            })?;
        cloned_bytes = cloned_bytes
            .checked_add(value.payload.current_bytes)
            .ok_or(MemoryPlanError::ArithmeticOverflow {
                field: "owned value cloned bytes",
            })?;
    }
    let demand = super::ResourceDemand {
        persistent_bytes: total_capacity,
        activation_bytes: total_capacity,
        turn_peak_bytes: transaction_peak,
        transaction_peak_bytes: transaction_peak,
        cloned_bytes,
        output_elements: value.current_elements,
        retained_nodes: value.payload.required_nodes.max(value.current_elements),
        storage_bindings: 1,
        work: super::WorkDemand {
            compute: value.current_elements,
            ..super::WorkDemand::default()
        },
        ..super::ResourceDemand::default()
    };
    let output_bytes = value_required_bytes(&value)?;
    if let Some(violation) = super::evaluate_memory_budget(
        owner,
        demand,
        output_bytes,
        total_capacity,
        request.target.limits,
    )
    .first()
    .cloned()
    {
        return Err(MemoryPlanError::TargetLimitExceeded { violation });
    }
    Ok(OwnedValueMemoryPlan {
        value,
        storage: request.storage.clone(),
        allocations: allocations.into_boxed_slice(),
        arenas: arenas.into_boxed_slice(),
        transactions: [super::TransactionRequirement::StageAndSwap { current, staged }],
        target: request.target.clone(),
        demand,
        output_bytes,
    })
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
    // A template/call plan is allowed to retain an explicit deferred witness.
    // The zero-valued payload below is not an estimate: the stage remains on
    // `CallMemoryPlan::deferred_witnesses` and must be replaced by the
    // activation or turn planner before covered materialization begins.
    let footprint = match request.witness {
        MemoryFootprintWitness::Known(footprint) => Some(footprint),
        MemoryFootprintWitness::Deferred(_) => None,
    };
    let slot = target_slot_layout(request.target, request.storage.slot)?;
    super::validate_alignment(slot.alignment)?;
    let storage = storage_layout(
        contract.topology,
        request.storage.capabilities.topology,
        request.storage.slot,
    )?;
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
            let maximum = capacities.iter().filter_map(|value| value.maximum).min();
            let evolution = match (maximum, joined_evolution(&capacities)) {
                (Some(_), ExtentEvolution::TurnUnbounded) => ExtentEvolution::TurnBounded,
                (_, evolution) => evolution,
            };
            (maximum, evolution)
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
            let current = footprint.map_or(0, |footprint| footprint.logical_elements);
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

fn storage_layout(
    semantic: MemoryTopology,
    physical: StorageTopology,
    slot: PlannedSlotKind,
) -> Result<StorageLayoutClass, MemoryPlanError> {
    Ok(match physical {
        StorageTopology::CanonicalValue => {
            StorageLayoutClass::CanonicalSnapshot { topology: semantic }
        }
        StorageTopology::Scalar(_) => StorageLayoutClass::Scalar { slot },
        StorageTopology::DenseSequence { .. } => match semantic {
            MemoryTopology::DenseSequence { rank: 2 } => {
                StorageLayoutClass::DenseColumnMajor { slot }
            }
            MemoryTopology::DenseSequence { rank } => {
                return Err(MemoryPlanError::UnsupportedDenseRank { rank });
            }
            _ => return Err(MemoryPlanError::DescriptorMismatch),
        },
        StorageTopology::Opaque => return Err(MemoryPlanError::UnsupportedStorageLayout),
        StorageTopology::Tagged
        | StorageTopology::Product
        | StorageTopology::Columnar
        | StorageTopology::OrderedSet
        | StorageTopology::OrderedMap
        | StorageTopology::ReifiedType => {
            StorageLayoutClass::CanonicalSnapshot { topology: semantic }
        }
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
    let Some(footprint) = footprint else {
        return Ok(PayloadCapacityPlan {
            current_bytes: 0,
            required_bytes: 0,
            maximum_bytes: None,
            current_nodes: 0,
            required_nodes: 0,
            maximum_nodes: None,
            authority: CapacityAuthority::CurrentValueWitness,
            growth: GrowthPolicy::ReplanBeforeGrowth,
        });
    };
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
            Complex(FloatWidth::W32) => layouts.c32_slot,
            Complex(FloatWidth::W64) => layouts.c64_slot,
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
        || request.bound_call.outputs().len() != request.published_output_witnesses.len()
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
fn payload_arena_for_space(space: MemorySpace) -> MemoryArenaId {
    let base = arena_for_space(space).get();
    MemoryArenaId::new(base | (1_u32 << 31))
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
        slot: Some(value.storage.planned_slot()),
        space: storage.space,
        current_bytes: value.current_address_span_bytes,
        capacity_bytes: value.capacity_bytes,
        payload_block_capacity: 0,
        alignment: value.slot.alignment,
        lifetime: storage.lifetime,
        placement: allocate_offset(offsets, arena, value.capacity_bytes, value.slot.alignment)?,
        reuse_group: None,
    });
    if value.payload.required_bytes != 0 || value.payload.maximum_bytes.is_none() {
        let payload = MemoryObjectId::new(*next_object);
        *next_object = checked_next_object(*next_object)?;
        let payload_arena = payload_arena_for_space(storage.space);
        allocations.push(AllocationPlan {
            id: payload,
            owner,
            role: AllocationRole::VariablePayload,
            slot: None,
            space: storage.space,
            current_bytes: value.payload.current_bytes,
            capacity_bytes: value.payload.required_bytes,
            payload_block_capacity: value.payload.required_nodes.max(1),
            alignment: 1,
            lifetime: storage.lifetime,
            placement: allocate_offset(offsets, payload_arena, value.payload.required_bytes, 1)?,
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
        let transaction_owner = MemoryObjectOwner::DirectCallPort {
            call: 0,
            direction: PortDirection::Output,
            port: checked_u16(ordinal, "transaction output ordinal")?,
        };
        allocations.push(AllocationPlan {
            id: staged,
            owner: transaction_owner.clone(),
            role: AllocationRole::TransactionStage,
            slot: Some(output.value.storage.planned_slot()),
            space: storage.space,
            current_bytes: output.value.current_address_span_bytes,
            capacity_bytes: output.value.capacity_bytes,
            payload_block_capacity: 0,
            alignment: output.value.slot.alignment,
            lifetime: MemoryLifetime::Transaction {
                first: super::MemoryPlanPoint::new(0),
                last: super::MemoryPlanPoint::new(0),
            },
            placement: allocate_offset(
                offsets,
                arena,
                output.value.capacity_bytes,
                output.value.slot.alignment,
            )?,
            reuse_group: None,
        });
        if output.value.payload.required_bytes != 0 || output.value.payload.maximum_bytes.is_none()
        {
            let payload = MemoryObjectId::new(*next_object);
            *next_object = checked_next_object(*next_object)?;
            let payload_arena = payload_arena_for_space(storage.space);
            allocations.push(AllocationPlan {
                id: payload,
                owner: transaction_owner,
                role: AllocationRole::VariablePayload,
                slot: None,
                space: storage.space,
                current_bytes: output.value.payload.current_bytes,
                capacity_bytes: output.value.payload.required_bytes,
                payload_block_capacity: output.value.payload.required_nodes.max(1),
                alignment: 1,
                lifetime: MemoryLifetime::Transaction {
                    first: super::MemoryPlanPoint::new(0),
                    last: super::MemoryPlanPoint::new(0),
                },
                placement: allocate_offset(
                    offsets,
                    payload_arena,
                    output.value.payload.required_bytes,
                    1,
                )?,
                reuse_group: None,
            });
        }
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

/// Describe every implementation temporary before deriving its byte demand.
/// IDs and ordinals are call-local until the program planner remaps the call.
#[cfg(feature = "functions")]
fn derive_scratch_allocations(
    request: &CallMemoryPlanningRequest<'_>,
    requirements: &crate::OperationMemoryRequirements,
    inputs: &[PortMemoryPlan],
    outputs: &[PortMemoryPlan],
    allocations: &mut Vec<AllocationPlan>,
    offsets: &mut BTreeMap<MemoryArenaId, u64>,
    next_object: &mut u32,
) -> Result<u64, MemoryPlanError> {
    let mut ordinal = 0_usize;
    let mut bytes = 0_u64;
    let mut scratch = |role, size, alignment, space| -> Result<(), MemoryPlanError> {
        let id = MemoryObjectId::new(*next_object);
        *next_object = checked_next_object(*next_object)?;
        allocations.push(AllocationPlan {
            id,
            owner: MemoryObjectOwner::NodeScratch {
                node: crate::NodeId::new(0),
                ordinal: checked_u16(ordinal, "call scratch ordinal")?,
            },
            role,
            slot: None,
            space,
            current_bytes: size,
            capacity_bytes: size,
            payload_block_capacity: 0,
            alignment,
            lifetime: MemoryLifetime::Turn {
                first: super::MemoryPlanPoint::new(0),
                last: super::MemoryPlanPoint::new(0),
            },
            placement: allocate_offset(offsets, arena_for_space(space), size, alignment)?,
            reuse_group: None,
        });
        ordinal += 1;
        bytes = checked_add(bytes, size, "implementation scratch bytes")?;
        Ok(())
    };
    for output in &requirements.outputs {
        if let Some(OutputConstruction::ReadModifyWrite { base_input, .. }) = output.construction {
            let input = inputs
                .get(base_input as usize)
                .ok_or(MemoryPlanError::DescriptorArityMismatch)?;
            scratch(
                AllocationRole::Scratch,
                value_current_bytes(&input.value)?,
                input.value.slot.alignment,
                request.input_storage[base_input as usize].space,
            )?;
        }
    }
    match request.implementation_memory {
        ImplementationMemoryClass::NoAdditionalScratch => {}
        ImplementationMemoryClass::CloneInput { input } => {
            let port = inputs
                .get(input as usize)
                .ok_or(MemoryPlanError::DescriptorArityMismatch)?;
            scratch(
                AllocationRole::Scratch,
                value_current_bytes(&port.value)?,
                port.value.slot.alignment,
                request.input_storage[input as usize].space,
            )?;
        }
        ImplementationMemoryClass::AbiContiguousBridge { input, output } => {
            let input = inputs
                .get(input as usize)
                .ok_or(MemoryPlanError::DescriptorArityMismatch)?;
            let output = outputs
                .get(output as usize)
                .ok_or(MemoryPlanError::DescriptorArityMismatch)?;
            scratch(
                AllocationRole::Scratch,
                value_current_bytes(&input.value)?,
                input.value.slot.alignment,
                MemorySpace::Host,
            )?;
            scratch(
                AllocationRole::Scratch,
                value_required_bytes(&output.value)?,
                output.value.slot.alignment,
                MemorySpace::Host,
            )?;
        }
        ImplementationMemoryClass::ExternalMarshalling => {
            let value_container_bytes = u64::try_from(inputs.len())
                .ok()
                .and_then(|count| count.checked_mul(core::mem::size_of::<crate::Value>() as u64))
                .ok_or(MemoryPlanError::ArithmeticOverflow {
                    field: "external argument container bytes",
                })?;
            scratch(
                AllocationRole::Scratch,
                value_container_bytes,
                u32::try_from(core::mem::align_of::<crate::Value>()).unwrap_or(u32::MAX),
                MemorySpace::Host,
            )?;
            for (ordinal, input) in inputs.iter().enumerate() {
                let (draft_bytes, finalization_bytes) = external_marshalling_input_bytes(
                    input,
                    request.input_witnesses.get(ordinal).copied(),
                )?;
                if draft_bytes != 0 {
                    scratch(
                        AllocationRole::Scratch,
                        draft_bytes,
                        u32::try_from(core::mem::align_of::<crate::ValueDataDraft>())
                            .unwrap_or(u32::MAX),
                        MemorySpace::Host,
                    )?;
                }
                if finalization_bytes != 0 {
                    scratch(
                        AllocationRole::Scratch,
                        finalization_bytes,
                        input.value.slot.alignment,
                        MemorySpace::Host,
                    )?;
                }
            }
        }
        ImplementationMemoryClass::MatrixSolve => {
            let [coefficients, _rhs] = inputs else {
                return Err(MemoryPlanError::MatrixSolveLayoutInvalid);
            };
            let [rows, columns] = coefficients.value.axes.as_ref() else {
                return Err(MemoryPlanError::MatrixSolveLayoutInvalid);
            };
            if rows.current != columns.current {
                return Err(MemoryPlanError::MatrixSolveLayoutInvalid);
            }
            let [solution] = outputs else {
                return Err(MemoryPlanError::MatrixSolveLayoutInvalid);
            };
            let space = request.input_storage[0].space;
            scratch(
                AllocationRole::Scratch,
                value_current_bytes(&coefficients.value)?,
                coefficients.value.slot.alignment,
                space,
            )?;
            scratch(
                AllocationRole::Scratch,
                value_required_bytes(&solution.value)?,
                solution.value.slot.alignment,
                request.output_storage[0].space,
            )?;
            let index = target_slot_layout(
                request.target,
                PlannedSlotKind::FixedScalar(crate::ScalarMemoryKind::Index),
            )?;
            let pivot_bytes = rows
                .current
                .checked_mul(align_up(index.bytes, index.alignment)?)
                .ok_or(MemoryPlanError::ArithmeticOverflow {
                    field: "matrix solve pivot bytes",
                })?;
            scratch(
                AllocationRole::OrderedIndex,
                pivot_bytes,
                index.alignment,
                space,
            )?;
        }
        ImplementationMemoryClass::CanonicalFinalize
        | ImplementationMemoryClass::CanonicalSortUnique => {
            for (ordinal, output) in outputs.iter().enumerate() {
                let footprint =
                    known_footprint(request.output_witnesses[ordinal])?.unwrap_or_default();
                // Draft and frozen finalization are separate coexistence
                // obligations. The finalizer includes packed payload plus its
                // immutable roots and schema/shape metadata.
                let draft_size = value_required_bytes(&output.value)?
                    .max(canonical_snapshot_draft_bytes(footprint)?);
                let finalization_size = canonical_snapshot_finalization_bytes(footprint)?;
                let space = request.output_storage[ordinal].space;
                scratch(
                    AllocationRole::Scratch,
                    draft_size,
                    output.value.slot.alignment,
                    space,
                )?;
                scratch(
                    AllocationRole::Scratch,
                    finalization_size,
                    output.value.slot.alignment,
                    space,
                )?;
                if request.implementation_memory == ImplementationMemoryClass::CanonicalSortUnique {
                    let index = target_slot_layout(
                        request.target,
                        PlannedSlotKind::FixedScalar(crate::ScalarMemoryKind::Index),
                    )?;
                    let size = output
                        .value
                        .capacity_elements
                        .required
                        .checked_mul(align_up(index.bytes, index.alignment)?)
                        .ok_or(MemoryPlanError::ArithmeticOverflow {
                            field: "canonical index bytes",
                        })?;
                    scratch(AllocationRole::OrderedIndex, size, index.alignment, space)?;
                }
            }
        }
    }
    Ok(bytes)
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
                let candidate =
                    known_footprint(request.output_witnesses[ordinal])?.unwrap_or_default();
                let current = known_footprint(request.published_output_witnesses[ordinal])?
                    .unwrap_or_default();
                demand.work.comparison = checked_add(
                    demand.work.comparison,
                    publication_comparison_work(current, candidate)?,
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
        }
        ImplementationMemoryClass::AbiContiguousBridge { input, output } => {
            let input = inputs
                .get(input as usize)
                .ok_or(MemoryPlanError::DescriptorArityMismatch)?;
            let output = outputs
                .get(output as usize)
                .ok_or(MemoryPlanError::DescriptorArityMismatch)?;
            demand.cloned_bytes = checked_add(
                demand.cloned_bytes,
                value_current_bytes(&input.value)?,
                "ABI input bridge copy",
            )?;
            demand.work.compute = checked_add(
                demand.work.compute,
                input
                    .value
                    .current_elements
                    .checked_add(output.value.current_elements)
                    .ok_or(MemoryPlanError::ArithmeticOverflow {
                        field: "ABI bridge element work",
                    })?,
                "ABI bridge element work",
            )?;
        }
        ImplementationMemoryClass::ExternalMarshalling => {
            for (ordinal, input) in inputs.iter().enumerate() {
                let (draft_bytes, finalization_bytes) = external_marshalling_input_bytes(
                    input,
                    request.input_witnesses.get(ordinal).copied(),
                )?;
                demand.cloned_bytes = checked_add(
                    demand.cloned_bytes,
                    checked_add(
                        draft_bytes,
                        finalization_bytes,
                        "external argument construction bytes",
                    )?,
                    "external argument marshalling bytes",
                )?;
                if input.value.storage.planned_slot() != PlannedSlotKind::CanonicalValueHandle {
                    demand.work.compute = checked_add(
                        demand.work.compute,
                        input.value.current_elements,
                        "external argument marshalling work",
                    )?;
                    if let Some(footprint) = request
                        .input_witnesses
                        .get(ordinal)
                        .copied()
                        .map(known_footprint)
                        .transpose()?
                        .flatten()
                    {
                        demand.retained_nodes = checked_add(
                            demand.retained_nodes,
                            footprint.retained_nodes,
                            "external argument retained nodes",
                        )?;
                    }
                }
            }
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
            demand.cloned_bytes = checked_add(
                demand.cloned_bytes,
                coefficient_bytes,
                "matrix solve coefficient clone",
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
            let contribution = canonical_footprint_demand(
                request.implementation_memory,
                outputs,
                request.output_witnesses,
            )?;
            demand.work.canonicalization = checked_add(
                demand.work.canonicalization,
                contribution.canonicalization,
                "canonicalization work",
            )?;
            demand.work.comparison = checked_add(
                demand.work.comparison,
                contribution.comparison,
                "canonical sorting work",
            )?;
            demand.retained_nodes = checked_add(
                demand.retained_nodes,
                contribution.retained_nodes,
                "retained nodes",
            )?;
        }
    }
    Ok(())
}

/// Conservative bytes required by the selected common canonical finalizer.
/// The payload/encoding term covers the packed immutable data; the fixed
/// metadata term covers the two shared roots, Value metadata, schema-table
/// shell, and shape parameters. Nested schema payload is supplied through the
/// witness's schema byte count when present.
pub fn canonical_snapshot_finalization_bytes(
    footprint: CurrentMemoryFootprint,
) -> Result<u64, MemoryPlanError> {
    let payload = footprint.payload_bytes.max(footprint.encoded_bytes);
    let shape = footprint
        .shape_parameter_count
        .checked_mul(core::mem::size_of::<u64>() as u64)
        .ok_or(MemoryPlanError::ArithmeticOverflow {
            field: "canonical finalization shape bytes",
        })?;
    let metadata = (core::mem::size_of::<crate::Value>() as u64)
        .checked_mul(4)
        .and_then(|bytes| bytes.checked_add(core::mem::size_of::<crate::SchemaTable>() as u64))
        .ok_or(MemoryPlanError::ArithmeticOverflow {
            field: "canonical finalization metadata bytes",
        })?;
    [payload, footprint.schema_bytes, shape, metadata]
        .into_iter()
        .try_fold(0_u64, |total, bytes| {
            checked_add(total, bytes, "canonical finalization bytes")
        })
}

/// Conservative draft/build workspace for one canonical candidate. Draft
/// nodes coexist with cloned String/nested payload and dimension metadata
/// until the common finalizer consumes them.
pub fn canonical_snapshot_draft_bytes(
    footprint: CurrentMemoryFootprint,
) -> Result<u64, MemoryPlanError> {
    let nodes = footprint.logical_elements.max(footprint.retained_nodes);
    let node_bytes = nodes
        .checked_mul(core::mem::size_of::<crate::ValueDataDraft>() as u64)
        .ok_or(MemoryPlanError::ArithmeticOverflow {
            field: "canonical draft node bytes",
        })?;
    let shape = footprint
        .shape_parameter_count
        .checked_mul(core::mem::size_of::<u64>() as u64)
        .ok_or(MemoryPlanError::ArithmeticOverflow {
            field: "canonical draft shape bytes",
        })?;
    [
        node_bytes,
        footprint.payload_bytes,
        footprint.encoded_bytes,
        shape,
    ]
    .into_iter()
    .try_fold(0_u64, |total, bytes| {
        checked_add(total, bytes, "canonical draft bytes")
    })
}

#[cfg(feature = "functions")]
fn external_marshalling_input_bytes(
    input: &PortMemoryPlan,
    witness: Option<MemoryFootprintWitness>,
) -> Result<(u64, u64), MemoryPlanError> {
    if input.value.storage.planned_slot() == PlannedSlotKind::CanonicalValueHandle {
        return Ok((0, 0));
    }
    let footprint = witness
        .map(known_footprint)
        .transpose()?
        .flatten()
        .unwrap_or(CurrentMemoryFootprint {
            logical_elements: input.value.current_elements,
            fixed_bytes: value_current_bytes(&input.value)?,
            shape_parameter_count: input.descriptor.shape().parameter_values().len() as u64,
            ..CurrentMemoryFootprint::default()
        });
    let draft_bytes = if matches!(
        input.value.storage,
        StorageLayoutClass::DenseColumnMajor { .. }
    ) {
        input
            .value
            .current_elements
            .checked_mul(core::mem::size_of::<crate::ValueDataDraft>() as u64)
            .ok_or(MemoryPlanError::ArithmeticOverflow {
                field: "external canonical draft bytes",
            })?
    } else {
        0
    };
    let canonical = CurrentMemoryFootprint {
        logical_elements: footprint.logical_elements.max(input.value.current_elements),
        payload_bytes: footprint
            .payload_bytes
            .max(value_current_bytes(&input.value)?),
        encoded_bytes: footprint.encoded_bytes,
        retained_nodes: footprint.retained_nodes,
        schema_bytes: footprint.schema_bytes,
        shape_parameter_count: footprint.shape_parameter_count,
        ..CurrentMemoryFootprint::default()
    };
    Ok((
        draft_bytes,
        canonical_snapshot_finalization_bytes(canonical)?,
    ))
}

#[cfg(feature = "functions")]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct FootprintDemandContribution {
    retained_nodes: u64,
    comparison: u64,
    canonicalization: u64,
}

/// Work for distinct retained and candidate values; neither side is inferred
/// from the other when a payload grows or shrinks between turns.
#[cfg(feature = "functions")]
pub fn publication_comparison_work(
    current: CurrentMemoryFootprint,
    candidate: CurrentMemoryFootprint,
) -> Result<u64, MemoryPlanError> {
    let shape = current
        .shape_parameter_count
        .max(candidate.shape_parameter_count)
        .checked_mul(8)
        .ok_or(MemoryPlanError::ArithmeticOverflow {
            field: "publication shape work",
        })?;
    [
        current.schema_bytes,
        candidate.schema_bytes,
        current.encoded_bytes,
        candidate.encoded_bytes,
        shape,
    ]
    .into_iter()
    .try_fold(0_u64, |sum, amount| {
        checked_add(sum, amount, "publication comparison work")
    })
}

#[cfg(feature = "functions")]
fn canonical_footprint_demand(
    implementation: ImplementationMemoryClass,
    outputs: &[PortMemoryPlan],
    witnesses: &[MemoryFootprintWitness],
) -> Result<FootprintDemandContribution, MemoryPlanError> {
    let mut encoded = 0_u64;
    let mut nodes = 0_u64;
    for witness in witnesses {
        let footprint = known_footprint(*witness)?.unwrap_or_default();
        encoded = checked_add(encoded, footprint.encoded_bytes, "canonical encoded bytes")?;
        nodes = checked_add(nodes, footprint.retained_nodes, "canonical retained nodes")?;
    }
    let traversal = encoded.max(nodes).max(1);
    let canonicalization = traversal
        .checked_mul(2)
        .ok_or(MemoryPlanError::ArithmeticOverflow {
            field: "canonical draft and finalization work",
        })?;
    let comparison = if implementation == ImplementationMemoryClass::CanonicalSortUnique {
        let entries = outputs
            .iter()
            .enumerate()
            .try_fold(0_u64, |total, (ordinal, output)| {
                let entries = witnesses
                    .get(ordinal)
                    .copied()
                    .map(known_footprint)
                    .transpose()?
                    .flatten()
                    .map_or(output.value.capacity_elements.required, |footprint| {
                        footprint.logical_elements
                    });
                checked_add(total, entries, "canonical output entries")
            })?;
        ceil_log2(entries.max(1))
            .checked_mul(encoded.max(nodes).max(entries))
            .ok_or(MemoryPlanError::ArithmeticOverflow {
                field: "canonical sorting work",
            })?
    } else {
        0
    };
    Ok(FootprintDemandContribution {
        retained_nodes: nodes,
        comparison,
        canonicalization,
    })
}

/// Resolves every footprint-dependent component of a call's demand through
/// the same semantic calculations used by initial call planning. The stored
/// demand remains the static baseline; deferred witnesses replace, rather
/// than layer independent estimates over, that baseline.
#[cfg(feature = "functions")]
pub fn resolve_deferred_call_demand(
    call: &CallMemoryPlan,
    resolved: &BTreeMap<(PortDirection, u16), CurrentMemoryFootprint>,
) -> Result<ResourceDemand, MemoryPlanError> {
    Ok(resolve_deferred_call_memory(call, resolved)?.demand)
}

/// Re-runs the complete call planner with every turn-deferred witness
/// replaced by its concrete footprint. Consumers that need transaction or
/// allocation geometry use this plan rather than independently patching the
/// original zero-footprint projection.
#[cfg(feature = "functions")]
pub fn resolve_deferred_call_memory(
    call: &CallMemoryPlan,
    resolved: &BTreeMap<(PortDirection, u16), CurrentMemoryFootprint>,
) -> Result<CallMemoryPlan, MemoryPlanError> {
    resolve_current_call_memory(call, &call.bound_call, resolved, None)
}

/// Re-runs one call's complete planner for current semantic descriptors and
/// current/prospective value footprints. This is the payload-aware sibling of
/// `replan_fixed_call_geometry`; no placement or demand field is patched.
#[cfg(feature = "functions")]
pub fn resolve_current_call_memory(
    call: &CallMemoryPlan,
    current: &crate::BoundCall,
    resolved: &BTreeMap<(PortDirection, u16), CurrentMemoryFootprint>,
    published_outputs: Option<&[CurrentMemoryFootprint]>,
) -> Result<CallMemoryPlan, MemoryPlanError> {
    if call.inputs.len() != call.input_witnesses.len()
        || call.inputs.len() != call.input_lifetimes.len()
        || call.outputs.len() != call.output_witnesses.len()
        || call.outputs.len() != call.output_lifetimes.len()
    {
        return Err(MemoryPlanError::DescriptorArityMismatch);
    }
    let mut input_witnesses = call.input_witnesses.to_vec();
    let mut output_witnesses = call.output_witnesses.to_vec();
    let mut published_output_witnesses = call.output_witnesses.to_vec();
    if let Some(published_outputs) = published_outputs {
        if published_outputs.len() != published_output_witnesses.len() {
            return Err(MemoryPlanError::DescriptorArityMismatch);
        }
        for (witness, footprint) in published_output_witnesses
            .iter_mut()
            .zip(published_outputs.iter().copied())
        {
            *witness = MemoryFootprintWitness::Known(footprint);
        }
    }
    for deferred in &call.deferred_witnesses {
        if deferred.stage != super::MemoryWitnessStage::Turn {
            continue;
        }
        let footprint = resolved
            .get(&(deferred.direction, deferred.port))
            .copied()
            .ok_or(MemoryPlanError::MissingFootprintWitness {
                stage: super::MemoryWitnessStage::Turn,
            })?;
        let witnesses = match deferred.direction {
            PortDirection::Input => &mut input_witnesses,
            PortDirection::Output => &mut output_witnesses,
        };
        *witnesses
            .get_mut(usize::from(deferred.port))
            .ok_or(MemoryPlanError::DescriptorArityMismatch)? =
            MemoryFootprintWitness::Known(footprint);
    }

    for (&(direction, port), &footprint) in resolved {
        let witnesses = match direction {
            PortDirection::Input => &mut input_witnesses,
            PortDirection::Output => &mut output_witnesses,
        };
        *witnesses
            .get_mut(usize::from(port))
            .ok_or(MemoryPlanError::DescriptorArityMismatch)? =
            MemoryFootprintWitness::Known(footprint);
    }

    plan_call_memory(CallMemoryPlanningRequest {
        bound_call: current,
        input_storage: &call.input_storage,
        output_storage: &call.output_storage,
        input_witnesses: &input_witnesses,
        output_witnesses: &output_witnesses,
        published_output_witnesses: &published_output_witnesses,
        implementation_memory: call.implementation_memory,
        target: &call.target,
        regions: &call.output_regions,
    })
}

#[cfg(feature = "functions")]
fn known_footprint(
    witness: MemoryFootprintWitness,
) -> Result<Option<CurrentMemoryFootprint>, MemoryPlanError> {
    match witness {
        MemoryFootprintWitness::Known(footprint) => Ok(Some(footprint)),
        MemoryFootprintWitness::Deferred(_) => Ok(None),
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

fn value_required_bytes(value: &ValueLayoutPlan) -> Result<u64, MemoryPlanError> {
    checked_add(
        value.capacity_bytes,
        value.payload.required_bytes,
        "required value bytes",
    )
}

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
