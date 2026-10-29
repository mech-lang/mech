//! Private tuple/record frames with native scalar leaves and admitted managed owners.
//! The complete connected component must have only pack/copy/project uses.
use super::{
    NativeCollectionPrimitive as Primitive, NativeControlLoop as Loop,
    NativeIterationAction as Action, NativeMatchAction as Scalar, NativeMatchValue as Value,
    NativeProgramArtifactPlan as Plan, NativeProgramFacts as Facts,
    NativeProgramProducerRole as Producer, NativeProgramReadRole as Read,
    NativeStructuralObservation as Observation, NativeStructuralStep as Step,
};
use mech_core::{IntegerWidth, SchemaBody, SchemaId};

#[derive(Clone, Debug)]
pub enum NativeBufferFrameOperation {
    Pack {
        output: Value,
    },
    Copy {
        source: Value,
        output: Value,
    },
    Project {
        source: Value,
        ordinal: usize,
        output: Value,
    },
}
#[derive(Clone, Debug)]
pub struct NativeBufferFrameAction {
    operation: NativeBufferFrameOperation,
    retire: Box<[Value]>,
}
impl NativeBufferFrameAction {
    pub fn operation(&self) -> &NativeBufferFrameOperation {
        &self.operation
    }
    pub fn retire(&self) -> &[Value] {
        &self.retire
    }
}
/// An immutable field recipe derived from exact same-component leaf schemas.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeBufferFrameLane {
    Scalar {
        at: usize,
        kind: super::NativeScalarKind,
    },
    Managed {
        at: usize,
    },
    /// A structural span in this same layout, never a canonical owner lane.
    Composite {
        at: usize,
        count: usize,
    },
}
#[derive(Clone, Copy, Debug)]
pub struct NativeBufferFrameField {
    schema: SchemaId,
    lane: NativeBufferFrameLane,
}
impl NativeBufferFrameField {
    pub fn schema(&self) -> SchemaId {
        self.schema
    }
    pub fn lane(&self) -> NativeBufferFrameLane {
        self.lane
    }
}
#[derive(Clone, Debug, Default)]
pub struct NativeBufferFrames {
    slots: Box<[Option<(usize, usize)>]>,
    actions: Box<[Option<NativeBufferFrameAction>]>,
    elements: usize,
    fields: Box<[NativeBufferFrameField]>,
    scalar_fields: usize,
    managed_fields: usize,
}
impl NativeBufferFrames {
    pub fn slot(&self, value: Value) -> Option<(usize, usize)> {
        self.slot_at(value.get() as usize)
    }
    pub fn slot_at(&self, slot: usize) -> Option<(usize, usize)> {
        self.slots.get(slot).copied().flatten()
    }
    pub fn action(&self, site: usize) -> Option<&NativeBufferFrameAction> {
        self.actions.get(site).and_then(Option::as_ref)
    }
    pub fn elements(&self) -> usize {
        self.elements
    }
    pub fn scalar_fields(&self) -> usize {
        self.scalar_fields
    }
    pub fn managed_fields(&self) -> usize {
        self.managed_fields
    }
    pub fn fields(&self, value: Value) -> Option<&[NativeBufferFrameField]> {
        let (at, count) = self.slot(value)?;
        self.fields.get(at..at + count)
    }
    pub fn fields_at(&self, value: usize) -> Option<&[NativeBufferFrameField]> {
        let (at, count) = self.slot_at(value)?;
        self.fields.get(at..at + count)
    }
    pub fn children(&self, field: NativeBufferFrameField) -> Option<&[NativeBufferFrameField]> {
        let NativeBufferFrameLane::Composite { at, count } = field.lane else {
            return None;
        };
        self.fields.get(at..at + count)
    }
}

