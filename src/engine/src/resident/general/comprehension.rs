use super::*;

#[derive(Clone, Debug)]
pub enum ActivatedCollectionStep {
    Generator {
        source: ResidentReadLocation,
        source_schema: SchemaId,
        /// Wildcard generators never materialize or descend into an element,
        /// so they do not require the element component to have its own
        /// retained schema-table entry.
        element_schema: Option<SchemaId>,
        shape_values: Box<[u64]>,
        pattern: crate::CollectionPattern<ActivatedPatternBinding, ActivatedPatternValue>,
    },
    Operation {
        node: ActivatedNodeIndex,
        work: u64,
    },
    Filter(ResidentReadLocation),
}

#[derive(Clone, Copy, Debug)]
pub struct ActivatedPatternBinding {
    pub region: ResidentRegion,
    pub schema: SchemaId,
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
    /// Constant-backed values referenced by the lexical declaration do not
    /// appear in the artifact node's capture list. Retain their locations so
    /// execution can import every schema owner reachable from the actual
    /// comprehension inputs.
    pub schema_reads: Box<[ResidentReadLocation]>,
    pub yield_value: ResidentReadLocation,
    pub yield_schema: SchemaId,
}

pub(super) fn local_definitions(
    control: &crate::ComprehensionDeclaration,
) -> Vec<(bool, SchemaId)> {
    let mut locals = Vec::new();
    for step in &control.steps {
        match step {
            crate::ComprehensionStep::Generator { pattern, .. } => {
                pattern.bindings(&mut |local, schema| {
                    assert_eq!(local as usize, locals.len(), "validated collection locals");
                    locals.push((true, *schema));
                })
            }
            crate::ComprehensionStep::Operation(operation) => {
                locals.push((false, operation.schema));
            }
            crate::ComprehensionStep::Filter(_) => {}
        }
    }
    locals
}

