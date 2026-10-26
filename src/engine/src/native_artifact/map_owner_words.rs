//! Fixed native word transport for CLOSED map-owner candidates. This is not
//! default storage selection. A runtime owner must supply/check opaque tokens
//! at every entry, selected map call and canonical exit before this can run.
use super::{
    NativeAggregateWord as Word, NativeCollectionPrimitive as Primitive,
    NativeIterationAction as Action, NativeMapOwnerRegion, NativeMatchAction as Scalar,
    NativeMatchValue as Value, NativeProgramArtifactPlan as Plan,
    NativeScalarAggregateAction as Transport, NativeScalarAggregates,
    NativeStructuralObservation as Observation, NativeStructuralStep as Step,
};
use mech_core::{IntegerWidth, SchemaBody as Schema, ValueData};

#[derive(Clone, Debug, serde::Serialize)]
pub struct NativeMapOwnerWords {
    regions: Box<[NativeMapOwnerRegion]>,
    aggregates: NativeScalarAggregates,
    initializers: Box<[Option<usize>]>,
    exits: Box<[Option<usize>]>,
    action_regions: Box<[Option<usize>]>,
    call_regions: Box<[Option<usize>]>,
    spans: Box<[NativeMapKeySpan]>,
    span_sites: Box<[Option<usize>]>,
    call_spans: Box<[Option<usize>]>,
    owner_base: usize,
    frame_words: usize,
}
crate::native_artifact::program_image_proofs::image_struct!(NativeMapOwnerWords {
    regions: Box<[NativeMapOwnerRegion]>,
    aggregates: NativeScalarAggregates,
    initializers: Box<[Option<usize>]>,
    exits: Box<[Option<usize>]>,
    action_regions: Box<[Option<usize>]>,
    call_regions: Box<[Option<usize>]>,
    spans: Box<[NativeMapKeySpan]>,
    span_sites: Box<[Option<usize>]>,
    call_spans: Box<[Option<usize>]>,
    owner_base: usize,
    frame_words: usize,
});

/// Non-escaping selected String key. Templates remain at their original plan
/// sites; this proof only selects intermediate storage, never a map operation.
#[derive(Clone, Debug, serde::Serialize)]
pub struct NativeMapKeySpan {
    region: usize,
    range_site: u32,
    selection_site: u32,
    source: Value,
    output: Value,
    operands: (Value, Value),
}
crate::native_artifact::program_image_proofs::image_struct!(NativeMapKeySpan {
    region: usize,
    range_site: u32,
    selection_site: u32,
    source: Value,
    output: Value,
    operands: (Value, Value),
});
impl NativeMapKeySpan {
    pub const fn region(&self) -> usize {
        self.region
    }
    pub const fn range_site(&self) -> u32 {
        self.range_site
    }
    pub const fn selection_site(&self) -> u32 {
        self.selection_site
    }
    pub const fn source(&self) -> Value {
        self.source
    }
    pub const fn output(&self) -> Value {
        self.output
    }
    pub const fn operands(&self) -> [Value; 2] {
        [self.operands.0, self.operands.1]
    }
}

