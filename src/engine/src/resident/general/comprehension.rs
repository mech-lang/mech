use super::*;

#[derive(Clone, Debug)]
pub enum ActivatedCollectionStep {
    Generator {
        source: ResidentReadLocation,
        pattern: crate::CollectionPattern<ResidentRegion, ResidentReadLocation>,
    },
    Operation {
        node: ActivatedNodeIndex,
        work: u64,
    },
    Filter(ResidentReadLocation),
}

#[derive(Clone, Debug)]
pub struct ActivatedComprehensionNode {
    pub artifact_node: NodeId,
    pub reads: Range<u32>,
    pub write: ResidentWriteLocation,
    pub kind: crate::ComprehensionKind,
    pub output_schema: SchemaId,
    pub steps: Box<[ActivatedCollectionStep]>,
    pub locals: Box<[ResidentRegion]>,
    pub yield_value: ResidentReadLocation,
}

pub(super) fn locals(control: &crate::ComprehensionDeclaration) -> Vec<SchemaId> {
    let mut locals = Vec::new();
    for step in &control.steps {
        match step {
            crate::ComprehensionStep::Generator { pattern, .. } => {
                pattern.bindings(&mut |local, schema| {
                    assert_eq!(local as usize, locals.len(), "validated collection locals");
                    locals.push(*schema);
                })
            }
            crate::ComprehensionStep::Operation(operation) => locals.push(operation.schema),
            crate::ComprehensionStep::Filter(_) => {}
        }
    }
    locals
}

pub(super) fn owns_output(artifact: &ProgramArtifact, mut slot: CellSlotId) -> bool {
    loop {
        let Some(declaration) = artifact.slots().get(slot.get() as usize) else {
            return false;
        };
        match declaration.producer {
            ProducerReference::NodeOutput { node, .. } => {
                return matches!(
                    artifact.nodes()[node.get() as usize].body,
                    crate::ExecutableNodeBody::Comprehension(_)
                );
            }
            ProducerReference::Output {
                source: ArtifactSource::Slot(source),
                ..
            } => slot = source,
            _ => return false,
        }
    }
}

fn primitive(body: &SchemaBody) -> bool {
    matches!(
        body,
        SchemaBody::Bool
            | SchemaBody::Index
            | SchemaBody::FloatingPoint(mech_core::FloatWidth::W64)
    )
}

fn supported_pattern(pattern: &crate::CollectionPattern, schemas: &mech_core::SchemaTable) -> bool {
    match pattern {
        crate::CollectionPattern::Wildcard | crate::CollectionPattern::Equal(_) => true,
        crate::CollectionPattern::Bind { schema, .. } => schemas
            .get(*schema)
            .is_some_and(|schema| primitive(schema.body())),
        crate::CollectionPattern::Tuple(items) => {
            items.iter().all(|item| supported_pattern(item, schemas))
        }
        crate::CollectionPattern::Array {
            prefix,
            rest,
            suffix,
        } => {
            prefix
                .iter()
                .chain(suffix.iter())
                .all(|item| supported_pattern(item, schemas))
                && rest
                    .as_deref()
                    .is_none_or(|rest| matches!(rest, crate::CollectionPattern::Wildcard))
        }
    }
}

fn supported_element(body: &SchemaBody) -> bool {
    primitive(body)
        || match body {
            SchemaBody::Atom(_) => true,
            SchemaBody::Tuple(fields) => fields.iter().all(supported_element),
            SchemaBody::Matrix { element, .. } => supported_element(element),
            _ => false,
        }
}

fn activate_pattern(
    pattern: &crate::CollectionPattern,
    binding: &impl Fn(u32) -> ResidentRegion,
    value: &impl Fn(crate::ComprehensionValue) -> Result<ResidentReadLocation, ResidentActivationError>,
) -> Result<crate::CollectionPattern<ResidentRegion, ResidentReadLocation>, ResidentActivationError>
{
    Ok(match pattern {
        crate::CollectionPattern::Wildcard => crate::CollectionPattern::Wildcard,
        crate::CollectionPattern::Bind { local, .. } => crate::CollectionPattern::Bind {
            local: *local,
            schema: binding(*local),
        },
        crate::CollectionPattern::Equal(peer) => crate::CollectionPattern::Equal(value(*peer)?),
        crate::CollectionPattern::Tuple(items) => crate::CollectionPattern::Tuple(
            items
                .iter()
                .map(|item| activate_pattern(item, binding, value))
                .collect::<Result<_, _>>()?,
        ),
        crate::CollectionPattern::Array {
            prefix,
            rest,
            suffix,
        } => crate::CollectionPattern::Array {
            prefix: prefix
                .iter()
                .map(|item| activate_pattern(item, binding, value))
                .collect::<Result<_, _>>()?,
            rest: rest
                .as_deref()
                .map(|item| activate_pattern(item, binding, value).map(Box::new))
                .transpose()?,
            suffix: suffix
                .iter()
                .map(|item| activate_pattern(item, binding, value))
                .collect::<Result<_, _>>()?,
        },
    })
}

