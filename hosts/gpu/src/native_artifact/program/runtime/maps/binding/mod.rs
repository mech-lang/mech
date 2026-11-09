//! Invocation-local storage binding, not a current-visit or publication permit.
//! The checked native caller must associate this borrowed plan/region pair with
//! its exact descriptor, frame and current site, and supply complete geometry.
use crate::native_artifact::program::runtime::maps::admission::{
    Error, Limits, add, check, finite, mul,
};
use crate::native_artifact::program::runtime::maps::string::{
    OwnedStringMap, PublicationGeometry as Geometry,
};
use mech_core::{
    ManagedMemoryReservation, ManagedNodeReservation, MemoryBudgetLimits, SchemaBody, SchemaTable,
    Value, ValueData,
};
use mech_engine::native_artifact::{
    NativeCollectionPrimitive as Primitive, NativeControlLoop, NativeIterationAction as Action,
    NativeLibraryWorkBudget, NativeMapOwnerRegion, NativeProgramArtifactPlan as Plan,
};
use std::mem::size_of;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

static TOKENS: AtomicU64 = AtomicU64::new(0);

/// A span is constructed only by the selected current-visit projection. Its
/// complete admitted owner stays pinned; the projected key is never a Value.
#[derive(Clone, Copy)]
pub(in crate::native_artifact) enum Key<'a> {
    Canonical(&'a Value),
    Span {
        value: mech_engine::native_artifact::NativeMatchValue,
        owner: &'a Value,
        bytes: &'a core::ops::Range<usize>,
    },
    MapOrdinal {
        value: mech_engine::native_artifact::NativeMatchValue,
        owner: &'a Value,
        text: &'a str,
        ordinal: usize,
    },
    MatrixCell {
        value: mech_engine::native_artifact::NativeMatchValue,
        owner: &'a Value,
        text: &'a str,
    },
}
impl<'a> From<&'a Value> for Key<'a> {
    fn from(value: &'a Value) -> Self {
        Self::Canonical(value)
    }
}
fn cap(value: Option<u64>) -> Result<u64, Error> {
    value.ok_or(Error::Binding("unbounded call dimension"))
}
// Global uniqueness rejects references from another region/invocation. Each
// CAS attempt is prepaid on the SAME finite account; contention cannot create
// an unmetered retry loop. Exhaustion never wraps or revives an old token.
fn token_from(counter: &AtomicU64, work: &NativeLibraryWorkBudget) -> Result<u64, Error> {
    let mut prior = counter.load(Ordering::Relaxed);
    loop {
        work.charge(128, 0)?;
        let next = prior.checked_add(1).ok_or(Error::Overflow)?;
        match counter.compare_exchange_weak(prior, next, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return Ok(next),
            Err(actual) => prior = actual,
        }
    }
}

pub(in crate::native_artifact) struct InitialWords {
    words: [u64; 33],
    len: usize,
    _reservation: ManagedMemoryReservation,
}
impl InitialWords {
    pub(in crate::native_artifact) fn words(&self) -> &[u64] {
        &self.words[..self.len]
    }
}
fn width(schema: &SchemaBody) -> Result<usize, Error> {
    match schema {
        SchemaBody::UnsignedInteger(_) | SchemaBody::Map { .. } => Ok(1),
        SchemaBody::Tuple(items) => items.iter().try_fold(0usize, |n, item| {
            n.checked_add(width(item)?).ok_or(Error::Overflow)
        }),
        SchemaBody::Record(fields) => fields.iter().try_fold(0usize, |n, field| {
            n.checked_add(width(&field.schema)?).ok_or(Error::Overflow)
        }),
        SchemaBody::Enum { variants, .. } => variants
            .iter()
            .try_fold(0usize, |n, variant| -> Result<usize, Error> {
                Ok(n.max(width(
                    variant
                        .payload
                        .as_ref()
                        .ok_or(Error::Binding("empty state payload"))?,
                )?))
            })?
            .checked_add(1)
            .ok_or(Error::Overflow),
        _ => Err(Error::Binding("unsupported state kind")),
    }
}
fn seed<'a>(
    schema: &SchemaBody,
    data: &'a ValueData,
    words: &mut [u64; 33],
    at: &mut usize,
    map: &mut Option<(usize, &'a mech_core::snapshot::MapValue)>,
) -> Result<(), Error> {
    if *at >= words.len() {
        return Err(Error::Binding("state word capacity"));
    }
    match (schema, data) {
        (SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64), ValueData::U64(value)) => {
            words[*at] = *value;
            *at += 1;
        }
        (SchemaBody::Map { .. }, ValueData::Map(value)) => {
            if map.replace((*at, value)).is_some() {
                return Err(Error::Binding("multiple seed maps"));
            }
            *at += 1;
        }
        (SchemaBody::Tuple(fields), ValueData::Tuple(items)) if fields.len() == items.len() => {
            for (field, item) in fields.iter().zip(items.iter()) {
                seed(field, item, words, at, map)?;
            }
        }
        (SchemaBody::Record(fields), ValueData::Record(value))
            if fields.len() == value.fields().len() =>
        {
            for (field, item) in fields.iter().zip(value.fields()) {
                seed(&field.schema, item, words, at, map)?;
            }
        }
        (SchemaBody::Enum { variants, .. }, ValueData::Enum(value)) => {
            words[*at] = u64::from(value.ordinal());
            *at += 1;
            let body = variants
                .get(value.ordinal() as usize)
                .and_then(|v| v.payload.as_ref())
                .ok_or(Error::Binding("seed variant"))?;
            seed(
                body,
                value.payload().ok_or(Error::Binding("seed payload"))?,
                words,
                at,
                map,
            )?;
        }
        _ => return Err(Error::Binding("seed shape/kind")),
    }
    Ok(())
}