#[cfg(any(feature = "pure_text", feature = "map_ops"))]
fn key_spans(
    plan: &Plan,
    regions: &[NativeMapOwnerRegion],
    facts: &crate::native_artifact::NativeProgramFacts<'_>,
) -> Result<Vec<NativeMapKeySpan>, &'static str> {
    use super::fusion_semantics::Read;
    let actions = facts.actions()?;
    let counts = facts.semantic_use_counts()?;
    let uses = facts.fusion_reads()?;
    let mut result: Vec<NativeMapKeySpan> = Vec::new();
    for (index, region) in regions.iter().enumerate() {
        for &site in region.reads().iter().chain(region.writes()) {
            let Action::LibraryCall { operands, .. } = &actions[site as usize] else {
                continue;
            };
            let key = operands[1];
            if result.iter().any(|span| span.output == key) {
                continue;
            }
            let Some(projection) =
                super::string_span_projection::selected_pair(plan, &key, &counts, &actions)
            else {
                continue;
            };
            if plan.blocks()[projection.selection_site() as usize].iteration()
                != Some(region.iteration())
            {
                continue;
            }
            // No transport, carrier, branch, constructor or unrelated consumer
            // may observe the absent canonical String. Ordinary dominance and
            // current-visit validation still governs every selected use.
            if uses[key.get() as usize].iter().any(|usage| match usage {
                Read::Action(at) if region.reads().contains(at) || region.writes().contains(at) =>
                    !matches!(&actions[*at as usize], Action::LibraryCall { operands, .. }
                        if operands[1] == key && operands.iter().filter(|&&v| v == key).count() == 1),
                _ => true,
            }) { continue; }
            result.push(NativeMapKeySpan {
                region: index,
                range_site: projection.range_site(),
                selection_site: projection.selection_site(),
                source: projection.source(),
                output: key,
                operands: (projection.operands()[0], projection.operands()[1]),
            });
        }
    }
    Ok(result)
}
#[cfg(not(any(feature = "pure_text", feature = "map_ops")))]
fn key_spans(_: &Plan, _: &[NativeMapOwnerRegion]) -> Result<Vec<NativeMapKeySpan>, &'static str> {
    Ok(Vec::new())
}
impl NativeMapOwnerWords {
    pub fn spans(&self) -> &[NativeMapKeySpan] {
        &self.spans
    }
    pub fn span_site(&self, site: usize) -> Option<usize> {
        self.span_sites.get(site).copied().flatten()
    }
    pub fn call_span(&self, site: usize) -> Option<usize> {
        self.call_spans.get(site).copied().flatten()
    }
    pub fn regions(&self) -> &[NativeMapOwnerRegion] {
        &self.regions
    }
    pub const fn aggregates(&self) -> &NativeScalarAggregates {
        &self.aggregates
    }
    pub fn initializer(&self, site: usize) -> Option<usize> {
        self.initializers.get(site).copied().flatten()
    }
    pub fn exit(&self, site: usize) -> Option<usize> {
        self.exits.get(site).copied().flatten()
    }
    pub fn action_region(&self, site: usize) -> Option<usize> {
        self.action_regions.get(site).copied().flatten()
    }
    pub fn call_region(&self, site: usize) -> Option<usize> {
        self.call_regions.get(site).copied().flatten()
    }
    pub const fn frame_words(&self) -> usize {
        self.frame_words
    }
    /// First private appended word. Root map references keep their value slots;
    /// ordinary scalar aggregates precede this boundary and never relocate.
    pub const fn owner_base(&self) -> usize {
        self.owner_base
    }

    /// Append private state after an already occupied native frame prefix.
    /// Borrowed String/closed-loop metadata keep all their existing offsets.
    /// This is layout composition, not runtime owner selection or authority.
    pub fn after_frame_words(mut self, prefix_words: usize) -> Result<Self, &'static str> {
        let shift = prefix_words
            .checked_sub(self.owner_base)
            .ok_or("map owner prefix overlaps ordinary native storage")?;
        let end = self
            .frame_words
            .checked_add(shift)
            .ok_or("map owner frame overflow")?;
        let words = self
            .aggregates
            .words
            .checked_add(shift)
            .ok_or("map owner word overflow")?;
        for region in &self.regions {
            for &value in region.values() {
                let slot = self
                    .aggregates
                    .slots
                    .get_mut(value.get() as usize)
                    .and_then(Option::as_mut)
                    .ok_or("map owner slot is missing")?;
                if slot.0 >= self.owner_base {
                    slot.0 = slot.0.checked_add(shift).ok_or("map owner slot overflow")?;
                } else if *slot != (value.get() as usize, 1) {
                    return Err("map owner root reference overlaps native metadata");
                }
            }
        }
        self.owner_base = prefix_words;
        self.frame_words = end;
        self.aggregates.words = words;
        Ok(self)
    }
}