type ControlCall = (
    NodeId,
    crate::memory_planner::CallSiteMemoryTemplate,
    mech_core::CallMemoryPlan,
);

pub(super) fn bind(
    artifact: &ProgramArtifact,
    catalog: &FunctionCatalog,
    owner: NodeId,
    control: &crate::ComprehensionDeclaration,
    layout: &LayoutBuild,
    steps: &mut Vec<ActivatedTurnStep>,
    reads: &mut Vec<ResidentReadLocation>,
    calls: &mut Vec<ControlCall>,
) -> Result<
    (
        Box<[ActivatedCollectionStep]>,
        Box<[ResidentRegion]>,
        ResidentReadLocation,
        mech_core::CallMemoryPlan,
    ),
    ResidentActivationError,
> {
    let unsupported = || ResidentActivationError::UnsupportedControlLayout { node: owner };
    let captures = node_inputs(artifact, owner)?;
    let output_slot = node_output_slot(artifact, owner)?;
    let output = slot_port_layout(&layout.slots[output_slot.get() as usize]);
    if output.kind != ResidentValueKind::Snapshot
        || !artifact
            .schemas()
            .get(output.schema_id)
            .is_some_and(|schema| match schema.body() {
                SchemaBody::Matrix { element, .. } | SchemaBody::Set { element, .. } => {
                    primitive(element)
                }
                _ => false,
            })
    {
        return Err(unsupported());
    }
    let source = |value: crate::ComprehensionValue| match value {
        crate::ComprehensionValue::Constant(id) => ArtifactSource::Constant(id),
        crate::ComprehensionValue::Input(ordinal) => captures[ordinal as usize],
        crate::ComprehensionValue::Local(local) => {
            ArtifactSource::Slot(layout.control_locals[&(owner, 0, local)].0)
        }
    };
    let port = |source: ArtifactSource| -> Result<ResidentPortLayout, ResidentActivationError> {
        match source {
            ArtifactSource::Slot(slot) => Ok(slot_port_layout(&layout.slots[slot.get() as usize])),
            ArtifactSource::Constant(id) => {
                let value = artifact.constants().get(id).unwrap();
                let region = layout.constant_regions[id.get() as usize];
                Ok(ResidentPortLayout {
                    kind: region.kind,
                    shape: region.shape,
                    schema_id: value.schema(),
                    schema_key: value.schema_key(),
                    shape_instance: value.shape().clone(),
                    activation_fixed_shape: true,
                    resolved_selector: ArtifactStaticSelectorResolver::new(artifact)
                        .resolve(artifact, source)?,
                })
            }
        }
    };
    let mut instructions = Vec::new();
    for step in &control.steps {
        match step {
            crate::ComprehensionStep::Generator {
                source: value,
                pattern,
            } => {
                let source = source(*value);
                let input = port(source)?;
                if !supported_pattern(pattern, artifact.schemas())
                    || !artifact
                        .schemas()
                        .get(input.schema_id)
                        .is_some_and(|schema| match schema.body() {
                            SchemaBody::Matrix { element, .. }
                            | SchemaBody::Set { element, .. } => supported_element(element),
                            _ => false,
                        })
                {
                    return Err(unsupported());
                }
                let pattern = activate_pattern(
                    pattern,
                    &|local| {
                        layout.slots[layout.control_locals[&(owner, 0, local)].0.get() as usize]
                            .region
                    },
                    &|value| {
                        let source = source_for_value(value, &captures, owner, layout);
                        let peer = port(source)?;
                        let schema = artifact.schemas().get(peer.schema_id).unwrap();
                        if !primitive(schema.body())
                            && !matches!(schema.body(), SchemaBody::Atom(_))
                        {
                            return Err(unsupported());
                        }
                        resolve_read(layout, source)
                    },
                )?;
                instructions.push(ActivatedCollectionStep::Generator {
                    source: resolve_read(layout, source)?,
                    pattern,
                });
            }
            crate::ComprehensionStep::Filter(value) => instructions.push(
                ActivatedCollectionStep::Filter(resolve_read(layout, source(*value))?),
            ),
            crate::ComprehensionStep::Operation(operation) => {
                let (output_slot, memory_node) =
                    layout.control_locals[&(owner, 0, operation.local)];
                let output = &layout.slots[output_slot.get() as usize];
                let input_sources = operation
                    .inputs
                    .iter()
                    .copied()
                    .map(source)
                    .collect::<Vec<_>>();
                let input_layouts = input_sources
                    .iter()
                    .copied()
                    .map(port)
                    .collect::<Result<Vec<_>, _>>()?;
                let (kernel, memory_plan) = bind_resident_operation(
                    artifact,
                    catalog,
                    owner,
                    &operation.operation,
                    operation.contract,
                    &input_layouts,
                    slot_port_layout(output),
                )?;
                calls.push((
                    owner,
                    crate::memory_planner::CallSiteMemoryTemplate {
                        node: memory_node,
                        input_sources: input_sources.clone().into_boxed_slice(),
                        output_slots: vec![output_slot].into_boxed_slice(),
                    },
                    memory_plan,
                ));
                let start = reads.len() as u32;
                for input in input_sources {
                    reads.push(resolve_read(layout, input)?);
                }
                let mech_core::ResolvedOperationContract::Declared(contract) =
                    artifact.contracts().get(operation.contract).unwrap()
                else {
                    unreachable!()
                };
                let policy = &contract.outputs[0];
                let index = ActivatedNodeIndex(steps.len() as u32);
                steps.push(ActivatedTurnStep::Kernel(ActivatedKernelNode {
                    artifact_node: owner,
                    memory_node,
                    reads: start..reads.len() as u32,
                    write: ResidentWriteLocation {
                        slot: output_slot,
                        storage: ResidentStorageClass::Scratch,
                        region: output.region,
                    },
                    construction: policy.construction.clone(),
                    rmw_base: None,
                    rmw_previous: None,
                    change_detection: policy.change_detection,
                    reads_state: reads[start as usize..]
                        .iter()
                        .any(|read| matches!(read, ResidentReadLocation::State { .. })),
                    scratch_prefix_reads: false,
                    kernel,
                }));
                // Dense fixed kernels do not all create materialization permits.
                // Qualify their bounded work here; repeated materializing calls
                // additionally contribute their own admitted costs through R5.
                let name = operation.operation.canonical_name();
                let scalar = primitive(artifact.schemas().get(output.schema).unwrap().body())
                    && input_layouts.iter().all(|port| {
                        primitive(artifact.schemas().get(port.schema_id).unwrap().body())
                    });
                let fixed_scalar = scalar
                    && (mech_core::maintained_math_operation(&name).is_some()
                        && !matches!(name.as_str(), "math/bessel/jn" | "math/bessel/yn")
                        || matches!(
                            name.as_str(),
                            "compare/eq"
                                | "compare/neq"
                                | "compare/lt"
                                | "compare/lte"
                                | "compare/gt"
                                | "compare/gte"
                                | "compare/min"
                                | "compare/max"
                                | "logic/and"
                                | "logic/or"
                                | "logic/xor"
                                | "logic/not"
                                | "access/index"
                        ));
                let elements = input_layouts
                    .iter()
                    .try_fold(1_u64, |total, input| {
                        total.checked_add(input.shape.len()? as u64)
                    })
                    .ok_or_else(unsupported)?;
                let work = if fixed_scalar {
                    256
                } else if matches!(name.as_str(), "stats/sum/column" | "stats/sum/row") {
                    // Reduction kernels admit their complete scan before execution.
                    0
                } else if matches!(
                    name.as_str(),
                    "matrix/horzcat" | "matrix/vertcat" | "access/scalar"
                ) && output.region.kind != ResidentValueKind::Snapshot
                    && input_layouts.iter().all(|port| {
                        matches!(
                            port.kind,
                            ResidentValueKind::Bool
                                | ResidentValueKind::Index
                                | ResidentValueKind::F64
                        )
                    })
                {
                    elements.checked_mul(64).ok_or_else(unsupported)?
                } else if name == "set/define"
                    && input_layouts.iter().all(|port| {
                        primitive(artifact.schemas().get(port.schema_id).unwrap().body())
                    })
                {
                    elements
                        .checked_mul(elements)
                        .and_then(|work| work.checked_mul(64))
                        .ok_or_else(unsupported)?
                } else {
                    return Err(unsupported());
                };
                instructions.push(ActivatedCollectionStep::Operation { node: index, work });
            }
        }
    }
    let locals = locals(control)
        .iter()
        .enumerate()
        .map(|(local, _)| {
            layout.slots[layout.control_locals[&(owner, 0, local as u32)].0.get() as usize].region
        })
        .collect();
    let inputs = captures
        .iter()
        .copied()
        .map(port)
        .collect::<Result<Vec<_>, _>>()?;
    let memory = materialization_memory(artifact, control.kind, &inputs, &output)?;
    Ok((
        instructions.into_boxed_slice(),
        locals,
        resolve_read(layout, source(control.yield_value))?,
        memory,
    ))
}

