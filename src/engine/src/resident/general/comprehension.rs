use super::*;

#[derive(Clone, Debug)]
pub enum ActivatedCollectionStep {
    Generator {
        source: ResidentReadLocation,
        source_schema: SchemaId,
        /// First local owned by this generator or a following step. Discard
        /// these payloads before advancing to the next source element.
        discard_from: u32,
        /// End of this generator's binding locals; later steps extend the
        /// cleanup range only when they actually execute.
        binding_end: u32,
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
        /// Prefix of the control's shared local inventory that is initialized
        /// when this operation executes. Nested matches retain their current
        /// output too because they have no call-local memory contract.
        retained_local_count: u32,
        /// Sorted indices in that prefix which the operation's own call plan
        /// already accounts for. Keeping only compact exclusions avoids a
        /// quadratic retained-region array for large comprehensions.
        excluded_locals: Box<[u32]>,
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
    pub(crate) memory_node: NodeId,
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

pub(super) fn all_local_definitions(
    control: &crate::ComprehensionDeclaration,
) -> Vec<(bool, bool, u32, u32, SchemaId)> {
    fn append_match(
        control: &crate::MatchDeclaration,
        output: &mut Vec<(bool, bool, u32, u32, SchemaId)>,
    ) {
        for arm in &control.arms {
            if let crate::MatchPattern::Structural(pattern) = &arm.pattern {
                pattern.bindings(&mut |local, schema| {
                    output.push((true, true, arm.body.id.0, local, *schema));
                });
            }
            for block in arm.guard.iter().chain(core::iter::once(&arm.body)) {
                for operation in &block.operations {
                    output.push((false, false, block.id.0, operation.node, operation.schema));
                    match &operation.body {
                        crate::ControlOperationBody::Operation { .. }
                        | crate::ControlOperationBody::Recur(_)
                        | crate::ControlOperationBody::Suspend
                        | crate::ControlOperationBody::Publish => {}
                        crate::ControlOperationBody::Match(nested) => append_match(nested, output),
                        crate::ControlOperationBody::Comprehension(nested) => {
                            append_comprehension(nested, output)
                        }
                    }
                }
            }
        }
    }

    fn append_comprehension(
        control: &crate::ComprehensionDeclaration,
        output: &mut Vec<(bool, bool, u32, u32, SchemaId)>,
    ) {
        let mut next_local = 0u32;
        for step in &control.steps {
            match step {
                crate::ComprehensionStep::Generator { pattern, .. } => {
                    pattern.bindings(&mut |local, schema| {
                        assert_eq!(local, next_local, "validated collection locals");
                        output.push((false, true, control.id.0, local, *schema));
                        next_local += 1;
                    });
                }
                crate::ComprehensionStep::Operation(operation) => {
                    assert_eq!(operation.local, next_local, "validated collection locals");
                    output.push((
                        false,
                        matches!(
                            operation.body,
                            crate::ControlOperationBody::Comprehension(_)
                        ),
                        control.id.0,
                        operation.local,
                        operation.schema,
                    ));
                    next_local += 1;
                    match &operation.body {
                        crate::ControlOperationBody::Match(nested) => append_match(nested, output),
                        crate::ControlOperationBody::Comprehension(nested) => {
                            append_comprehension(nested, output)
                        }
                        crate::ControlOperationBody::Operation { .. }
                        | crate::ControlOperationBody::Recur(_)
                        | crate::ControlOperationBody::Suspend
                        | crate::ControlOperationBody::Publish => {}
                    }
                }
                crate::ComprehensionStep::Filter(_) => {}
            }
        }
    }

    let mut output = Vec::new();
    append_comprehension(control, &mut output);
    output
}

pub(super) fn all_match_local_definitions(
    control: &crate::MatchDeclaration,
) -> Vec<(bool, bool, u32, u32, SchemaId)> {
    fn append(
        control: &crate::MatchDeclaration,
        output: &mut Vec<(bool, bool, u32, u32, SchemaId)>,
    ) {
        for arm in &control.arms {
            if let crate::MatchPattern::Structural(pattern) = &arm.pattern {
                pattern.bindings(&mut |local, schema| {
                    output.push((true, true, arm.body.id.0, local, *schema));
                });
            }
            for block in arm.guard.iter().chain(core::iter::once(&arm.body)) {
                for operation in &block.operations {
                    output.push((false, false, block.id.0, operation.node, operation.schema));
                    match &operation.body {
                        crate::ControlOperationBody::Operation { .. }
                        | crate::ControlOperationBody::Recur(_)
                        | crate::ControlOperationBody::Suspend
                        | crate::ControlOperationBody::Publish => {}
                        crate::ControlOperationBody::Match(nested) => append(nested, output),
                        crate::ControlOperationBody::Comprehension(nested) => {
                            output.extend(all_local_definitions(nested));
                        }
                    }
                }
            }
        }
    }
    let mut output = Vec::new();
    append(control, &mut output);
    output
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

fn operation_retained_local_scope(
    operation: &crate::ComprehensionOperation,
) -> Option<(u32, Box<[u32]>)> {
    let nested_match = matches!(operation.body, crate::ControlOperationBody::Match(_));
    let retained_local_count = operation.local.checked_add(u32::from(nested_match))?;
    let excluded_locals = if nested_match {
        Box::new([])
    } else {
        let mut excluded = operation
            .inputs
            .iter()
            .filter_map(|value| match value {
                crate::ComprehensionValue::Local(local) if *local < retained_local_count => {
                    Some(*local)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        excluded.sort_unstable();
        excluded.dedup();
        excluded.into_boxed_slice()
    };
    Some((retained_local_count, excluded_locals))
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
        crate::CollectionPattern::Enum { ordinal, payload } => {
            let enum_parent = match parent.body() {
                SchemaBody::Enum { .. } => parent.clone(),
                SchemaBody::Option(body) if matches!(body.as_ref(), SchemaBody::Enum { .. }) => {
                    let Some(schema) = canonical_component_schema(parent, body) else {
                        return false;
                    };
                    schema
                }
                _ => return true,
            };
            let SchemaBody::Enum { variants, .. } = enum_parent.body() else {
                return true;
            };
             let Some(variant) = variants.get(*ordinal as usize) else {
                 return true;
             };
             match (payload.as_deref(), variant.payload.as_ref()) {
                 (Some(crate::CollectionPattern::Wildcard), Some(_)) => true,
                 (Some(pattern), Some(body)) => {
                     canonical_component_schema_id(&enum_parent, body, schemas).is_some_and(
                         |schema| pattern_components_addressable(pattern, schema, schemas),
                     )
                 }
                 _ => true,
             }
        }
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
        crate::CollectionPattern::Enum { payload, .. } => payload
            .as_deref()
            .is_none_or(|payload| supported_pattern_inner(payload, schemas)),
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
        crate::CollectionPattern::Enum { .. }
        | crate::CollectionPattern::Tuple(_)
        | crate::CollectionPattern::Array { .. } => true,
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
        crate::CollectionPattern::Enum { payload, .. } => {
            if let Some(payload) = payload {
                visit_pattern_values(payload, visit);
            }
        }
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
        crate::CollectionPattern::Enum { ordinal, payload } => crate::CollectionPattern::Enum {
            ordinal: *ordinal,
            payload: payload
                .as_deref()
                .map(|payload| activate_pattern(payload, binding, value).map(Box::new))
                .transpose()?,
        },
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

pub(super) type ControlCall = (
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
    let captures = node_inputs(artifact, owner)?;
    let output_slot = node_output_slot(artifact, owner)?;
    bind_inner(
        artifact,
        catalog,
        owner,
        control,
        &captures,
        output_slot,
        layout,
        steps,
        reads,
        calls,
    )
}

pub(super) fn bind_inner(
    artifact: &ProgramArtifact,
    catalog: &FunctionCatalog,
    owner: NodeId,
    control: &crate::ComprehensionDeclaration,
    captures: &[ArtifactSource],
    output_slot: CellSlotId,
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
            ArtifactSource::Slot(layout.control_locals[&(owner, control.id.0, local)].0)
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
    let locals = locals(control)
        .iter()
        .enumerate()
        .map(|(local, _)| {
            layout.slots[layout.control_locals[&(owner, control.id.0, local as u32)]
                .0
                .get() as usize]
                .region
        })
        .collect::<Box<[_]>>();
    let mut instructions = Vec::new();
    let mut next_local = 0usize;
    for step in &control.steps {
        match step {
            crate::ComprehensionStep::Generator {
                source: value,
                pattern,
            } => {
                let discard_from = u32::try_from(next_local).map_err(|_| unsupported())?;
                pattern.bindings(&mut |local, _| {
                    debug_assert_eq!(local as usize, next_local);
                    next_local += 1;
                });
                let binding_end = u32::try_from(next_local).map_err(|_| unsupported())?;
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
                            [layout.control_locals[&(owner, control.id.0, local)].0.get() as usize]
                            .region,
                        schema,
                    },
                    &|value| {
                        let source = source_for_value(value, captures, owner, control.id.0, layout);
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
                    discard_from,
                    binding_end,
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
                debug_assert_eq!(operation.local as usize, next_local);
                next_local += 1;
                let (output_slot, memory_node) =
                    layout.control_locals[&(owner, control.id.0, operation.local)];
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
                let input_reads = input_sources
                    .iter()
                    .copied()
                    .map(|source| resolve_read(layout, source))
                    .collect::<Result<Vec<_>, _>>()?;
                let (retained_local_count, excluded_locals) =
                    operation_retained_local_scope(operation).ok_or_else(unsupported)?;
                let (reference, contract_id) = match &operation.body {
                    crate::ControlOperationBody::Operation {
                        operation,
                        contract,
                    } => (operation, *contract),
                    crate::ControlOperationBody::Match(nested) => {
                        let index = ActivatedNodeIndex(steps.len() as u32);
                        let prepared = super::prepare_match_node(
                            artifact,
                            owner,
                            memory_node,
                            nested,
                            &input_sources,
                            &input_reads,
                            output_slot,
                            layout,
                        )?;
                        steps.push(ActivatedTurnStep::Match(prepared));
                        let arms = super::bind_match_arms(
                            artifact,
                            catalog,
                            owner,
                            nested,
                            &input_sources,
                            &input_reads,
                            layout,
                            steps,
                            reads,
                            calls,
                            &[index],
                        )?;
                        let ActivatedTurnStep::Match(prepared) = &mut steps[index.get() as usize]
                        else {
                            unreachable!()
                        };
                        prepared.arms = arms;
                        instructions.push(ActivatedCollectionStep::Operation {
                            node: index,
                            work: 0,
                            retained_local_count,
                            excluded_locals,
                        });
                        continue;
                    }
                    crate::ControlOperationBody::Comprehension(nested) => {
                        let index = ActivatedNodeIndex(steps.len() as u32);
                        let start = reads.len() as u32;
                        reads.extend(input_reads.iter().copied());
                        steps.push(ActivatedTurnStep::Comprehension(std::sync::Arc::new(
                            ActivatedComprehensionNode {
                                artifact_node: owner,
                                memory_node,
                                reads: start..reads.len() as u32,
                                write: ResidentWriteLocation {
                                    slot: output_slot,
                                    storage: ResidentStorageClass::Scratch,
                                    region: output.region,
                                },
                                kind: nested.kind,
                                output_schema: output.schema,
                                steps: Box::new([]),
                                locals: Box::new([]),
                                schema_reads: Box::new([]),
                                yield_value: ResidentReadLocation::Scratch(output.region),
                                yield_schema: output.schema,
                            },
                        )));
                        let (
                            nested_steps,
                            nested_locals,
                            nested_schema_reads,
                            yielded,
                            yield_schema,
                            memory,
                        ) = bind_inner(
                            artifact,
                            catalog,
                            owner,
                            nested,
                            &input_sources,
                            output_slot,
                            layout,
                            steps,
                            reads,
                            calls,
                        )?;
                        let ActivatedTurnStep::Comprehension(prepared) =
                            &mut steps[index.get() as usize]
                        else {
                            unreachable!()
                        };
                        let prepared = std::sync::Arc::get_mut(prepared)
                            .expect("unpublished nested collection plan");
                        prepared.steps = nested_steps;
                        prepared.locals = nested_locals;
                        prepared.schema_reads = nested_schema_reads;
                        prepared.yield_value = yielded;
                        prepared.yield_schema = yield_schema;
                        calls.push((
                            owner,
                            crate::memory_planner::CallSiteMemoryTemplate {
                                node: memory_node,
                                input_sources: input_sources.clone().into_boxed_slice(),
                                output_slots: vec![output_slot].into_boxed_slice(),
                            },
                            memory,
                        ));
                        instructions.push(ActivatedCollectionStep::Operation {
                            node: index,
                            work: 0,
                            retained_local_count,
                            excluded_locals,
                        });
                        continue;
                    }
                    crate::ControlOperationBody::Recur(_)
                    | crate::ControlOperationBody::Suspend
                    | crate::ControlOperationBody::Publish => {
                        return Err(ResidentActivationError::UnsupportedControlLayout {
                            node: owner,
                        });
                    }
                };
                let (kernel, memory_plan) = bind_resident_operation(
                    artifact,
                    catalog,
                    owner,
                    reference,
                    contract_id,
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
                reads.extend(input_reads.iter().copied());
                let mech_core::ResolvedOperationContract::Declared(contract) =
                    artifact.contracts().get(contract_id).unwrap()
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
                let name = reference.canonical_name();
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
                } else if name == "convert/kind"
                    && input_layouts.len() == 1
                    && crate::is_control_scalar_schema(
                        artifact.schemas().get(input_layouts[0].schema_id).unwrap(),
                    )
                    && crate::is_control_scalar_schema(
                        artifact.schemas().get(output.schema).unwrap(),
                    )
                {
                    // Scalar conversion takes fixed work even when the source
                    // uses a canonical snapshot lane (for example, u8).
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
                instructions.push(ActivatedCollectionStep::Operation {
                    node: index,
                    work,
                    retained_local_count,
                    excluded_locals,
                });
            }
        }
    }
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
    block: u32,
    layout: &LayoutBuild,
) -> ArtifactSource {
    match value {
        crate::ComprehensionValue::Constant(id) => ArtifactSource::Constant(id),
        crate::ComprehensionValue::Input(ordinal) => captures[ordinal as usize],
        crate::ComprehensionValue::Local(local) => {
            ArtifactSource::Slot(layout.control_locals[&(owner, block, local)].0)
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
        crate::ComprehensionKind::MatrixPreserveShape => "shape-preserving-matrix-comprehension",
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
    fn comprehension_local_inventory_traverses_large_operation_lists_in_source_order() {
        const OPERATION_COUNT: u32 = 4_096;
        let schema = SchemaId::new(0);
        let steps = (0..OPERATION_COUNT)
            .map(|local| {
                crate::ComprehensionStep::Operation(crate::ComprehensionOperation {
                    local,
                    body: crate::ControlOperationBody::Operation {
                        operation: crate::OperationReference {
                            module_path: Box::new([]),
                            operation_name: "identity".to_owned(),
                        },
                        contract: mech_core::OperationContractId::new(0),
                    },
                    inputs: Box::new([]),
                    schema,
                })
            })
            .collect();
        let control = crate::ComprehensionDeclaration {
            id: crate::ControlBlockId(7),
            kind: crate::ComprehensionKind::Matrix,
            steps,
            yield_value: crate::ComprehensionValue::Local(OPERATION_COUNT - 1),
        };

        let definitions = all_local_definitions(&control);
        assert_eq!(definitions.len(), OPERATION_COUNT as usize);
        for (local, definition) in definitions.into_iter().enumerate() {
            assert_eq!(definition, (false, false, 7, local as u32, schema));
        }
        let scopes = control
            .steps
            .iter()
            .map(|step| {
                let crate::ComprehensionStep::Operation(operation) = step else {
                    unreachable!()
                };
                operation_retained_local_scope(operation).unwrap()
            })
            .collect::<Vec<_>>();
        assert_eq!(scopes.len(), OPERATION_COUNT as usize);
        assert!(
            scopes
                .iter()
                .enumerate()
                .all(|(local, (count, excluded))| *count == local as u32 && excluded.is_empty()),
            "each operation retains only a constant-sized view of the shared inventory"
        );
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
            id: crate::ControlBlockId(0),
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