fn width(schema: &Schema) -> Result<usize, &'static str> {
    match schema {
        Schema::UnsignedInteger(IntegerWidth::W64) | Schema::Map { .. } => Ok(1),
        Schema::Tuple(items) => items.iter().try_fold(0usize, |n, item| {
            n.checked_add(width(item)?)
                .ok_or("map state width overflow")
        }),
        Schema::Enum { variants, .. } => variants
            .iter()
            .try_fold(0usize, |n, variant| -> Result<usize, &'static str> {
                Ok(n.max(width(
                    variant.payload.as_ref().ok_or("map state empty payload")?,
                )?))
            })?
            .checked_add(1)
            .ok_or("map state width overflow"),
        _ => Err("unsupported private map state word kind"),
    }
}
fn map(schema: &Schema) -> bool {
    matches!(schema, Schema::Map { .. })
}
fn pack(plan: &Plan, action: &Action) -> Result<Option<Transport>, &'static str> {
    let Action::LibraryCall {
        template,
        operands,
        output,
    } = action
    else {
        return Ok(None);
    };
    let schema = |value: Value| plan.values()[value.get() as usize].definition().body();
    let words = match template.primitive() {
        Primitive::CompositePack(_) => operands
            .iter()
            .map(|&value| {
                if map(schema(value)) {
                    Word::Field(value, 0)
                } else {
                    Word::Scalar(value)
                }
            })
            .collect::<Vec<_>>(),
        Primitive::EnumPack => {
            let [tag, payload] = operands.as_ref() else {
                return Err("map state enum pack arity");
            };
            let ordinal = plan
                .constants()
                .iter()
                .find_map(|entry| {
                    if entry.value() != *tag {
                        return None;
                    }
                    let ValueData::Index(value) = entry.constant().data() else {
                        return None;
                    };
                    value.checked_sub(1)
                })
                .ok_or("map state requires selected constant constructor ordinal")?;
            let mut words = vec![Word::Constant(ordinal)];
            words.extend((0..width(schema(*payload))?).map(|i| Word::Field(*payload, i)));
            words.resize(width(schema(*output))?, Word::Constant(0));
            words
        }
        _ => return Ok(None),
    };
    if words.len() != width(schema(*output))? {
        return Err("map state constructor width mismatch");
    }
    Ok(Some(Transport::Pack {
        output: *output,
        words: words.into(),
    }))
}
fn project(plan: &Plan, action: &Action) -> Result<Transport, &'static str> {
    let Action::Structural {
        source,
        path,
        observation,
        output,
    } = action
    else {
        return Err("map state projection action");
    };
    let mut schema = plan.values()[source.get() as usize].definition().body();
    let mut offset = 0usize;
    let mut guards = Vec::new();
    for step in path {
        schema = match (step, schema) {
            (Step::EnumPayload { ordinal }, Schema::Enum { variants, .. }) => {
                guards.push((offset, *ordinal));
                offset = offset
                    .checked_add(1)
                    .ok_or("map state projection overflow")?;
                variants
                    .get(*ordinal as usize)
                    .and_then(|v| v.payload.as_ref())
                    .ok_or("map state projection variant")?
            }
            (Step::TupleItem { ordinal }, Schema::Tuple(items)) => {
                let ordinal = *ordinal as usize;
                for item in items.get(..ordinal).ok_or("map state projection field")? {
                    offset = offset
                        .checked_add(width(item)?)
                        .ok_or("map state projection overflow")?;
                }
                items.get(ordinal).ok_or("map state projection field")?
            }
            _ => return Err("map state projection path"),
        };
    }
    match observation {
        Observation::CanonicalValue => {
            if schema != plan.values()[output.get() as usize].definition().body() {
                return Err("map state projection output kind");
            }
            Ok(Transport::Project {
                source: *source,
                offset,
                guards: guards.into(),
                output: *output,
            })
        }
        Observation::U64 | Observation::EnumOrdinal { .. } => Ok(Transport::Observe {
            source: *source,
            offset,
            ordinal: match observation {
                Observation::EnumOrdinal { ordinal } => Some(*ordinal),
                _ => None,
            },
            guards: guards.into(),
            output: *output,
        }),
        _ => Err("unsupported private map state observation"),
    }
}
impl Plan {
    /// Explicit compiler groundwork, not the default aggregate selection.
    /// Runtime lifetime/geometry/site checks remain required before emission
    /// of a full program can adopt these opaque owner words.
    pub fn map_owner_words(&self) -> Result<NativeMapOwnerWords, &'static str> {
        self.facts().map_owner_words()
    }
}