// Composition resolves every output axis directly from current leaf shapes.
// Other shape policies retain the existing opaque/canonical constructor path.
fn composable(plan: &Plan, value: Value) -> bool {
    use mech_core::DimensionExpr as D;
    let definition = plan.values()[value.get() as usize].definition();
    if definition.dimension_parameters().iter().any(|p| {
        p.lifetime() != mech_core::DimensionLifetime::Turn
            || p.lower_bound() != &D::Constant(0)
            || p.upper_bound().is_some()
    }) {
        return false;
    }
    fn visit(body: &SchemaBody, bound: &mut [bool]) -> bool {
        match body {
            SchemaBody::Tuple(fields) => fields.iter().all(|f| visit(f, bound)),
            SchemaBody::Record(fields) => fields.iter().all(|f| visit(&f.schema, bound)),
            SchemaBody::Matrix {
                element,
                dimensions,
            } => {
                // Matrix elements stay opaque; no array-of-record flattening.
                let closed = mech_core::SchemaDraft {
                    body: element.as_ref().clone(),
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .is_ok();
                closed
                    && dimensions.iter().all(|axis| match axis {
                        D::Constant(_) => true,
                        D::Parameter(id) => bound.get_mut(id.get() as usize).is_some_and(|v| {
                            *v = true;
                            true
                        }),
                        _ => false,
                    })
            }
            _ => true,
        }
    }
    let mut bound = vec![false; definition.dimension_parameters().len()];
    visit(definition.body(), &mut bound) && bound.iter().all(|b| *b)
}

/// The leaf shape recipe cannot resolve parameters hidden in an opaque
/// canonical record. Keep that complete use component on the constructor
/// path. Structural descent also checks names and matrix ranks before a
/// private child can become a span; preparation is not allowed to reject a
/// valid program merely because this optional specialization is unsupported.
fn composition_shape_matches(
    expected: &SchemaBody,
    actual: SchemaId,
    definitions: &std::collections::BTreeMap<SchemaId, &mech_core::Schema>,
    layouts: &std::collections::BTreeMap<SchemaId, Vec<SchemaId>>,
    candidates: &std::collections::BTreeSet<SchemaId>,
) -> bool {
    let Some(definition) = definitions.get(&actual) else {
        return false;
    };
    if candidates.contains(&actual) {
        let expected = match (expected, definition.body()) {
            (SchemaBody::Tuple(a), SchemaBody::Tuple(b)) if a.len() == b.len() => {
                a.iter().collect::<Vec<_>>()
            }
            (SchemaBody::Record(a), SchemaBody::Record(b))
                if a.len() == b.len() && a.iter().zip(b).all(|(a, b)| a.name == b.name) =>
            {
                a.iter().map(|f| &f.schema).collect()
            }
            _ => return false,
        };
        let Some(children) = layouts.get(&actual) else {
            return false;
        };
        return children.len() == expected.len()
            && expected
                .into_iter()
                .zip(children)
                .all(|(expected, actual)| {
                    composition_shape_matches(expected, *actual, definitions, layouts, candidates)
                });
    }
    match (expected, definition.body()) {
        (
            SchemaBody::Matrix {
                element: a,
                dimensions: ad,
            },
            SchemaBody::Matrix {
                element: b,
                dimensions: bd,
            },
        ) => a == b && ad.len() == bd.len(),
        _ => definition.dimension_parameters().is_empty() && expected == definition.body(),
    }
}

fn fields(plan: &Plan, value: Value) -> Option<usize> {
    // Activation shape histories retain their ordinary bound-factory owner.
    // A private frame carries only fixed or freshly resolved Turn geometry.
    if plan.values()[value.get() as usize]
        .definition()
        .dimension_parameters()
        .iter()
        .any(|parameter| parameter.lifetime() != mech_core::DimensionLifetime::Turn)
    {
        return None;
    }
    let fields = match plan.values()[value.get() as usize].definition().body() {
        SchemaBody::Tuple(fields) => fields.iter().collect::<Vec<_>>(),
        SchemaBody::Record(fields) => fields.iter().map(|field| &field.schema).collect(),
        _ => return None,
    };
    let mut eligible = false;
    for field in &fields {
        match field {
            SchemaBody::UnsignedInteger(
                IntegerWidth::W8 | IntegerWidth::W16 | IntegerWidth::W32 | IntegerWidth::W64,
            )
            | SchemaBody::SignedInteger(IntegerWidth::W64)
            | SchemaBody::Bool
            | SchemaBody::FloatingPoint(mech_core::FloatWidth::W64) => {}
            SchemaBody::Matrix { element, .. }
                if matches!(
                    element.as_ref(),
                    SchemaBody::UnsignedInteger(
                        IntegerWidth::W8
                            | IntegerWidth::W16
                            | IntegerWidth::W32
                            | IntegerWidth::W64
                    )
                ) =>
            {
                eligible = true
            }
            // Candidate compound siblings may remain opaque canonical Values.
            // The complete shape/use proof below selects structural spans only
            // when the child can stay private through every composition edge.
            SchemaBody::Tuple(_) | SchemaBody::Record(_) => eligible = true,
            // A fixed aggregate matrix remains an admitted immutable Value.
            // It is not a packed buffer, but must not force sibling buffers
            // through canonical snapshot packing and discard their capacity.
            SchemaBody::Matrix {
                element,
                dimensions,
            } if dimensions.len() == 2 && crate::is_control_fixed_aggregate_element(element) => {}
            _ => return None,
        }
    }
    eligible.then_some(fields.len())
}
fn pack(plan: &Plan, action: &Action) -> Option<(Value, Vec<SchemaId>)> {
    let Action::LibraryCall {
        template,
        operands,
        output,
    } = action
    else {
        return None;
    };
    let count = fields(plan, *output)?;
    if template.primitive() != Primitive::CompositePack(count)
        || operands.len() != count
        || template.inputs().len() != count
        || template.output() != plan.values()[output.get() as usize].definition()
        || operands
            .iter()
            .zip(template.inputs())
            .any(|(value, schema)| plan.values()[value.get() as usize].definition() != schema)
    {
        return None;
    }
    Some((
        *output,
        operands
            .iter()
            .map(|v| plan.values()[v.get() as usize].schema())
            .collect(),
    ))
}
fn movement(plan: &Plan, action: &Action) -> Option<(Value, Value)> {
    match action {
        Action::Scalar(Scalar::Copy { source, output }) => Some((*source, *output)),
        Action::Initialize { iteration } | Action::Latch { iteration } => {
            let init = matches!(action, Action::Initialize { .. });
            match &plan.iterations()[*iteration as usize] {
                Loop::Tail(t) => Some((if init { t.seed() } else { t.next() }, t.carrier())),
                Loop::Iteration(t) => Some((if init { t.seed() } else { t.next() }, t.carrier())),
                _ => None,
            }
        }
        _ => None,
    }
}
fn projection(plan: &Plan, action: &Action) -> Option<(Value, usize, Value)> {
    if matches!(action, Action::LibraryCall { .. }) {
        return super::record_projection::constant_field(plan, action);
    }
    let Action::Structural {
        source,
        path,
        observation,
        output,
    } = action
    else {
        return None;
    };
    let [Step::TupleItem { ordinal }] = path.as_ref() else {
        return None;
    };
    matches!(observation, Observation::U64 | Observation::CanonicalValue).then_some((
        *source,
        *ordinal as usize,
        *output,
    ))
}
fn read_site(plan: &Plan, role: Read) -> Option<(u32, bool)> {
    Some(match role {
        Read::Action(site) => (site, false),
        Read::Terminator(site) => (site, true),
        Read::LoopSeed(id) | Read::LoopNext(id) => {
            let seed = matches!(role, Read::LoopSeed(_));
            (
                match &plan.iterations()[id as usize] {
                    Loop::Tail(t) => {
                        if seed {
                            t.initialize()
                        } else {
                            t.latch()
                        }
                    }
                    Loop::Iteration(t) => {
                        if seed {
                            t.initialize()
                        } else {
                            t.latch()
                        }
                    }
                    _ => return None,
                },
                false,
            )
        }
        Read::LoopReturned(id) => (
            match &plan.iterations()[id as usize] {
                Loop::Tail(t) => t.finish(),
                _ => return None,
            },
            true,
        ),
        _ => return None,
    })
}
/// Reads precede writes. A future definition kills only the old visit's owner.
/// Unlike a fusion Read::Other, loop seed/latch roles retain their exact site.
fn last_use(facts: &Facts<'_>, value: Value, frontier: u32) -> bool {
    let Ok(reads) = facts.read_roles(value) else {
        return false;
    };
    let Some(reads) = reads
        .iter()
        .map(|r| read_site(facts.plan(), *r))
        .collect::<Option<Vec<_>>>()
    else {
        return false;
    };
    !facts
        .old_value_read_before_redefinition(
            value,
            frontier,
            |site| reads.contains(&(site, false)),
            |site| reads.contains(&(site, true)),
        )
        .unwrap_or(true)
}

impl Facts<'_> {
    /// Recomputed from the same validated plan on independent image loading.
    pub fn buffer_frames(&self) -> NativeBufferFrames {
        let plan = self.plan();
        let count = plan.values().len();
        let mut bad = (0..count)
            .map(|v| fields(plan, Value(v as u32)).is_none())
            .collect::<Vec<_>>();
        if bad.iter().all(|bad| *bad) {
            return NativeBufferFrames::default();
        }
        let Ok(actions) = self.actions() else {
            return NativeBufferFrames::default();
        };
        let mut edges = vec![Vec::new(); count];
        let mut schemas = vec![None::<Vec<SchemaId>>; count];
        let mut by_schema = std::collections::BTreeMap::new();
        let mut ambiguous = std::collections::BTreeSet::new();
        for action in actions.iter() {
            if let Some((value, fields)) = pack(plan, action) {
                let schema = plan.values()[value.get() as usize].schema();
                if by_schema
                    .insert(schema, fields.clone())
                    .is_some_and(|old| old != fields)
                {
                    ambiguous.insert(schema);
                }
                schemas[value.get() as usize] = Some(fields);
            }
            if let Some((source, output)) = movement(plan, action) {
                edges[source.get() as usize].push(output.get() as usize);
                edges[output.get() as usize].push(source.get() as usize);
            }
        }
        for (i, rejected) in bad.iter_mut().enumerate() {
            if ambiguous.contains(&plan.values()[i].schema()) {
                *rejected = true;
            }
        }
        // Establish one exact leaf schema layout across the whole copy component.
        let mut pending = schemas
            .iter()
            .enumerate()
            .filter_map(|(i, s)| s.is_some().then_some(i))
            .collect::<Vec<_>>();
        while let Some(i) = pending.pop() {
            for &j in &edges[i] {
                if let Some(other) = &schemas[j] {
                    if Some(other) != schemas[i].as_ref() {
                        bad[i] = true;
                        bad[j] = true;
                    }
                } else {
                    schemas[j] = schemas[i].clone();
                    pending.push(j);
                }
            }
        }
        // A projected child has the exact schema of the corresponding pack
        // operand. Its layout is the same recipe, not a newly inferred owner.
        for (i, layout) in schemas.iter_mut().enumerate() {
            if layout.is_none() && !bad[i] {
                *layout = by_schema.get(&plan.values()[i].schema()).cloned();
            }
        }
        let mut composable = (0..count)
            .map(|i| !bad[i] && composable(plan, Value(i as u32)))
            .collect::<Vec<_>>();
        let definitions = plan
            .values()
            .iter()
            .map(|ty| (ty.schema(), ty.definition()))
            .collect::<std::collections::BTreeMap<_, _>>();
        let candidates = (0..count)
            .filter(|&i| composable[i])
            .map(|i| plan.values()[i].schema())
            .filter(|schema| by_schema.contains_key(schema))
            .collect::<std::collections::BTreeSet<_>>();
        for (i, composable) in composable.iter_mut().enumerate() {
            let ty = &plan.values()[i];
            *composable &= composition_shape_matches(
                ty.definition().body(),
                ty.schema(),
                &definitions,
                &by_schema,
                &candidates,
            );
        }
        let mut composition = vec![Vec::new(); count];
        let mut link = |parent: Value, child: Value| {
            let (a, b) = (parent.get() as usize, child.get() as usize);
            if composable[a] && composable[b] && schemas[a].is_some() && schemas[b].is_some() {
                composition[a].push(b);
                composition[b].push(a);
            }
        };
        for action in actions.iter() {
            if let Some((output, _)) = pack(plan, action) {
                let Action::LibraryCall { operands, .. } = action else {
                    unreachable!()
                };
                for operand in operands {
                    link(output, *operand);
                }
            }
            if let Some((source, ordinal, output)) = projection(plan, action) {
                if schemas[source.get() as usize]
                    .as_ref()
                    .and_then(|f| f.get(ordinal))
                    == Some(&plan.values()[output.get() as usize].schema())
                {
                    link(source, output);
                }
            }
        }
        let connected = |parent: Value, child: Value| {
            composition[parent.get() as usize].contains(&(child.get() as usize))
        };
        for i in 0..count {
            if bad[i] {
                continue;
            }
            let value = Value(i as u32);
            let Some(fields) = schemas[i].as_ref() else {
                bad[i] = true;
                continue;
            };
            let Ok(definitions) = self.definition_sites(i as u32) else {
                return NativeBufferFrames::default();
            };
            if definitions.iter().any(|d| match d.role() {
                Producer::Library | Producer::Structural => d.site().is_none_or(|site| {
                    let action = &actions[site as usize];
                    pack(plan, action).is_none()
                        && !projection(plan, action).is_some_and(|(source, _, output)| {
                            output == value && connected(source, output)
                        })
                }),
                Producer::Scalar | Producer::InitializeCarrier | Producer::LatchCarrier => d
                    .site()
                    .is_none_or(|site| movement(plan, &actions[site as usize]).is_none()),
                _ => true,
            }) {
                bad[i] = true;
            }
            let Ok(reads) = self.read_roles(value) else {
                return NativeBufferFrames::default();
            };
            for read in reads {
                let allowed = match *read {
                    Read::Action(site) => {
                        let action = &actions[site as usize];
                        if let Some((source, output)) = movement(plan, action) {
                            source == value
                                && plan.values()[source.get() as usize]
                                    == plan.values()[output.get() as usize]
                        } else if let Some((source, ordinal, output)) = projection(plan, action) {
                            source == value
                                && fields.get(ordinal).is_some_and(|schema| {
                                    *schema == plan.values()[output.get() as usize].schema()
                                })
                        } else if let Some((output, _)) = pack(plan, action) {
                            connected(output, value)
                        } else {
                            false
                        }
                    }
                    Read::LoopSeed(_) | Read::LoopNext(_) => read_site(plan, *read).is_some(),
                    Read::LoopReturned(id) => match &plan.iterations()[id as usize] {
                        // A tail finish only branches to this existing managed
                        // copy. It does not publish a canonical record. Escape
                        // rejection still propagates from the result through
                        // the complete copy component.
                        Loop::Tail(t) => {
                            t.returned() == value
                                && plan.values()[value.get() as usize]
                                    == plan.values()[t.result().get() as usize]
                                && matches!(&actions[t.exit() as usize], Action::Scalar(Scalar::Copy { source, output })
                                if *source == value && *output == t.result())
                        }
                        _ => false,
                    },
                    _ => false,
                };
                if !allowed {
                    bad[i] = true;
                }
            }
        }
        for (copy, composed) in edges.iter_mut().zip(composition) {
            copy.extend(composed);
        }
        let mut pending = bad
            .iter()
            .enumerate()
            .filter_map(|(i, b)| b.then_some(i))
            .collect::<Vec<_>>();
        while let Some(i) = pending.pop() {
            for &j in &edges[i] {
                if !bad[j] {
                    bad[j] = true;
                    pending.push(j);
                }
            }
        }
        let kinds = plan
            .values()
            .iter()
            .map(|ty| (ty.schema(), ty.scalar_kind()))
            .collect::<std::collections::BTreeMap<_, _>>();
        let mut scalar_fields = 0;
        let mut managed_fields = 0;
        let mut field_layout = Vec::new();
        let nested = (0..count)
            .filter(|&i| !bad[i] && composable[i])
            .map(|i| plan.values()[i].schema())
            .collect::<std::collections::BTreeSet<_>>();
        fn layout(
            schemas: &[SchemaId],
            compose: bool,
            nested: &std::collections::BTreeSet<SchemaId>,
            by_schema: &std::collections::BTreeMap<SchemaId, Vec<SchemaId>>,
            kinds: &std::collections::BTreeMap<SchemaId, Option<super::NativeScalarKind>>,
            field_layout: &mut Vec<NativeBufferFrameField>,
            scalar_fields: &mut usize,
            managed_fields: &mut usize,
        ) -> (usize, usize) {
            let start = field_layout.len();
            field_layout.extend(schemas.iter().map(|schema| NativeBufferFrameField {
                schema: *schema,
                lane: NativeBufferFrameLane::Managed { at: usize::MAX },
            }));
            for (index, schema) in schemas.iter().enumerate() {
                let lane = if let Some(kind) = kinds[schema] {
                    let at = *scalar_fields;
                    *scalar_fields += 1;
                    NativeBufferFrameLane::Scalar { at, kind }
                } else if compose && nested.contains(schema) {
                    let (at, count) = layout(
                        &by_schema[schema],
                        true,
                        nested,
                        by_schema,
                        kinds,
                        field_layout,
                        scalar_fields,
                        managed_fields,
                    );
                    NativeBufferFrameLane::Composite { at, count }
                } else {
                    let at = *managed_fields;
                    *managed_fields += 1;
                    NativeBufferFrameLane::Managed { at }
                };
                field_layout[start + index].lane = lane;
            }
            (start, schemas.len())
        }
        let slots = (0..count)
            .map(|i| {
                if bad[i] {
                    return None;
                }
                Some(layout(
                    schemas[i].as_ref()?,
                    composable[i],
                    &nested,
                    &by_schema,
                    &kinds,
                    &mut field_layout,
                    &mut scalar_fields,
                    &mut managed_fields,
                ))
            })
            .collect::<Box<[_]>>();
        let elements = field_layout.len();
        let selected = |value: Value| slots[value.get() as usize].is_some();
        let actions = plan
            .blocks()
            .iter()
            .enumerate()
            .map(|(site, block)| {
                let action = block.action();
                let (operation, reads, output) =
                    if let Some((output, _)) = pack(plan, action).filter(|(v, _)| selected(*v)) {
                        let Action::LibraryCall { operands, .. } = action else {
                            unreachable!()
                        };
                        (
                            NativeBufferFrameOperation::Pack { output },
                            operands.to_vec(),
                            output,
                        )
                    } else if let Some((source, output)) =
                        movement(plan, action).filter(|(_, v)| selected(*v))
                    {
                        (
                            NativeBufferFrameOperation::Copy { source, output },
                            vec![source],
                            output,
                        )
                    } else if let Some((source, ordinal, output)) =
                        projection(plan, action).filter(|(v, _, _)| selected(*v))
                    {
                        (
                            NativeBufferFrameOperation::Project {
                                source,
                                ordinal,
                                output,
                            },
                            vec![source],
                            output,
                        )
                    } else {
                        return None;
                    };
                let retire = reads
                    .into_iter()
                    .filter(|v| {
                        *v != output
                            && plan.values()[v.get() as usize].scalar_kind().is_none()
                            && last_use(self, *v, site as u32)
                    })
                    .collect();
                Some(NativeBufferFrameAction { operation, retire })
            })
            .collect();
        NativeBufferFrames {
            slots,
            actions,
            elements,
            fields: field_layout.into(),
            scalar_fields,
            managed_fields,
        }
    }
}