/// Private observation of one pinned immutable key. Only a closed-loop owner
/// retains this between calls; the source String stays alive and unchanged.
/// A table token, mutation version and exact key span must all still match.
pub(in crate::native_artifact) struct NativeProbe {
    token: u64,
    version: u64,
    address: usize,
    bytes: usize,
    value: Option<u64>,
}

#[cfg(test)]
thread_local! {
    static NATIVE_PROBE_COUNTS: std::cell::Cell<(u64, u64)> = const { std::cell::Cell::new((0, 0)) };
}
#[cfg(test)]
pub(in crate::native_artifact) fn take_native_probe_counts() -> (u64, u64) {
    NATIVE_PROBE_COUNTS.with(|counts| counts.replace((0, 0)))
}

enum SeedSource<'a> {
    Canonical(&'a Value),
    Frame(&'a mech_engine::native_artifact::NativeMapSeedFrame),
}
impl SeedSource<'_> {
    fn schema(&self) -> mech_core::SchemaId {
        match self {
            Self::Canonical(v) => v.schema(),
            Self::Frame(v) => v.schema(),
        }
    }
    fn key(&self) -> mech_core::SchemaKey {
        match self {
            Self::Canonical(v) => v.schema_key(),
            Self::Frame(v) => v.schema_key(),
        }
    }
    fn shape_empty(&self) -> bool {
        match self {
            Self::Canonical(v) => v.shape().parameter_values().is_empty(),
            Self::Frame(_) => true,
        }
    }
    fn retained(&self, work: &NativeLibraryWorkBudget) -> Result<u64, Error> {
        let memory = work.memory_budget();
        match self {
            Self::Canonical(v) => v
                .memory_budget_retained_bytes(&memory)
                .ok_or(Error::Binding("unadmitted seed")),
            Self::Frame(v) => {
                v.check_account(work)
                    .map_err(|_| Error::Binding("foreign private seed account"))?;
                // Children may alias. A genuine same-account pin is enough;
                // complete caller inventory, not a sum of shared owners, prices coexistence.
                let mut largest = 0;
                for child in v.children().iter().flatten() {
                    largest = largest.max(
                        child
                            .memory_budget_retained_bytes(&memory)
                            .ok_or(Error::Binding("unadmitted private seed child"))?,
                    );
                }
                add(largest, v.metadata_demand().turn_peak_bytes)
            }
        }
    }
    fn footprint(&self, schemas: &SchemaTable) -> Result<(u64, u64), Error> {
        match self {
            Self::Canonical(v) => {
                let f = v
                    .retained_footprint(schemas)
                    .map_err(|_| Error::Binding("seed footprint"))?;
                Ok((f.retained_bytes, f.node_count))
            }
            Self::Frame(v) => Ok((v.logical_retained_bytes(), v.logical_nodes())),
        }
    }
    fn words<'a>(
        &'a self,
        body: &SchemaBody,
        words: &mut [u64; 33],
    ) -> Result<(usize, &'a mech_core::snapshot::MapValue), Error> {
        match self {
            Self::Canonical(v) => {
                let mut at = 0;
                let mut map = None;
                seed(body, v.data(), words, &mut at, &mut map)?;
                map.ok_or(Error::Binding("missing seed map"))
            }
            Self::Frame(v) => {
                if !matches!(body, SchemaBody::Tuple(_) | SchemaBody::Record(_))
                    || width(body)? != v.children().len()
                {
                    return Err(Error::Binding("private seed layout differs"));
                }
                let mut map = None;
                for (at, child) in v.children().iter().enumerate() {
                    match child
                        .as_ref()
                        .ok_or(Error::Binding("missing private seed child"))?
                        .data()
                    {
                        ValueData::U64(value) if at != v.map_ordinal() => words[at] = *value,
                        ValueData::Map(value) if at == v.map_ordinal() && map.is_none() => {
                            map = Some((at, value))
                        }
                        _ => return Err(Error::Binding("private seed kind differs")),
                    }
                }
                map.ok_or(Error::Binding("private seed lost map"))
            }
        }
    }
}