pub(in crate::native_artifact) fn derive(
    plan: &Plan,
    facts: &crate::native_artifact::NativeProgramFacts<'_>,
) -> Result<NativeMapOwnerWords, &'static str> {
    let regions = super::map_owner_region::derive(plan, facts)?;
    let spans = key_spans(plan, &regions, facts)?;
    let mut span_sites = vec![None; plan.blocks().len()];
    let mut call_spans = vec![None; plan.blocks().len()];
    for (index, span) in spans.iter().enumerate() {
        for site in [span.range_site, span.selection_site] {
            if span_sites[site as usize].replace(index).is_some() {
                return Err("overlapping map key projections");
            }
        }
        let region = &regions[span.region];
        for &site in region.reads().iter().chain(region.writes()) {
            if matches!(plan.blocks()[site as usize].action(),
                Action::LibraryCall { operands, .. } if operands[1] == span.output)
            {
                call_spans[site as usize] = Some(index);
            }
        }
    }
    let ordinary = facts.scalar_aggregates();
    let mut end = plan
        .values()
        .len()
        .checked_add(plan.iterations().len())
        .and_then(|n| n.checked_add(ordinary.words()))
        .ok_or("map state frame overflow")?;
    let base = plan
        .values()
        .len()
        .checked_add(plan.iterations().len())
        .ok_or("map state frame overflow")?;
    let owner_base = end;
    // Preserve every existing scalar component and its offsets. New owner
    // words append after that complete layout; no strategy replaces it.
    let mut slots = ordinary.slots.to_vec();
    let mut actions = ordinary.actions.to_vec();
    let mut initializers = vec![None; plan.blocks().len()];
    let mut exits = vec![None; plan.blocks().len()];
    let mut action_regions = vec![None; plan.blocks().len()];
    let mut call_regions = vec![None; plan.blocks().len()];
    for (index, region) in regions.iter().enumerate() {
        initializers[region.tail().initialize() as usize] = Some(index);
        exits[region.tail().exit() as usize] = Some(index);
        for &site in region.reads().iter().chain(region.writes()) {
            if call_regions[site as usize].replace(index).is_some() {
                return Err("overlapping map owner call regions");
            }
        }
        let selected = |value: Value| region.values().contains(&value);
        for &value in region.values() {
            let definition = plan.values()[value.get() as usize].definition();
            let words = width(definition.body())?;
            let at = if map(definition.body()) {
                value.get() as usize
            } else {
                let at = end;
                end = end.checked_add(words).ok_or("map state frame overflow")?;
                at
            };
            if slots[value.get() as usize].replace((at, words)).is_some()
                || ordinary.slot(value).is_some()
            {
                return Err("overlapping map state word selection");
            }
        }
        for (site, block) in plan.blocks().iter().enumerate() {
            let action = match block.action() {
                Action::Scalar(Scalar::Copy { source, output }) if selected(*output) => {
                    if !selected(*source) {
                        return Err("foreign map state copy");
                    }
                    // A root binding is the zero-offset form of the same
                    // fixed slice used by structural payload bindings.
                    Some(Transport::Project {
                        source: *source,
                        offset: 0,
                        guards: Box::new([]),
                        output: *output,
                    })
                }
                Action::Structural { source, .. } if selected(*source) => {
                    Some(project(plan, block.action())?)
                }
                Action::LibraryCall { output, .. } if selected(*output) => {
                    pack(plan, block.action())?
                }
                Action::Latch { iteration } if *iteration == region.iteration() => {
                    Some(Transport::Copy {
                        source: region.tail().next(),
                        output: region.tail().carrier(),
                    })
                }
                _ => None,
            };
            if let Some(action) = action {
                if actions[site].replace(action).is_some()
                    || action_regions[site].replace(index).is_some()
                {
                    return Err("overlapping map state transport actions");
                }
            }
        }
    }
    Ok(NativeMapOwnerWords {
        regions,
        spans: spans.into(),
        span_sites: span_sites.into(),
        call_spans: call_spans.into(),
        initializers: initializers.into(),
        exits: exits.into(),
        action_regions: action_regions.into(),
        call_regions: call_regions.into(),
        owner_base,
        frame_words: end,
        aggregates: NativeScalarAggregates {
            slots: slots.into(),
            actions: actions.into(),
            constants: ordinary.constants.clone(),
            words: end - base,
        },
    })
}
