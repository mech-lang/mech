//! Invocation-local storage binding, not a current-visit or publication permit.
//! The checked native caller must associate this borrowed plan/region pair with
//! its exact descriptor, frame and current site, and supply complete geometry.
use super::owned_string_map::{Error, Limits, OwnedStringMap, PublicationGeometry as Geometry};
use mech_core::{
    ManagedMemoryReservation, ManagedNodeReservation, MemoryBudgetLimits, SchemaBody, SchemaTable, Value, ValueData,
};
use mech_engine::native_artifact::{
    NativeCollectionPrimitive as Primitive, NativeControlLoop, NativeIterationAction as Action,
    NativeLibraryWorkBudget, NativeMapOwnerRegion, NativeProgramArtifactPlan as Plan,
};
use std::{
    mem::size_of,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

static TOKENS: AtomicU64 = AtomicU64::new(0);

/// A span is constructed only by the selected current-visit projection. Its
/// complete admitted owner stays pinned; the projected key is never a Value.
pub(super) enum Key<'a> {
    Canonical(&'a Value),
    Span { value: mech_engine::native_artifact::NativeMatchValue,
        owner: &'a Value, bytes: &'a core::ops::Range<usize> },
}
impl<'a> From<&'a Value> for Key<'a> {
    fn from(value: &'a Value) -> Self { Self::Canonical(value) }
}
fn add(a: u64, b: u64) -> Result<u64, Error> {
    a.checked_add(b).ok_or(Error::Overflow)
}
fn mul(a: u64, b: u64) -> Result<u64, Error> {
    a.checked_mul(b).ok_or(Error::Overflow)
}
fn cap(value: Option<u64>) -> Result<u64, Error> {
    value.ok_or(Error::Binding("unbounded call dimension"))
}
fn check(required: u64, limit: u64, field: &'static str) -> Result<(), Error> {
    if required > limit {
        Err(Error::Limit {
            required,
            limit,
            field,
        })
    } else {
        Ok(())
    }
}
fn finite(limits: MemoryBudgetLimits) -> Result<(), Error> {
    for limit in [
        limits.max_output_elements,
        limits.max_output_bytes,
        limits.max_temporary_bytes,
        limits.max_cloned_bytes,
        limits.max_retained_nodes,
        limits.max_compute_work,
        limits.max_comparison_work,
    ] {
        cap(limit)?;
    }
    Ok(())
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

pub(super) struct InitialWords {
    words: [u64; 33],
    len: usize,
    _reservation: ManagedMemoryReservation,
}
impl InitialWords {
    pub(super) fn words(&self) -> &[u64] {
        &self.words[..self.len]
    }
}
fn width(schema: &SchemaBody) -> Result<usize, Error> {
    match schema {
        SchemaBody::UnsignedInteger(_) | SchemaBody::Map { .. } => Ok(1),
        SchemaBody::Tuple(items) => items.iter().try_fold(0usize, |n, item| {
            n.checked_add(width(item)?).ok_or(Error::Overflow)
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
pub(super) struct NativeProbe {
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
pub(super) fn take_native_probe_counts() -> (u64, u64) {
    NATIVE_PROBE_COUNTS.with(|counts| counts.replace((0, 0)))
}

pub(super) struct Binding<'plan> {
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
    pub(super) fn import(
        plan: &'plan Plan,
        region: &'plan NativeMapOwnerRegion,
        schemas: &Arc<SchemaTable>,
        source: &Value,
        work: NativeLibraryWorkBudget,
        geometry: Geometry,
        limits: MemoryBudgetLimits,
    ) -> Result<(Self, InitialWords), Error> {
        finite(limits)?;
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
        let memory = work.memory_budget().with_limits(cap(limits.max_temporary_bytes)?,
            cap(limits.max_retained_nodes)?);
        let retained = source
            .memory_budget_retained_bytes(&memory)
            .ok_or(Error::Binding("unadmitted seed"))?;
        if source.schema() != ty.schema()
            || source.schema_key() != ty.schema_key()
            || !source.shape().parameter_values().is_empty()
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
                let footprint = source
                    .retained_footprint(schemas)
                    .map_err(|_| Error::Binding("seed footprint"))?;
                if geometry.live_bytes < retained || geometry.live_nodes < footprint.node_count {
                    return Err(Error::Binding("incomplete seed geometry"));
                }
                check(1, cap(limits.max_output_elements)?, "output elements")?;
                check(
                    footprint.retained_bytes,
                    cap(limits.max_output_bytes)?,
                    "output bytes",
                )?;
                check(
                    footprint.retained_bytes,
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
                let mut source_map = None;
                let mut at = 0;
                seed(
                    ty.definition().body(),
                    source.data(),
                    &mut initial.words,
                    &mut at,
                    &mut source_map,
                )?;
                let (map_word, source_map) =
                    source_map.ok_or(Error::Binding("missing seed map"))?;
                let nodes = add(source_map.entries().len() as u64, 3)?;
                check(
                    add(geometry.live_nodes, nodes)?,
                    cap(limits.max_retained_nodes)?,
                    "retained nodes",
                )?;
                let remaining = cap(limits.max_temporary_bytes)?
                    .checked_sub(outside)
                    .ok_or(Error::Overflow)?;
                let mut map = OwnedStringMap::new_with_memory(
                    work.clone(),
                    Limits {
                        entries: source_map.entries().len() as u64,
                        retained_bytes: remaining,
                        temporary_bytes: remaining,
                    },
                    memory.clone(),
                )?;
                for entry in source_map.entries() {
                    let (ValueData::String(key), ValueData::U64(value)) =
                        (entry.key().data(), entry.value())
                    else {
                        return Err(Error::Binding("seed map key/value kind"));
                    };
                    map.insert(key, *value)?;
                }
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
    pub(super) fn retained_bytes(&self) -> Result<u64, Error> {
        add(
            self.map
                .as_ref()
                .map_or(Ok(0), OwnedStringMap::retained_bytes)?,
            self.metadata.capacity_bytes(),
        )
    }
    pub(super) fn retained_nodes(&self) -> Result<u64, Error> {
        self.map.as_ref().map_or(Ok(1), |map| add(map.len() as u64, 3))
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
        let (owner, bytes) = match key {
            Key::Canonical(key) => {
                if key.schema() != ty.schema() || key.schema_key() != ty.schema_key() {
                    return Err(Error::Binding("unadmitted or wrong typed key"));
                }
                (key, None)
            }
            Key::Span { value, owner, bytes } => {
                if value != operand || ty.definition().body() != &SchemaBody::String
                    || !ty.definition().dimension_parameters().is_empty() {
                    return Err(Error::Binding("wrong selected key projection"));
                }
                (owner, Some(bytes))
            }
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
            Some(bytes) => text.get(bytes.clone()).ok_or(Error::Binding("key span bounds")),
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
        finite(limits)?;
        let map = self.current(token)?;
        if geometry.live_bytes < self.retained_bytes()?
            || geometry.live_nodes < add(map.len() as u64, 3)?
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
    pub(super) fn read<'a>(
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
    pub(super) fn insert<'a>(
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
        let key = self.key(operands[1], key.into(), geometry)?;
        let work = self.work.clone();
        work.with_call_limits(
            cap(limits.max_compute_work)?,
            cap(limits.max_comparison_work)?,
            || {
                work.charge(256, 0)?;
                let map = self.current(token)?;
                let fresh = map.get(key)?.is_none();
                let count = add(map.len() as u64, u64::from(fresh))?;
                let cloned = if fresh { key.len() as u64 } else { 0 };
                let payload = add(map.key_bytes(), cloned)?;
                let storage = self.geometry(token, geometry, limits, count, payload, cloned)?;
                // Issue before mutation; a work/exhaustion failure cannot update
                // the table or advance this binding's current version.
                let next = token_from(&TOKENS, &work)?;
                let map = self.map.as_mut().ok_or(Error::Binding("closed owner"))?;
                map.set_limits(storage);
                map.insert(key, value)?;
                self.token = next;
                Ok(next)
            },
        )?
    }
    /// Only the invocation's closed-loop owner may call these methods. The
    /// compiler's linear-use proof excludes any observer of an intermediate
    /// version, so its private token stays stable until canonical publication.
    /// This is still generic storage: the generated caller supplies the value.
    pub(super) fn native_seed_is_empty(&self) -> Result<bool, Error> {
        Ok(self.current(self.token)?.len() == 0)
    }
    pub(super) fn install_native_map(&mut self, map: OwnedStringMap) -> Result<(), Error> {
        if !self.native_seed_is_empty()? { return Err(Error::Binding("packed map replaced nonempty seed")); }
        self.map = Some(map);
        self.native_version = self.native_version.checked_add(1).ok_or(Error::Overflow)?;
        Ok(())
    }
    fn native_lookup(&self, key: &str, probe: &mut Option<NativeProbe>) -> Result<Option<u64>, Error> {
        let map = self.current(self.token)?;
        if let Some(cached) = probe.as_ref().filter(|cached|
            cached.token == self.token && cached.version == self.native_version
                && cached.address == key.as_ptr() as usize && cached.bytes == key.len())
        {
            // Preserve the complete original lookup debit before observing
            // its cached scalar, including current per-call/root caps.
            map.lookup_work(key, 1)?;
            #[cfg(test)]
            NATIVE_PROBE_COUNTS.with(|counts| { let (hits, misses) = counts.get(); counts.set((hits + 1, misses)); });
            return Ok(cached.value);
        }
        let value = map.get(key)?;
        *probe = Some(NativeProbe { token: self.token, version: self.native_version,
            address: key.as_ptr() as usize, bytes: key.len(), value });
        #[cfg(test)]
        NATIVE_PROBE_COUNTS.with(|counts| { let (hits, misses) = counts.get(); counts.set((hits, misses + 1)); });
        Ok(value)
    }
    pub(super) fn native_read(&self, key: &str, contains: bool, probe: &mut Option<NativeProbe>,
        limits: MemoryBudgetLimits) -> Result<u64, Error>
    {
        self.work.with_call_limits(cap(limits.max_compute_work)?,
            cap(limits.max_comparison_work)?, || {
                let found = self.native_lookup(key, probe)?;
                if contains { Ok(u64::from(found.is_some())) }
                else { found.ok_or(Error::Binding("missing map key")) }
            })?
    }
    pub(super) fn native_insert(&mut self, key: &str, value: u64, probe: &mut Option<NativeProbe>,
        geometry: Geometry, limits: MemoryBudgetLimits) -> Result<u64, Error>
    {
        let work = self.work.clone();
        work.with_call_limits(cap(limits.max_compute_work)?,
            cap(limits.max_comparison_work)?, || {
                let fresh = self.native_lookup(key, probe)?.is_none();
                let map = self.current(self.token)?;
                let count = add(map.len() as u64, u64::from(fresh))?;
                let cloned = if fresh { key.len() as u64 } else { 0 };
                let storage = self.geometry(self.token, geometry, limits,
                    count, add(map.key_bytes(), cloned)?, cloned)?;
                let next_version = self.native_version.checked_add(1).ok_or(Error::Overflow)?;
                // Retire every old observation before storage can mutate,
                // including replacement without growth and failed attempts.
                self.native_version = next_version;
                *probe = None;
                let map = self.map.as_mut().ok_or(Error::Binding("closed owner"))?;
                map.set_limits(storage);
                map.insert(key, value)?;
                Ok(self.token)
            })?
    }
    /// Check the same original publication boundary without materializing a
    /// canonical map. Complete frozen storage and logical caps are checked by
    /// the private caller before this binding is retired.
    pub(super) fn frozen_exit_schema(&self, site: usize, token: u64, schemas: &Arc<SchemaTable>)
        -> Result<mech_core::SchemaId, Error>
    {
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

    pub(super) fn finish(
        &mut self,
        site: usize,
        token: u64,
        schemas: &Arc<SchemaTable>,
        geometry: Geometry,
        limits: MemoryBudgetLimits,
    ) -> Result<Value, Error> {
        self.current(token)?;
        if site != self.region.tail().exit() as usize {
            return Err(Error::Binding("unselected owner exit"));
        }
        let work = self.work.clone();
        work.with_call_limits(
            cap(limits.max_compute_work)?,
            cap(limits.max_comparison_work)?,
            || {
                work.charge(256, 0)?;
                let ty = &self.plan.values()[self.region.tail().result().get() as usize];
                if schemas.get(ty.schema()) != Some(ty.definition()) {
                    return Err(Error::Binding("result schema context"));
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
                let value = map.publish(schemas, ty.schema(), geometry, limits)?;
                self.token = 0;
                drop(self.map.take());
                Ok(value)
            },
        )?
    }
}

#[cfg(test)]
#[path = "owned_map_binding_tests.rs"]
mod tests;