pub(in crate::native_artifact) struct Binding<'plan> {
    // Retire actual table/keys before the enclosing binding's reservation.
    map: Option<OwnedStringMap>,
    plan: &'plan Plan,
    region: &'plan NativeMapOwnerRegion,
    token: u64,
    native_version: u64,
    work: NativeLibraryWorkBudget,
    metadata: ManagedMemoryReservation,
    _nodes: ManagedNodeReservation,
}
impl<'plan> Binding<'plan> {
    pub(in crate::native_artifact) fn import(
        plan: &'plan Plan,
        region: &'plan NativeMapOwnerRegion,
        schemas: &Arc<SchemaTable>,
        source: &Value,
        work: NativeLibraryWorkBudget,
        geometry: Geometry,
        limits: MemoryBudgetLimits,
    ) -> Result<(Self, InitialWords), Error> {
        Self::import_source(
            plan,
            region,
            schemas,
            SeedSource::Canonical(source),
            None,
            work,
            geometry,
            limits,
        )
    }
    pub(in crate::native_artifact) fn import_frame(
        plan: &'plan Plan,
        region: &'plan NativeMapOwnerRegion,
        schemas: &Arc<SchemaTable>,
        frame: &mech_engine::native_artifact::NativeMapSeedFrame,
        work: NativeLibraryWorkBudget,
        geometry: Geometry,
        limits: MemoryBudgetLimits,
    ) -> Result<(Self, InitialWords), Error> {
        Self::import_source(
            plan,
            region,
            schemas,
            SeedSource::Frame(frame),
            None,
            work,
            geometry,
            limits,
        )
    }
    pub(in crate::native_artifact) fn import_frame_with_keys(
        plan: &'plan Plan,
        region: &'plan NativeMapOwnerRegion,
        schemas: &Arc<SchemaTable>,
        frame: &mech_engine::native_artifact::NativeMapSeedFrame,
        keys: &Value,
        work: NativeLibraryWorkBudget,
        geometry: Geometry,
        limits: MemoryBudgetLimits,
    ) -> Result<(Self, InitialWords), Error> {
        Self::import_source(
            plan,
            region,
            schemas,
            SeedSource::Frame(frame),
            Some(keys),
            work,
            geometry,
            limits,
        )
    }
    pub(in crate::native_artifact) fn has_borrowed_keys(&self) -> bool {
        self.map
            .as_ref()
            .is_some_and(OwnedStringMap::has_borrowed_keys)
    }
    pub(in crate::native_artifact) fn fixed_call_geometry(&self,geometry:Geometry,limits:MemoryBudgetLimits)->Result<(),Error>{
        let map=self.current(self.token)?;self.geometry(self.token,geometry,limits,map.len()as u64,map.key_bytes(),0).map(|_|())
    }
    pub(in crate::native_artifact) fn fixed_native_values(&mut self,source:&Value,geometry:Geometry,limits:MemoryBudgetLimits)->Result<Option<(u64,u64,u64)>,Error>{
        let map=self.current(self.token)?;
        self.geometry(self.token,geometry,limits,map.len()as u64,map.key_bytes(),0)?;
        let token=self.token;
        self.map.as_mut().ok_or(Error::Binding("closed fixed-key owner"))?.native_values(source).map(|v|v.map(|(pointer,len)|(pointer,len,token)))
    }
    fn import_source(
        plan: &'plan Plan,
        region: &'plan NativeMapOwnerRegion,
        schemas: &Arc<SchemaTable>,
        source: SeedSource<'_>,
        borrowed_keys: Option<&Value>,
        work: NativeLibraryWorkBudget,
        geometry: Geometry,
        limits: MemoryBudgetLimits,
    ) -> Result<(Self, InitialWords), Error> {
        finite(limits, cap)?;
        if plan.program_revision() != region.program_revision() {
            return Err(Error::Binding("foreign program revision"));
        }
        let Some(NativeControlLoop::Tail(tail)) =
            plan.iterations().get(region.iteration() as usize)
        else {
            return Err(Error::Binding("foreign owner region"));
        };
        let expected = region.tail();
        if [
            tail.initialize(),
            tail.header(),
            tail.body(),
            tail.latch(),
            tail.finish(),
            tail.exit(),
        ] != [
            expected.initialize(),
            expected.header(),
            expected.body(),
            expected.latch(),
            expected.finish(),
            expected.exit(),
        ] || [
            tail.seed(),
            tail.carrier(),
            tail.next(),
            tail.returned(),
            tail.result(),
        ] != [
            expected.seed(),
            expected.carrier(),
            expected.next(),
            expected.returned(),
            expected.result(),
        ] {
            return Err(Error::Binding("foreign owner region"));
        }
        let ty = plan
            .values()
            .get(tail.seed().get() as usize)
            .ok_or(Error::Binding("seed type"))?;
        let memory = work.memory_budget().with_limits(
            cap(limits.max_temporary_bytes)?,
            cap(limits.max_retained_nodes)?,
        );
        let retained = source.retained(&work)?;
        if source.schema() != ty.schema()
            || source.key() != ty.schema_key()
            || !source.shape_empty()
        {
            return Err(Error::Binding("seed schema"));
        }
        work.with_call_limits(
            cap(limits.max_compute_work)?,
            cap(limits.max_comparison_work)?,
            || {
                work.charge(add(mul(retained, 8)?, 256)?, 0)?;
                if schemas.get(ty.schema()) != Some(ty.definition()) {
                    return Err(Error::Binding("seed schema context"));
                }
                let (footprint_bytes, footprint_nodes) = source.footprint(schemas)?;
                if geometry.live_bytes < retained || geometry.live_nodes < footprint_nodes {
                    return Err(Error::Binding("incomplete seed geometry"));
                }
                check(1, cap(limits.max_output_elements)?, "output elements")?;
                check(
                    footprint_bytes,
                    cap(limits.max_output_bytes)?,
                    "output bytes",
                )?;
                check(
                    footprint_bytes,
                    cap(limits.max_cloned_bytes)?,
                    "cloned bytes",
                )?;
                let words_len = width(ty.definition().body())?;
                if words_len > 33 {
                    return Err(Error::Binding("seed word width"));
                }
                let fixed = add(size_of::<Self>() as u64, size_of::<InitialWords>() as u64)?;
                let outside = add(geometry.live_bytes, fixed)?;
                check(outside, cap(limits.max_temporary_bytes)?, "temporary bytes")?;
                let metadata = memory.reserve_capacity(size_of::<Self>() as u64)?;
                let private_nodes = memory.reserve_nodes(2)?;
                let mut initial = InitialWords {
                    words: [0; 33],
                    len: words_len,
                    _reservation: memory.reserve_capacity(size_of::<InitialWords>() as u64)?,
                };
                let (map_word, source_map) =
                    source.words(ty.definition().body(), &mut initial.words)?;
                let nodes = add(source_map.entries().len() as u64, 3)?;
                check(
                    add(geometry.live_nodes, nodes)?,
                    cap(limits.max_retained_nodes)?,
                    "retained nodes",
                )?;
                let remaining = cap(limits.max_temporary_bytes)?
                    .checked_sub(outside)
                    .ok_or(Error::Overflow)?;
                let storage = Limits {
                    entries: source_map.entries().len() as u64,
                    retained_bytes: remaining,
                    temporary_bytes: remaining,
                };
                let borrowed =
                    if let (SeedSource::Frame(frame), Some(keys)) = (&source, borrowed_keys) {
                        let child = frame
                            .children()
                            .get(map_word)
                            .and_then(Option::as_ref)
                            .ok_or(Error::Binding("private seed lost borrowed map child"))?;
                        match OwnedStringMap::try_with_borrowed_keys(
                            child,
                            keys,
                            schemas,
                            work.clone(),
                            storage,
                            memory.clone(),
                        ) {
                            Ok(owner) => owner,
                            // Keep all failed admission work on this original call.
                            // Ordinary import receives no renewed relative allowance.
                            Err(
                                Error::Memory(_)
                                | Error::Work(_)
                                | Error::Limit { .. }
                                | Error::Allocation,
                            ) => None,
                            Err(error) => return Err(error),
                        }
                    } else {
                        None
                    };
                let map = if let Some(map) = borrowed {
                    map
                } else {
                    let mut map =
                        OwnedStringMap::new_with_memory(work.clone(), storage, memory.clone())?;
                    for entry in source_map.entries() {
                        let (ValueData::String(key), ValueData::U64(value)) =
                            (entry.key().data(), entry.value())
                        else {
                            return Err(Error::Binding("seed map key/value kind"));
                        };
                        map.insert(key, *value)?;
                    }
                    map
                };
                check(
                    add(geometry.live_nodes, add(map.retained_nodes(), 2)?)?,
                    cap(limits.max_retained_nodes)?,
                    "retained nodes",
                )?;
                let token = token_from(&TOKENS, &work)?;
                initial.words[map_word] = token;
                Ok((
                    Self {
                        map: Some(map),
                        plan,
                        region,
                        token,
                        native_version: 0,
                        work: work.clone(),
                        metadata,
                        _nodes: private_nodes,
                    },
                    initial,
                ))
            },
        )?
    }
    fn current(&self, token: u64) -> Result<&OwnedStringMap, Error> {
        if token == 0 || token != self.token {
            return Err(Error::Binding("stale or foreign map reference"));
        }
        self.map.as_ref().ok_or(Error::Binding("closed owner"))
    }
    pub(in crate::native_artifact) fn constructor_statistics(
        &self,
        site: usize,
        token: u64,
    ) -> Result<(u64, u64), Error> {
        let map = self.current(token)?;
        let Some(Action::LibraryCall {
            template, output, ..
        }) = self.plan.blocks().get(site).map(|block| block.action())
        else {
            return Err(Error::Binding("constructor action missing"));
        };
        if !self.region.values().contains(output)
            || !matches!(
                template.primitive(),
                Primitive::CompositePack(_) | Primitive::EnumPack
            )
        {
            return Err(Error::Binding("unselected owner constructor"));
        }
        Ok((map.len() as u64, map.key_bytes()))
    }
    pub(in crate::native_artifact) fn retained_bytes(&self) -> Result<u64, Error> {
        add(
            self.map
                .as_ref()
                .map_or(Ok(0), OwnedStringMap::retained_bytes)?,
            self.metadata.capacity_bytes(),
        )
    }
    pub(in crate::native_artifact) fn retained_nodes(&self) -> Result<u64, Error> {
        self.map
            .as_ref()
            .map_or(Ok(1), |map| add(map.retained_nodes(), 2))
    }
    fn site(
        &self,
        site: usize,
        write: bool,
    ) -> Result<(&[mech_engine::native_artifact::NativeMatchValue], Primitive), Error> {
        let selected = if write {
            self.region.writes()
        } else {
            self.region.reads()
        };
        if !selected.contains(&(site as u32)) {
            return Err(Error::Binding("unselected owner site"));
        }
        let block = self
            .plan
            .blocks()
            .get(site)
            .ok_or(Error::Binding("owner site range"))?;
        let Action::LibraryCall {
            template, operands, ..
        } = block.action()
        else {
            return Err(Error::Binding("owner site is not a maintained call"));
        };
        if block.iteration() != Some(self.region.iteration()) {
            return Err(Error::Binding("foreign lexical owner"));
        }
        match (write, template.primitive(), operands.len()) {
            (true, Primitive::MapInsert, 3)
            | (false, Primitive::MapContainsKey | Primitive::MapKeyAccess, 2) => {
                Ok((operands, template.primitive()))
            }
            _ => Err(Error::Binding("owner call kind/arity")),
        }
    }
    fn key<'a>(
        &self,
        operand: mech_engine::native_artifact::NativeMatchValue,
        key: Key<'a>,
        geometry: Geometry,
    ) -> Result<&'a str, Error> {
        let ty = &self.plan.values()[operand.get() as usize];
        if let Key::MapOrdinal {
            value, owner, text, ..
        } = key
        {
            if value != operand
                || ty.definition().body() != &SchemaBody::String
                || !ty.definition().dimension_parameters().is_empty()
                || !matches!(owner.data(), ValueData::Map(_))
            {
                return Err(Error::Binding("wrong borrowed Map key"));
            }
            let retained = owner
                .memory_budget_retained_bytes(&self.work.memory_budget())
                .ok_or(Error::Binding("unadmitted Map key snapshot"))?;
            let map = self.map.as_ref().ok_or(Error::Binding("closed owner"))?;
            if geometry.live_bytes < add(self.retained_bytes()?, retained)?
                || geometry.live_nodes
                    < add(map.len() as u64, 5)?.max(add(map.retained_nodes(), 4)?)
            {
                return Err(Error::Binding("incomplete Map key geometry"));
            }
            return Ok(text);
        }
        if let Key::MatrixCell { value, owner, text } = key {
            if value != operand
                || ty.definition().body() != &SchemaBody::String
                || !ty.definition().dimension_parameters().is_empty()
                || !matches!(owner.data(), ValueData::Matrix(matrix)
                    if matches!(matrix.elements(), mech_core::snapshot::SequenceView::String(_)))
            {
                return Err(Error::Binding("wrong borrowed matrix key"));
            }
            let retained = owner
                .memory_budget_retained_bytes(&self.work.memory_budget())
                .ok_or(Error::Binding("unadmitted matrix key source"))?;
            let map = self.map.as_ref().ok_or(Error::Binding("closed owner"))?;
            if geometry.live_bytes < add(self.retained_bytes()?, retained)?
                || geometry.live_nodes < add(map.len() as u64, 5)?
            {
                return Err(Error::Binding("incomplete matrix key geometry"));
            }
            return Ok(text);
        }
        let (owner, bytes) = match key {
            Key::Canonical(key) => {
                if key.schema() != ty.schema() || key.schema_key() != ty.schema_key() {
                    return Err(Error::Binding("unadmitted or wrong typed key"));
                }
                (key, None)
            }
            Key::Span {
                value,
                owner,
                bytes,
            } => {
                if value != operand
                    || ty.definition().body() != &SchemaBody::String
                    || !ty.definition().dimension_parameters().is_empty()
                {
                    return Err(Error::Binding("wrong selected key projection"));
                }
                (owner, Some(bytes))
            }
            Key::MatrixCell { .. } | Key::MapOrdinal { .. } => unreachable!("handled above"),
        };
        let ValueData::String(text) = owner.data() else {
            return Err(Error::Binding("key kind"));
        };
        let retained = owner
            .memory_budget_retained_bytes(&self.work.memory_budget())
            .ok_or(Error::Binding("unadmitted key"))?;
        let map = self.map.as_ref().ok_or(Error::Binding("closed owner"))?;
        if geometry.live_bytes < add(self.retained_bytes()?, retained)?
            || geometry.live_nodes < add(map.len() as u64, 5)?
        {
            return Err(Error::Binding("incomplete key geometry"));
        }
        match bytes {
            Some(bytes) => text
                .get(bytes.clone())
                .ok_or(Error::Binding("key span bounds")),
            None => Ok(text),
        }
    }
    fn geometry(
        &self,
        token: u64,
        geometry: Geometry,
        limits: MemoryBudgetLimits,
        count: u64,
        payload: u64,
        cloned: u64,
    ) -> Result<Limits, Error> {
        finite(limits, cap)?;
        let map = self.current(token)?;
        if geometry.live_bytes < self.retained_bytes()?
            || geometry.live_nodes < add(map.len() as u64, 3)?.max(add(map.retained_nodes(), 2)?)
        {
            return Err(Error::Binding("incomplete owner geometry"));
        }
        let output = add(
            add(size_of::<Value>() as u64, size_of::<ValueData>() as u64)?,
            add(
                mul(
                    count,
                    (size_of::<mech_core::snapshot::MapEntryValue>() + 2 * size_of::<ValueData>())
                        as u64,
                )?,
                payload,
            )?,
        )?;
        check(1, cap(limits.max_output_elements)?, "output elements")?;
        check(output, cap(limits.max_output_bytes)?, "output bytes")?;
        check(cloned, cap(limits.max_cloned_bytes)?, "cloned bytes")?;
        check(
            add(geometry.live_nodes, add(mul(count, 2)?, 2)?)?,
            cap(limits.max_retained_nodes)?,
            "retained nodes",
        )?;
        check(
            geometry.live_bytes,
            cap(limits.max_temporary_bytes)?,
            "temporary bytes",
        )?;
        let outside = geometry
            .live_bytes
            .checked_sub(map.retained_bytes()?)
            .ok_or(Error::Overflow)?;
        let remaining = cap(limits.max_temporary_bytes)?
            .checked_sub(outside)
            .ok_or(Error::Overflow)?;
        Ok(Limits {
            entries: count,
            retained_bytes: remaining,
            temporary_bytes: remaining,
        })
    }
    pub(in crate::native_artifact) fn read<'a>(
        &self,
        site: usize,
        token: u64,
        key: impl Into<Key<'a>>,
        geometry: Geometry,
        limits: MemoryBudgetLimits,
    ) -> Result<u64, Error> {
        self.current(token)?;
        let (operands, primitive) = self.site(site, false)?;
        let key = self.key(operands[1], key.into(), geometry)?;
        self.work.with_call_limits(
            cap(limits.max_compute_work)?,
            cap(limits.max_comparison_work)?,
            || {
                self.work.charge(256, 0)?;
                let map = self.current(token)?;
                // Reads return one native scalar, not a newly cloned whole map.
                self.geometry(token, geometry, limits, 0, 0, 0)?;
                let found = map.get(key)?;
                match primitive {
                    Primitive::MapContainsKey => Ok(u64::from(found.is_some())),
                    Primitive::MapKeyAccess => found.ok_or(Error::Binding("missing map key")),
                    _ => Err(Error::Binding("owner read kind")),
                }
            },
        )?
    }
    pub(in crate::native_artifact) fn insert<'a>(
        &mut self,
        site: usize,
        token: u64,
        key: impl Into<Key<'a>>,
        value: u64,
        geometry: Geometry,
        limits: MemoryBudgetLimits,
    ) -> Result<u64, Error> {
        self.current(token)?;
        let (operands, _) = self.site(site, true)?;
        let key = key.into();
        let ordinal = match key {
            Key::MapOrdinal {
                owner,
                ordinal,
                text,
                ..
            } => Some((owner, ordinal, text)),
            _ => None,
        };
        let key = self.key(operands[1], key, geometry)?;
        let work = self.work.clone();
        work.with_call_limits(
            cap(limits.max_compute_work)?,
            cap(limits.max_comparison_work)?,
            || {
                work.charge(256, 0)?;
                let map = self.current(token)?;
                let borrowed = map.has_borrowed_keys();
                if borrowed && ordinal.is_none() {
                    return Err(Error::Binding("borrowed dictionary write lost ordinal"));
                }
                let fresh = if borrowed {
                    false
                } else {
                    map.get(key)?.is_none()
                };
                let count = add(map.len() as u64, u64::from(fresh))?;
                let cloned = if fresh { key.len() as u64 } else { 0 };
                let payload = add(map.key_bytes(), cloned)?;
                let storage = self.geometry(token, geometry, limits, count, payload, cloned)?;
                // Issue before mutation; a work/exhaustion failure cannot update
                // the table or advance this binding's current version.
                let next = token_from(&TOKENS, &work)?;
                let map = self.map.as_mut().ok_or(Error::Binding("closed owner"))?;
                map.set_limits(storage);
                if borrowed {
                    let (source, index, text) = ordinal.unwrap();
                    map.replace_borrowed(source, index, text, value)?;
                } else {
                    map.insert(key, value)?;
                }
                self.token = next;
                Ok(next)
            },
        )?
    }
    /// Only the invocation's closed-loop owner may call these methods. The
    /// compiler's linear-use proof excludes any observer of an intermediate
    /// version, so its private token stays stable until canonical publication.
    /// This is still generic storage: the generated caller supplies the value.
    pub(in crate::native_artifact) fn native_seed_is_empty(&self) -> Result<bool, Error> {
        Ok(self.current(self.token)?.len() == 0)
    }
    pub(in crate::native_artifact) fn install_native_map(
        &mut self,
        map: OwnedStringMap,
    ) -> Result<(), Error> {
        if !self.native_seed_is_empty()? {
            return Err(Error::Binding("packed map replaced nonempty seed"));
        }
        self.map = Some(map);
        self.native_version = self.native_version.checked_add(1).ok_or(Error::Overflow)?;
        Ok(())
    }
    fn native_lookup(
        &self,
        key: &str,
        probe: &mut Option<NativeProbe>,
    ) -> Result<Option<u64>, Error> {
        let map = self.current(self.token)?;
        if let Some(cached) = probe.as_ref().filter(|cached| {
            cached.token == self.token
                && cached.version == self.native_version
                && cached.address == key.as_ptr() as usize
                && cached.bytes == key.len()
        }) {
            // Preserve the complete original lookup debit before observing
            // its cached scalar, including current per-call/root caps.
            map.lookup_work(key, 1)?;
            #[cfg(test)]
            NATIVE_PROBE_COUNTS.with(|counts| {
                let (hits, misses) = counts.get();
                counts.set((hits + 1, misses));
            });
            return Ok(cached.value);
        }
        let value = map.get(key)?;
        *probe = Some(NativeProbe {
            token: self.token,
            version: self.native_version,
            address: key.as_ptr() as usize,
            bytes: key.len(),
            value,
        });
        #[cfg(test)]
        NATIVE_PROBE_COUNTS.with(|counts| {
            let (hits, misses) = counts.get();
            counts.set((hits, misses + 1));
        });
        Ok(value)
    }
    pub(in crate::native_artifact) fn native_read(
        &self,
        key: &str,
        contains: bool,
        probe: &mut Option<NativeProbe>,
        limits: MemoryBudgetLimits,
    ) -> Result<u64, Error> {
        self.work.with_call_limits(
            cap(limits.max_compute_work)?,
            cap(limits.max_comparison_work)?,
            || {
                let found = self.native_lookup(key, probe)?;
                if contains {
                    Ok(u64::from(found.is_some()))
                } else {
                    found.ok_or(Error::Binding("missing map key"))
                }
            },
        )?
    }
    pub(in crate::native_artifact) fn native_insert(
        &mut self,
        key: &str,
        value: u64,
        probe: &mut Option<NativeProbe>,
        geometry: Geometry,
        limits: MemoryBudgetLimits,
    ) -> Result<u64, Error> {
        let work = self.work.clone();
        work.with_call_limits(
            cap(limits.max_compute_work)?,
            cap(limits.max_comparison_work)?,
            || {
                let fresh = self.native_lookup(key, probe)?.is_none();
                let map = self.current(self.token)?;
                let count = add(map.len() as u64, u64::from(fresh))?;
                let cloned = if fresh { key.len() as u64 } else { 0 };
                let storage = self.geometry(
                    self.token,
                    geometry,
                    limits,
                    count,
                    add(map.key_bytes(), cloned)?,
                    cloned,
                )?;
                let next_version = self.native_version.checked_add(1).ok_or(Error::Overflow)?;
                // Retire every old observation before storage can mutate,
                // including replacement without growth and failed attempts.
                self.native_version = next_version;
                *probe = None;
                let map = self.map.as_mut().ok_or(Error::Binding("closed owner"))?;
                map.set_limits(storage);
                map.insert(key, value)?;
                Ok(self.token)
            },
        )?
    }
    /// Check the same original publication boundary without materializing a
    /// canonical map. Complete frozen storage and logical caps are checked by
    /// the private caller before this binding is retired.
    pub(in crate::native_artifact) fn frozen_exit_schema(
        &self,
        site: usize,
        token: u64,
        schemas: &Arc<SchemaTable>,
    ) -> Result<mech_core::SchemaId, Error> {
        self.current(token)?;
        if site != self.region.tail().exit() as usize {
            return Err(Error::Binding("unselected frozen owner exit"));
        }
        let ty = &self.plan.values()[self.region.tail().result().get() as usize];
        if schemas.get(ty.schema()) != Some(ty.definition()) {
            return Err(Error::Binding("frozen result schema context"));
        }
        Ok(ty.schema())
    }

    #[cfg(test)]
    pub(in crate::native_artifact) fn finish(
        &mut self,
        site: usize,
        token: u64,
        schemas: &Arc<SchemaTable>,
        geometry: Geometry,
        limits: MemoryBudgetLimits,
    ) -> Result<Value, Error> {
        self.finish_words(site, token, &[token], schemas, geometry, limits)
    }

    pub(in crate::native_artifact) fn finish_words(
        &mut self,
        site: usize,
        token: u64,
        words: &[u64],
        schemas: &Arc<SchemaTable>,
        geometry: Geometry,
        limits: MemoryBudgetLimits,
    ) -> Result<Value, Error> {
        match self.finish_words_choice(site, token, words, schemas, geometry, limits, false)? {
            super::string::frozen::FreezeChoice::Canonical(value) => Ok(value),
            super::string::frozen::FreezeChoice::Frozen(_) => {
                Err(Error::Binding("unexpected private canonical publication"))
            }
        }
    }
    pub(in crate::native_artifact) fn finish_words_choice(
        &mut self,
        site: usize,
        token: u64,
        words: &[u64],
        schemas: &Arc<SchemaTable>,
        geometry: Geometry,
        limits: MemoryBudgetLimits,
        may_freeze: bool,
    ) -> Result<super::string::frozen::FreezeChoice, Error> {
        self.current(token)?;
        if site != self.region.tail().exit() as usize {
            return Err(Error::Binding("unselected owner exit"));
        }
        let work = self.work.clone();
        let enclosing = work.has_active_call_limit()?;
        let started = work.used();
        work.with_call_limits(
            cap(limits.max_compute_work)?,
            cap(limits.max_comparison_work)?,
            || {
                work.charge(256, 0)?;
                let ty = &self.plan.values()[self.region.tail().result().get() as usize];
                if schemas.get(ty.schema()) != Some(ty.definition()) {
                    return Err(Error::Binding("result schema context"));
                }
                let map_word = match ty.definition().body() {
                    SchemaBody::Map { .. } => Some(0),
                    SchemaBody::Tuple(fields) => fields
                        .iter()
                        .position(|field| matches!(field, SchemaBody::Map { .. })),
                    SchemaBody::Record(fields) => fields
                        .iter()
                        .position(|field| matches!(field.schema, SchemaBody::Map { .. })),
                    _ => None,
                }
                .ok_or(Error::Binding("result map field"))?;
                if words.len() != width(ty.definition().body())?
                    || words.get(map_word).copied() != Some(token)
                {
                    return Err(Error::Binding("result word shape/token"));
                }
                let map = self.current(token)?;
                self.geometry(
                    token,
                    geometry,
                    limits,
                    map.len() as u64,
                    map.key_bytes(),
                    map.key_bytes(),
                )?;
                let value = if may_freeze
                    && !enclosing
                    && matches!(ty.definition().body(), SchemaBody::Map { .. })
                {
                    let map = self
                        .map
                        .take()
                        .ok_or(Error::Binding("closed dictionary owner"))?;
                    map.freeze_or_publish_in_call(
                        schemas,
                        ty.schema(),
                        geometry,
                        limits,
                        started,
                        true,
                    )?
                } else {
                    super::string::frozen::FreezeChoice::Canonical(map.publish_result(
                        schemas,
                        ty.schema(),
                        words,
                        geometry,
                        limits,
                    )?)
                };
                self.token = 0;
                drop(self.map.take());
                Ok(value)
            },
        )?
    }
}

#[cfg(test)]
mod tests;