pub(super) fn locals(control: &crate::ComprehensionDeclaration) -> Vec<SchemaId> {
    local_definitions(control)
        .into_iter()
        .map(|(_, schema)| schema)
        .collect()
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

fn canonical_component_schema(
    parent: &mech_core::Schema,
    body: &SchemaBody,
) -> Option<mech_core::Schema> {
    parent.canonical_component_schema(body).ok()
}

pub(super) fn canonical_component_schema_id(
    parent: &mech_core::Schema,
    body: &SchemaBody,
    schemas: &mech_core::SchemaTable,
) -> Option<SchemaId> {
    schemas.find_by_key(canonical_component_schema(parent, body)?.key())
}

fn pattern_components_addressable(
    pattern: &crate::CollectionPattern,
    schema: SchemaId,
    schemas: &mech_core::SchemaTable,
) -> bool {
    let Some(parent) = schemas.get(schema) else {
        return false;
    };
    match pattern {
        // These forms consume the value already selected by their parent.
        crate::CollectionPattern::Wildcard
        | crate::CollectionPattern::Bind { .. }
        | crate::CollectionPattern::Equal(_) => true,
        crate::CollectionPattern::Tuple(patterns) => {
            let SchemaBody::Tuple(items) = parent.body() else {
                // Shape mismatch is an ordinary runtime nonmatch. There is no
                // structural path to preflight in this schema.
                return true;
            };
            patterns.iter().enumerate().all(|(index, pattern)| {
                if matches!(pattern, crate::CollectionPattern::Wildcard) {
                    return true;
                }
                let Some(body) = items.get(index) else {
                    return true;
                };
                canonical_component_schema_id(parent, body, schemas)
                    .is_some_and(|schema| pattern_components_addressable(pattern, schema, schemas))
            })
        }
        crate::CollectionPattern::Array { prefix, suffix, .. } => {
            let SchemaBody::Matrix { element, .. } = parent.body() else {
                return true;
            };
            prefix.iter().chain(suffix.iter()).all(|pattern| {
                if matches!(pattern, crate::CollectionPattern::Wildcard) {
                    return true;
                }
                canonical_component_schema_id(parent, element, schemas)
                    .is_some_and(|schema| pattern_components_addressable(pattern, schema, schemas))
            })
        }
    }
}
fn supported_pattern_inner(
    pattern: &crate::CollectionPattern,
    schemas: &mech_core::SchemaTable,
) -> bool {
    match pattern {
        crate::CollectionPattern::Wildcard | crate::CollectionPattern::Equal(_) => true,
        crate::CollectionPattern::Bind { schema, .. } => schemas.get(*schema).is_some(),
        crate::CollectionPattern::Tuple(items) => items
            .iter()
            .all(|item| supported_pattern_inner(item, schemas)),
        crate::CollectionPattern::Array {
            prefix,
            rest,
            suffix,
        } => {
            prefix
                .iter()
                .chain(suffix.iter())
                .all(|item| supported_pattern_inner(item, schemas))
                && rest
                    .as_deref()
                    .is_none_or(|rest| supported_pattern_inner(rest, schemas))
        }
    }
}

fn supported_pattern(
    pattern: &crate::CollectionPattern,
    element_schema: SchemaId,
    schemas: &mech_core::SchemaTable,
) -> bool {
    supported_pattern_inner(pattern, schemas)
        && pattern_components_addressable(pattern, element_schema, schemas)
}

fn generator_element_schema(
    pattern: &crate::CollectionPattern,
    source: &mech_core::Schema,
    schemas: &mech_core::SchemaTable,
) -> Option<Option<SchemaId>> {
    let element = match source.body() {
        SchemaBody::Matrix { element, .. } | SchemaBody::Set { element, .. } => element.as_ref(),
        _ => return None,
    };
    if matches!(pattern, crate::CollectionPattern::Wildcard) {
        return Some(None);
    }
    let element_schema = canonical_component_schema_id(source, element, schemas)?;
    supported_pattern(pattern, element_schema, schemas).then_some(Some(element_schema))
}

fn schema_adapting_pattern(
    pattern: &crate::CollectionPattern<ActivatedPatternBinding, ActivatedPatternValue>,
    schemas: &mech_core::SchemaTable,
) -> bool {
    let composite = |schema| {
        schemas.get(schema).is_some_and(|schema| {
            matches!(
                schema.body(),
                SchemaBody::Tuple(_) | SchemaBody::Matrix { .. }
            )
        })
    };
    match pattern {
        crate::CollectionPattern::Tuple(_) | crate::CollectionPattern::Array { .. } => true,
        crate::CollectionPattern::Bind { schema, .. } => composite(schema.schema),
        crate::CollectionPattern::Equal(value) => composite(value.schema),
        crate::CollectionPattern::Wildcard => false,
    }
}

pub(super) fn uses_structural_patterns(
    control: &ActivatedComprehensionNode,
    schemas: &mech_core::SchemaTable,
) -> bool {
    control.steps.iter().any(|step| {
        matches!(
            step,
            ActivatedCollectionStep::Generator { pattern, .. }
                if schema_adapting_pattern(pattern, schemas)
        )
    })
}

fn visit_pattern_values(
    pattern: &crate::CollectionPattern,
    visit: &mut impl FnMut(crate::ComprehensionValue),
) {
    match pattern {
        crate::CollectionPattern::Equal(value) => visit(*value),
        crate::CollectionPattern::Tuple(items) => {
            for item in items {
                visit_pattern_values(item, visit);
            }
        }
        crate::CollectionPattern::Array {
            prefix,
            rest,
            suffix,
        } => {
            for item in prefix
                .iter()
                .chain(rest.iter().map(Box::as_ref))
                .chain(suffix.iter())
            {
                visit_pattern_values(item, visit);
            }
        }
        crate::CollectionPattern::Wildcard | crate::CollectionPattern::Bind { .. } => {}
    }
}

fn comprehension_constant_ids(
    control: &crate::ComprehensionDeclaration,
) -> Vec<mech_core::ConstantId> {
    let mut constants = Vec::new();
    let mut visit = |value: crate::ComprehensionValue| {
        if let crate::ComprehensionValue::Constant(id) = value
            && !constants.contains(&id)
        {
            constants.push(id);
        }
    };
    for step in &control.steps {
        match step {
            crate::ComprehensionStep::Generator { source, pattern } => {
                visit(*source);
                visit_pattern_values(pattern, &mut visit);
            }
            crate::ComprehensionStep::Operation(operation) => {
                operation.inputs.iter().copied().for_each(&mut visit);
            }
            crate::ComprehensionStep::Filter(value) => visit(*value),
        }
    }
    visit(control.yield_value);
    constants
}

fn activate_pattern(
    pattern: &crate::CollectionPattern,
    binding: &impl Fn(u32, SchemaId) -> ActivatedPatternBinding,
    value: &impl Fn(crate::ComprehensionValue) -> Result<ActivatedPatternValue, ResidentActivationError>,
) -> Result<
    crate::CollectionPattern<ActivatedPatternBinding, ActivatedPatternValue>,
    ResidentActivationError,
> {
    Ok(match pattern {
        crate::CollectionPattern::Wildcard => crate::CollectionPattern::Wildcard,
        crate::CollectionPattern::Bind { local, schema } => crate::CollectionPattern::Bind {
            local: *local,
            schema: binding(*local, *schema),
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
        Box<[ResidentReadLocation]>,
        ResidentReadLocation,
        SchemaId,
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
                SchemaBody::Matrix { .. } => true,
                SchemaBody::Set { element, .. } => primitive(element),
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
                let Some(source_schema) = artifact.schemas().get(input.schema_id) else {
                    return Err(unsupported());
                };
                let Some(element_schema) =
                    generator_element_schema(pattern, source_schema, artifact.schemas())
                else {
                    return Err(unsupported());
                };
                let pattern = activate_pattern(
                    pattern,
                    &|local, schema| ActivatedPatternBinding {
                        region: layout.slots
                            [layout.control_locals[&(owner, 0, local)].0.get() as usize]
                            .region,
                        schema,
                    },
                    &|value| {
                        let source = source_for_value(value, &captures, owner, layout);
                        let peer = port(source)?;
                        Ok(ActivatedPatternValue {
                            location: resolve_read(layout, source)?,
                            schema: peer.schema_id,
                        })
                    },
                )?;
                instructions.push(ActivatedCollectionStep::Generator {
                    source: resolve_read(layout, source)?,
                    source_schema: input.schema_id,
                    element_schema,
                    shape_values: input
                        .shape_instance
                        .parameter_values()
                        .to_vec()
                        .into_boxed_slice(),
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
                let work = if output.region.kind == ResidentValueKind::Snapshot {
                    // Snapshot-producing kernels carry their complete retained
                    // and transient demand in the bound call plan. The control
                    // meter owns only the repeated dispatch overhead here.
                    0
                } else if fixed_scalar {
                    256
                } else if matches!(name.as_str(), "stats/sum/column" | "stats/sum/row") {
                    // Reduction kernels admit their complete scan before execution.
                    0
                } else if name == "access/scalar"
                    && input_layouts
                        .first()
                        .is_some_and(|port| port.kind == ResidentValueKind::Snapshot)
                {
                    // Turn-shaped pattern bindings live in canonical snapshot
                    // storage. The managed access kernel and its bound call
                    // plan own selection materialization and admission.
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
    let yielded = port(source(control.yield_value))?;
    let schema_reads = comprehension_constant_ids(control)
        .into_iter()
        .map(|constant| resolve_read(layout, ArtifactSource::Constant(constant)))
        .collect::<Result<Vec<_>, _>>()?;
    Ok((
        instructions.into_boxed_slice(),
        locals,
        schema_reads.into_boxed_slice(),
        resolve_read(layout, source(control.yield_value))?,
        yielded.schema_id,
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

#[cfg(test)]
mod tests {
    use super::*;
    use mech_core::{
        DimensionExpr, DimensionLifetime, DimensionParameterDeclaration, DimensionParameterId,
        DimensionParameterOrigin, FloatWidth, SchemaDraft, SchemaTableBuilder,
    };

    fn parameter(id: u32) -> DimensionParameterDeclaration {
        DimensionParameterDeclaration {
            id: DimensionParameterId::new(id),
            origin: DimensionParameterOrigin::Explicit,
            lifetime: DimensionLifetime::Turn,
            lower_bound: DimensionExpr::Constant(0),
            upper_bound: Some(DimensionExpr::Constant(8)),
        }
    }

    #[test]
    fn root_composite_bindings_and_equalities_request_projection_planning() {
        let mut builder = SchemaTableBuilder::new();
        let tuple = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Tuple(
                        vec![SchemaBody::Dynamic, SchemaBody::Dynamic].into_boxed_slice(),
                    ),
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let scalar = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::FloatingPoint(FloatWidth::W64),
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let build = builder.finish().unwrap();
        let tuple = build.resolve(tuple).unwrap();
        let scalar = build.resolve(scalar).unwrap();
        let region = ResidentRegion {
            kind: ResidentValueKind::Snapshot,
            offset: 0,
            len: 1,
            shape: mech_core::ResidentShape::SCALAR,
        };
        let binding = |schema| crate::CollectionPattern::Bind {
            local: 0,
            schema: ActivatedPatternBinding { region, schema },
        };
        let equal = crate::CollectionPattern::Equal(ActivatedPatternValue {
            location: ResidentReadLocation::Constant(region),
            schema: tuple,
        });

        assert!(schema_adapting_pattern(&binding(tuple), &build.table));
        assert!(schema_adapting_pattern(&equal, &build.table));
        assert!(!schema_adapting_pattern(&binding(scalar), &build.table));
    }

    #[test]
    fn component_addressability_uses_canonical_parameter_numbering() {
        let component = |parameter| SchemaBody::Matrix {
            element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
            dimensions: vec![
                DimensionExpr::Constant(1),
                DimensionExpr::Parameter(DimensionParameterId::new(parameter)),
            ]
            .into_boxed_slice(),
        };
        let mut builder = SchemaTableBuilder::new();
        let root = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Tuple(vec![component(0), component(1)].into_boxed_slice()),
                    dimension_parameters: vec![parameter(0), parameter(1)].into_boxed_slice(),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        builder
            .insert(
                SchemaDraft {
                    body: component(1),
                    dimension_parameters: vec![parameter(0), parameter(1)].into_boxed_slice(),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::FloatingPoint(FloatWidth::W64),
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let build = builder.finish().unwrap();
        let root = build.resolve(root).unwrap();
        let schemas = build.table;
        let pattern = crate::CollectionPattern::Tuple(
            vec![
                crate::CollectionPattern::Wildcard,
                crate::CollectionPattern::Bind {
                    local: 0,
                    schema: root,
                },
            ]
            .into_boxed_slice(),
        );

        let tuple = schemas.get(root).unwrap();
        let SchemaBody::Tuple(components) = tuple.body() else {
            panic!("first schema is the tuple witness")
        };
        assert!(
            components
                .iter()
                .all(|body| canonical_component_schema_id(tuple, body, &schemas).is_some())
        );
        assert!(pattern_components_addressable(&pattern, root, &schemas));
    }

    #[test]
    fn structural_pattern_rejects_an_unaddressable_nested_component() {
        let mut builder = SchemaTableBuilder::new();
        let root = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Tuple(
                        vec![SchemaBody::Tuple(Box::new([]))].into_boxed_slice(),
                    ),
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let build = builder.finish().unwrap();
        let root = build.resolve(root).unwrap();
        let schemas = build.table;
        let pattern = crate::CollectionPattern::Tuple(
            vec![crate::CollectionPattern::Tuple(Box::new([]))].into_boxed_slice(),
        );

        assert!(!pattern_components_addressable(&pattern, root, &schemas));
        assert!(!supported_pattern(&pattern, root, &schemas));
    }

    #[test]
    fn wildcard_generator_does_not_require_a_retained_element_schema() {
        let mut builder = SchemaTableBuilder::new();
        let source = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Matrix {
                        element: Box::new(SchemaBody::Tuple(
                            vec![SchemaBody::Bool].into_boxed_slice(),
                        )),
                        dimensions: vec![DimensionExpr::Constant(1), DimensionExpr::Constant(1)]
                            .into_boxed_slice(),
                    },
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let build = builder.finish().unwrap();
        let source = build.resolve(source).unwrap();
        let schemas = build.table;
        let source = schemas.get(source).unwrap();

        assert_eq!(
            generator_element_schema(&crate::CollectionPattern::Wildcard, source, &schemas),
            Some(None),
        );
        assert_eq!(
            generator_element_schema(
                &crate::CollectionPattern::Tuple(
                    vec![crate::CollectionPattern::Wildcard].into_boxed_slice(),
                ),
                source,
                &schemas,
            ),
            None,
        );
    }

    #[test]
    fn pattern_addressability_ignores_unrelated_incomplete_schemas() {
        let mut builder = SchemaTableBuilder::new();
        let root = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Tuple(vec![SchemaBody::Bool].into_boxed_slice()),
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let boolean = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Bool,
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Tuple(
                        vec![SchemaBody::Tuple(Box::new([]))].into_boxed_slice(),
                    ),
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let build = builder.finish().unwrap();
        let root = build.resolve(root).unwrap();
        let boolean = build.resolve(boolean).unwrap();
        let schemas = build.table;
        let pattern = crate::CollectionPattern::Tuple(
            vec![crate::CollectionPattern::Bind {
                local: 0,
                schema: boolean,
            }]
            .into_boxed_slice(),
        );

        assert!(supported_pattern(&pattern, root, &schemas));
    }

    #[test]
    fn comprehension_schema_sources_include_constant_generator_and_yield() {
        let generator = mech_core::ConstantId::new(3);
        let peer = mech_core::ConstantId::new(5);
        let yielded = mech_core::ConstantId::new(7);
        let control = crate::ComprehensionDeclaration {
            kind: crate::ComprehensionKind::Matrix,
            steps: vec![crate::ComprehensionStep::Generator {
                source: crate::ComprehensionValue::Constant(generator),
                pattern: crate::CollectionPattern::Equal(crate::ComprehensionValue::Constant(peer)),
            }]
            .into_boxed_slice(),
            yield_value: crate::ComprehensionValue::Constant(yielded),
        };

        assert_eq!(
            comprehension_constant_ids(&control),
            vec![generator, peer, yielded]
        );
    }
}