fn source_for_value(
    value: crate::ComprehensionValue,
    captures: &[ArtifactSource],
    owner: NodeId,
    layout: &LayoutBuild,
) -> ArtifactSource {
    match value {
        crate::ComprehensionValue::Constant(id) => ArtifactSource::Constant(id),
        crate::ComprehensionValue::Input(ordinal) => captures[ordinal as usize],
        crate::ComprehensionValue::Local(local) => {
            ArtifactSource::Slot(layout.control_locals[&(owner, 0, local)].0)
        }
    }
}

/// R5 qualifies the actual resident control executor's materialization. This
/// target binding is derived from the validated typed declaration; it is not
/// an ordinary artifact node or a replacement semantic contract for its body.
fn materialization_memory(
    artifact: &ProgramArtifact,
    kind: crate::ComprehensionKind,
    inputs: &[ResidentPortLayout],
    output: &ResidentPortLayout,
) -> Result<mech_core::CallMemoryPlan, ResidentActivationError> {
    let name = match kind {
        crate::ComprehensionKind::Matrix => "matrix-comprehension",
        crate::ComprehensionKind::Set => "set-comprehension",
    };
    let declaration = mech_core::OperationContractDeclaration {
        inputs: mech_core::InputPortLayout::Fixed(
            vec![
                mech_core::InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal
                };
                inputs.len()
            ]
            .into_boxed_slice(),
        ),
        outputs: vec![mech_core::OutputPortPolicy {
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: materialization_construction(),
            alias: mech_core::AliasPolicy::NoAlias,
            change_detection: ChangeDetectionPolicy::KernelReported,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    };
    let descriptor = |port: &ResidentPortLayout| {
        mech_core::ResolvedValueDescriptor::from_schema(
            artifact.schemas().get(port.schema_id).unwrap().clone(),
            port.shape_instance.clone(),
        )
        .map_err(|_| ResidentActivationError::InvalidSnapshotRepresentation)
    };
    let operation = mech_core::ResolvedOperationDescriptor::from_name(
        format!("control/{name}"),
        declaration.clone(),
    )
    .map_err(|_| ResidentActivationError::InvalidSnapshotRepresentation)?;
    let call = BoundCall::artifact_operation(
        operation,
        inputs
            .iter()
            .map(descriptor)
            .collect::<Result<Box<[_]>, _>>()?,
        vec![descriptor(output)?].into_boxed_slice(),
        ResidentOperationKey::new(
            vec!["control".to_owned()].into_boxed_slice(),
            name.to_owned(),
        )
        .unwrap(),
    )
    .map_err(|_| ResidentActivationError::InvalidSnapshotRepresentation)?;
    let policy = &declaration.outputs[0];
    let resolved = mech_core::ResolvedOutputPort {
        schema: output.schema_id,
        access: policy.access,
        delivery: policy.delivery,
        construction: policy.construction.clone(),
        alias: policy.alias,
        change_detection: policy.change_detection,
    };
    resident_call_memory_plan(
        &call,
        inputs,
        output,
        &resolved,
        ImplementationMemoryClass::CanonicalFinalize,
    )
}

pub(super) fn materialization_construction() -> OutputConstruction {
    OutputConstruction::FullWrite {
        shape: ShapeRule::Declared,
    }
}
