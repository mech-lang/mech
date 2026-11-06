//! Same-plan proof for a packed numeric leaf under a suspended parent.
//! Exact frame recipes own the connected escape analysis. This adds identity
//! provenance for every managed field, including every alternative definition.
use mech_core::{IntegerWidth, SchemaBody};
use mech_engine::native_artifact::{
    NativeAggregateWord, NativeBufferFrameLane as Lane, NativeBufferFrameOperation as Frame,
    NativeBufferFrames, NativeControlLoop, NativeIterationAction as Action,
    NativeIterationTerminator as Term, NativeMatchAction as Scalar,
    NativeMatchTerminator as Branch, NativePreparedLibraryCall, NativeProgramArtifactPlan as Plan,
    NativeProgramFacts, NativeScalarAggregateAction as Aggregate, NativeScalarAggregates,
    NativeScalarKind, NativeTailLoop,
};
use std::collections::{BTreeMap, BTreeSet};
#[path = "numeric_append.rs"]
mod append;
#[path = "numeric_fields.rs"]
pub(super) mod numeric_fields;
#[path = "numeric_results.rs"]
mod results;
#[path = "numeric_versions.rs"]
mod versions;
pub(in crate::native_artifact) use results::ResultRecipe;
pub(super) use results::scalar_aggregate;

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) enum ArrayStorage {
    Carrier {
        field: usize,
        physical: usize,
    },
    /// Exact immutable ancestor value; never a mutable carrier or body result.
    Capture {
        value: u32,
    },
    /// A managed leaf of an exact immutable ancestor tuple/record. The field
    /// is a structural leaf ordinal; physical is its current private home.
    FrameCapture {
        value: u32,
        field: usize,
        physical: usize,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct Array {
    /// Proof-local identity. Carrier roots use their structural leaf ordinal;
    /// immutable captures receive disjoint identities after those leaves.
    pub root: usize,
    pub storage: ArrayStorage,
    pub schema: mech_core::SchemaId,
    pub width: u8,
    /// Exact selected site whose original factory validates the entry view.
    pub witness: usize,
    pub writable: bool,
    pub appends: bool,
}
impl Array {
    pub fn physical(&self) -> Option<usize> {
        match self.storage {
            ArrayStorage::Carrier { physical, .. } => Some(physical),
            ArrayStorage::Capture { .. } | ArrayStorage::FrameCapture { .. } => None,
        }
    }
    pub fn entry_key(&self, tail: &NativeTailLoop) -> (u32, Option<usize>) {
        match self.storage {
            ArrayStorage::Carrier { field, .. } => (tail.carrier().get(), Some(field)),
            ArrayStorage::Capture { value } => (value, None),
            ArrayStorage::FrameCapture { value, field, .. } => (value, Some(field)),
        }
    }
}
#[path = "numeric_frame.rs"]
pub(in crate::native_artifact) mod numeric_frame;
pub(in crate::native_artifact) use numeric_frame::{FrameWords, OwnedFrameWords};

pub(in crate::native_artifact::program::prepare) fn scalar_node_count(
    paths: &[(Vec<usize>, usize)],
) -> usize {
    let mut previous: &[usize] = &[];
    let mut count = 0;
    for (path, _) in paths {
        let common = previous
            .iter()
            .zip(path)
            .take_while(|(a, b)| a == b)
            .count();
        count += path.len() - common;
        previous = path;
    }
    count
}
/// Fixed invocation scalar-bank offsets, derived from the exact current plan.
/// Neither an offset nor a schema path grants ownership or entry authority.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct ScalarBinding {
    pub value: u32,
    pub word: usize,
    pub physical: usize,
    #[serde(deserialize_with = "deserialize_scalar_kind")]
    pub kind: NativeScalarKind,
    pub steps: usize,
}
fn deserialize_scalar_kind<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<NativeScalarKind, D::Error> {
    struct Kind;
    impl<'de> serde::de::Visitor<'de> for Kind {
        type Value = NativeScalarKind;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("a prepared scalar binding kind")
        }
        fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
            match value {
                "Bool" => Ok(NativeScalarKind::Bool),
                "U8" => Ok(NativeScalarKind::U8),
                "U16" => Ok(NativeScalarKind::U16),
                "U32" => Ok(NativeScalarKind::U32),
                "U64" => Ok(NativeScalarKind::U64),
                "I64" => Ok(NativeScalarKind::I64),
                "F64" => Ok(NativeScalarKind::F64),
                _ => Err(E::custom("unknown prepared scalar binding kind")),
            }
        }
    }
    deserializer.deserialize_str(Kind)
}

#[cfg(test)]
mod scalar_kind_borrow_tests {
    use super::*;
    use serde::Deserialize;
    #[derive(Deserialize)]
    struct Kind(#[serde(deserialize_with = "deserialize_scalar_kind")] NativeScalarKind);
    #[test]
    fn borrowed_frame_metadata_scalar_kinds_keep_all_variants_and_reject_other_values() {
        for (name, expected) in [
            ("Bool", NativeScalarKind::Bool),
            ("U8", NativeScalarKind::U8),
            ("U16", NativeScalarKind::U16),
            ("U32", NativeScalarKind::U32),
            ("U64", NativeScalarKind::U64),
            ("I64", NativeScalarKind::I64),
            ("F64", NativeScalarKind::F64),
        ] {
            let json = serde_json::to_vec(name).unwrap();
            let native = mech_engine::native_artifact::encode_native_metadata_image(&json).unwrap();
            for bytes in [&json[..], &native[..]] {
                let mut reader =
                    mech_engine::native_artifact::BuildOwnedImageReader::from_image(bytes).unwrap();
                assert_eq!(Kind::deserialize(&mut reader).unwrap().0, expected);
                reader.end().unwrap();
            }
        }
        for json in [r#""u64""#, r#""Other""#, "0", "true", "null", "[]", "{}"] {
            let native =
                mech_engine::native_artifact::encode_native_metadata_image(json.as_bytes())
                    .unwrap();
            for bytes in [json.as_bytes(), &native[..]] {
                let mut reader =
                    mech_engine::native_artifact::BuildOwnedImageReader::from_image(bytes).unwrap();
                assert!(Kind::deserialize(&mut reader).is_err());
            }
        }
    }
}
pub(in crate::native_artifact::program::prepare) fn scalar_bindings(
    frames: &NativeBufferFrames,
    value: u32,
    paths: &[(Vec<usize>, usize)],
) -> Result<Vec<ScalarBinding>, &'static str> {
    paths
        .iter()
        .map(|(path, word)| {
            let mut span = frames
                .fields_at(value as usize)
                .ok_or("scalar binding lacks frame")?;
            let mut leaf = None;
            for (depth, ordinal) in path.iter().enumerate() {
                let field = *span.get(*ordinal).ok_or("scalar binding leaves its span")?;
                if depth + 1 == path.len() {
                    leaf = Some(field);
                } else {
                    span = frames
                        .children(field)
                        .ok_or("scalar binding loses its child")?;
                }
            }
            let Some(Lane::Scalar { at, kind }) = leaf.map(|field| field.lane()) else {
                return Err("scalar binding is not a scalar leaf");
            };
            Ok(ScalarBinding {
                value,
                word: *word,
                physical: at,
                kind,
                steps: path.len(),
            })
        })
        .collect()
}
pub(in crate::native_artifact) use super::numeric_plan::Operation;
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct ColdAlias {
    pub value: u32,
    pub array: usize,
}
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct Proof<Frame = FrameWords> {
    pub nested_managed: bool,
    pub arrays: Vec<Array>,
    pub frames: Vec<Frame>,
    pub actions: Vec<(usize, Operation)>,
    pub sites: Vec<usize>,
    pub scalar_words: usize,
    pub scalar_inputs: Vec<ScalarBinding>,
    /// Read-only entry imports. They never participate in carrier restoration
    /// or successful export: the ancestor's physical lanes remain unchanged.
    pub scalar_captures: Vec<ScalarBinding>,
    pub scalar_outputs: Vec<ScalarBinding>,
    pub words: usize,
    /// Independently bounded source and shadow homes for entry/exit emission.
    pub boundary_words: usize,
    pub boundary_work: u64,
    pub result: ResultRecipe,
    /// Exact node-header continuation and current ancestor array values that
    /// ordinary execution can still consume without replaying their producer.
    pub cold_aliases: Vec<(usize, Vec<ColdAlias>)>,
    /// Exact immutable scalar Frame shadows needed by a final header fallback.
    /// These are restored only after the generated loan has finished.
    pub cold_scalars: Vec<(usize, Vec<ScalarBinding>)>,
    /// Exact current managed leaves of elided immutable ancestor frames.
    pub cold_frames: Vec<(usize, Vec<(usize, usize)>)>,
    /// Maximum appends before the next tree header or final exit. Includes the
    /// ancestor continuation after a child returns, not just its lexical body.
    pub append_limits: Vec<(usize, Vec<(usize, usize)>)>,
    pub prepaid_sites: Vec<usize>,
    pub phase: Option<super::numeric_phase::Schedule>,
    pub exits: Vec<super::numeric_plan::Exit>,
    pub fresh_values: Vec<u32>,
    pub boundary_values: super::numeric_plan::BoundaryValues,
}
impl<Frame: Into<FrameWords>> Proof<Frame> {
    pub(in crate::native_artifact) fn into_runtime(self) -> Proof {
        Proof {
            nested_managed: self.nested_managed,
            arrays: self.arrays,
            frames: self.frames.into_iter().map(Into::into).collect(),
            actions: self.actions,
            sites: self.sites,
            scalar_words: self.scalar_words,
            scalar_inputs: self.scalar_inputs,
            scalar_captures: self.scalar_captures,
            scalar_outputs: self.scalar_outputs,
            words: self.words,
            boundary_words: self.boundary_words,
            boundary_work: self.boundary_work,
            result: self.result,
            cold_aliases: self.cold_aliases,
            cold_scalars: self.cold_scalars,
            cold_frames: self.cold_frames,
            append_limits: self.append_limits,
            prepaid_sites: self.prepaid_sites,
            phase: self.phase,
            exits: self.exits,
            fresh_values: self.fresh_values,
            boundary_values: self.boundary_values,
        }
    }
}

impl Proof {
    /// Only current input lanes cross an initial entry. Internal frame lanes
    /// are defined by the selected source actions, not by the helper's scratch
    /// bank. Keep the full bank's admission and first-change transport intact.
    #[cfg(feature = "canonical-native-program")]
    #[cfg(test)]
    pub fn entry_words(&self, carrier: u32) -> Result<Vec<usize>, &'static str> {
        let mut words = self
            .frame(carrier)
            .ok_or("numeric entry lacks its carrier fields")?
            .scalar_slots()
            .collect::<Vec<_>>();
        for array in 0..self.arrays.len() {
            words.extend(self.view(array)..self.view(array) + 4);
            if self.writes() {
                words.push(self.private_flag(array));
            }
            words.extend(self.capacity(array));
        }
        if let Some(phase) = &self.phase {
            for debit in &phase.loops {
                words.push(debit.counter);
                words.extend(debit.first_visit);
            }
        }
        words.sort_unstable();
        words.dedup();
        if words.last().is_some_and(|word| *word >= self.words) {
            return Err("numeric entry input is outside its admitted word bank");
        }
        Ok(words)
    }

    pub fn action(&self, site: usize) -> Option<&Operation> {
        self.actions
            .iter()
            .find(|(s, _)| *s == site)
            .map(|(_, action)| action)
    }
    pub fn fields(&self, value: u32) -> Option<&[Option<usize>]> {
        self.frames
            .iter()
            .find(|f| f.value == value)
            .map(|f| f.fields.as_slice())
    }
    pub fn frame(&self, value: u32) -> Option<&FrameWords> {
        self.frames.iter().find(|frame| frame.value == value)
    }
    pub fn view(&self, array: usize) -> usize {
        self.scalar_words + array * 4
    }
    pub fn writes(&self) -> bool {
        self.arrays.iter().any(|array| array.writable)
    }
    pub fn private_flag(&self, array: usize) -> usize {
        self.scalar_words + self.arrays.len() * 4 + array
    }
    pub fn capacity(&self, array: usize) -> Option<usize> {
        self.arrays[array].appends.then(|| {
            self.scalar_words
                + self.arrays.len() * 5
                + self.arrays[..array].iter().filter(|a| a.appends).count()
        })
    }
    #[cfg(feature = "canonical-native-program")]
    pub fn append_limit(&self, header: usize, array: usize) -> Option<usize> {
        self.append_limits
            .iter()
            .find(|(site, _)| *site == header)?
            .1
            .iter()
            .find(|(at, _)| *at == array)
            .map(|(_, limit)| *limit)
    }
}

pub(super) fn width(body: &SchemaBody) -> Option<u8> {
    let SchemaBody::Matrix {
        element,
        dimensions,
    } = body
    else {
        return None;
    };
    if dimensions.len() != 2 {
        return None;
    }
    Some(match element.as_ref() {
        SchemaBody::UnsignedInteger(IntegerWidth::W8) => 1,
        SchemaBody::UnsignedInteger(IntegerWidth::W16) => 2,
        SchemaBody::UnsignedInteger(IntegerWidth::W32) => 4,
        SchemaBody::UnsignedInteger(IntegerWidth::W64) => 8,
        _ => return None,
    })
}

pub(in crate::native_artifact::program::prepare) fn scalar_children(
    frames: &NativeBufferFrames,
    field: mech_engine::native_artifact::NativeBufferFrameField,
) -> bool {
    match field.lane() {
        Lane::Scalar { .. } => true,
        Lane::Composite { .. } => frames.children(field).is_some_and(|children| {
            !children.is_empty() && children.iter().all(|child| scalar_children(frames, *child))
        }),
        Lane::Managed { .. } | Lane::Tagged { .. } => false,
    }
}

pub(in crate::native_artifact::program::prepare) fn scalar_paths(
    frames: &NativeBufferFrames,
    fields: &[mech_engine::native_artifact::NativeBufferFrameField],
    path: &mut Vec<usize>,
    words: &mut usize,
    output: &mut Vec<(Vec<usize>, usize)>,
) {
    for (ordinal, field) in fields.iter().enumerate() {
        path.push(ordinal);
        match field.lane() {
            Lane::Scalar { .. } => {
                output.push((path.clone(), *words));
                *words += 1;
            }
            Lane::Composite { .. } => scalar_paths(
                frames,
                frames.children(*field).unwrap(),
                path,
                words,
                output,
            ),
            Lane::Managed { .. } | Lane::Tagged { .. } => {}
        }
        path.pop();
    }
}

// A key denotes either an array Value or one managed field in a private frame.
// All definitions must agree. Cycles are rejected except the explicit carrier
// roots, whose next-state identity is checked separately for every field.
type Transfers = std::rc::Rc<[(numeric_fields::Key, numeric_fields::Key)]>;
struct Provenance<'a, 'p> {
    facts: &'a NativeProgramFacts<'p>,
    frames: &'a NativeBufferFrames,
    tail: &'a NativeTailLoop,
    calls: &'a [Option<NativePreparedLibraryCall>],
    // Exact untagged leaves already derived for this root. Rechecking a root
    // identity must not rebuild the same recursive structural vector.
    carrier_fields: &'a [mech_engine::native_artifact::NativeBufferFrameField],
    // Structural copy recipes depend only on this immutable plan and its
    // frame layouts. Cache the unfiltered recipe, never a freshness decision
    // or a root assignment (child roots are installed independently below).
    transfers: BTreeMap<usize, Result<Transfers, &'static str>>,
    cache: BTreeMap<(u32, Option<usize>), Option<usize>>,
    active: BTreeSet<(u32, Option<usize>)>,
    /// Child carrier fields are identity phis rooted through the exact seed.
    /// Their next/return values are independently checked before selection.
    child_roots: BTreeMap<(u32, usize), usize>,
    captures: BTreeMap<numeric_fields::Key, usize>,
    // Same-analysis identity facts only. No freshness state or image-provided
    // authority is cached; every branch/fixed-point meet still runs below.
    domain: Option<versions::Domain>,
}
impl Provenance<'_, '_> {
    fn transfers(&mut self, site: usize) -> Result<Transfers, &'static str> {
        if let Some(answer) = self.transfers.get(&site) {
            return answer.clone();
        }
        let answer =
            numeric_fields::transfers(self.facts.plan(), self.frames, site).map(std::rc::Rc::from);
        self.transfers.insert(site, answer.clone());
        answer
    }

    fn array(&mut self, value: u32, field: Option<usize>) -> Option<usize> {
        let key = (value, field);
        if let Some(root) = self.captures.get(&key) {
            return Some(*root);
        }
        if let Some(ordinal) = field {
            if let Some(root) = self.child_roots.get(&(value, ordinal)) {
                return Some(*root);
            }
        }
        if value == self.tail.carrier().get() {
            let ordinal = field?;
            return matches!(
                self.carrier_fields.get(ordinal)?.lane(),
                Lane::Managed { .. }
            )
            .then_some(ordinal);
        }
        if let Some(answer) = self.cache.get(&key) {
            return *answer;
        }
        if !self.active.insert(key) {
            return None;
        }
        let answer = self.definitions(value, field);
        self.active.remove(&key);
        self.cache.insert(key, answer);
        answer
    }
    fn definitions(&mut self, value: u32, field: Option<usize>) -> Option<usize> {
        let definitions = self.facts.definition_sites(value).ok()?.to_vec();
        if definitions.is_empty() {
            return None;
        }
        let mut selected = None;
        for definition in definitions {
            let site = definition.site()? as usize;
            let action = self.facts.plan().blocks()[site].action();
            let origin = if let Some(frame) = self.frames.action(site) {
                match (frame.operation(), field) {
                    (Frame::Pack { output }, Some(leaf)) if output.get() == value => {
                        let Action::LibraryCall { operands, .. } = action else {
                            return None;
                        };
                        let (ordinal, range) = (0..operands.len()).find_map(|ordinal| {
                            let range = numeric_fields::span(self.frames, value, ordinal)?;
                            range.contains(&leaf).then_some((ordinal, range))
                        })?;
                        let (source, field) = numeric_fields::operand(
                            self.frames,
                            *operands.get(ordinal)?,
                            leaf - range.start,
                        )?;
                        self.array(source, field)?
                    }
                    (Frame::Copy { source, output }, Some(ordinal)) if output.get() == value => {
                        self.array(source.get(), Some(ordinal))?
                    }
                    (
                        Frame::Project {
                            source,
                            ordinal,
                            output,
                        },
                        relative,
                    ) if output.get() == value => {
                        let range = numeric_fields::span(self.frames, source.get(), *ordinal)?;
                        let at = range.start.checked_add(relative.unwrap_or(0))?;
                        if !range.contains(&at) {
                            return None;
                        }
                        self.array(source.get(), Some(at))?
                    }
                    _ => return None,
                }
            } else if let (Action::Scalar(Scalar::Copy { source, output }), None) = (action, field)
            {
                if output.get() != value {
                    return None;
                }
                self.array(source.get(), None)?
            } else if let (
                Action::LibraryCall {
                    operands, output, ..
                },
                None,
            ) = (action, field)
            {
                let call = self.calls.get(site)?.as_ref()?;
                if output.get() != value
                    || !(call.supports_unsigned_matrix_replace()
                        || call.supports_unsigned_matrix_append())
                {
                    return None;
                }
                self.array(operands.first()?.get(), None)?
            } else {
                return None;
            };
            if selected.is_some_and(|old| old != origin) {
                return None;
            }
            selected = Some(origin);
        }
        selected
    }
}

pub(in crate::native_artifact) fn derive(
    plan: &Plan,
    tail: &NativeTailLoop,
    frames: &NativeBufferFrames,
    aggregates: &NativeScalarAggregates,
    calls: &[Option<NativePreparedLibraryCall>],
    visits: &[crate::native_artifact::iteration_abi::NativeIterationVisit],
) -> Result<Proof, &'static str> {
    super::phase_numeric::derive(plan, tail, frames, aggregates, calls, visits)
        .and_then(|phase| from_phase(plan, tail, frames, aggregates, phase))
        .or_else(|_| derive_leaf(plan, tail, frames, aggregates, calls))
}

#[cfg(test)]
pub(in crate::native_artifact) fn derive_fueled(
    plan: &Plan,
    tail: &NativeTailLoop,
    frames: &NativeBufferFrames,
    aggregates: &NativeScalarAggregates,
    calls: &[Option<NativePreparedLibraryCall>],
    visits: &[crate::native_artifact::iteration_abi::NativeIterationVisit],
) -> Result<Proof, &'static str> {
    from_phase(
        plan,
        tail,
        frames,
        aggregates,
        super::phase_numeric::derive_fueled(plan, tail, frames, aggregates, calls, visits)?,
    )
}
#[cfg(test)]
pub(in crate::native_artifact) fn derive_hybrid(
    plan: &Plan,
    tail: &NativeTailLoop,
    frames: &NativeBufferFrames,
    aggregates: &NativeScalarAggregates,
    calls: &[Option<NativePreparedLibraryCall>],
    visits: &[crate::native_artifact::iteration_abi::NativeIterationVisit],
) -> Result<Proof, &'static str> {
    from_phase(
        plan,
        tail,
        frames,
        aggregates,
        super::phase_numeric::derive_hybrid(plan, tail, frames, aggregates, calls, visits)?,
    )
}
#[cfg(test)]
pub(in crate::native_artifact) fn derive_interval(
    plan: &Plan,
    tail: &NativeTailLoop,
    frames: &NativeBufferFrames,
    aggregates: &NativeScalarAggregates,
    calls: &[Option<NativePreparedLibraryCall>],
    visits: &[crate::native_artifact::iteration_abi::NativeIterationVisit],
) -> Result<Proof, &'static str> {
    from_phase(
        plan,
        tail,
        frames,
        aggregates,
        super::phase_numeric::derive_interval(plan, tail, frames, aggregates, calls, visits)?,
    )
}

fn from_phase(
    plan: &Plan,
    tail: &NativeTailLoop,
    frames: &NativeBufferFrames,
    aggregates: &NativeScalarAggregates,
    phase: super::numeric_plan::Plan,
) -> Result<Proof, &'static str> {
    let carrier = frames
        .fields(tail.carrier())
        .ok_or("phase carrier lacks fields")?;
    let arrays = phase
        .arrays
        .into_iter()
        .map(|array| {
            let field = *carrier
                .get(array.field)
                .ok_or("phase array leaves carrier")?;
            let Lane::Managed { at: physical } = field.lane() else {
                return Err("phase array is not managed");
            };
            Ok(Array {
                root: array.field,
                storage: ArrayStorage::Carrier {
                    field: array.field,
                    physical,
                },
                schema: field.schema(),
                width: array.width,
                witness: array.witness,
                writable: array.writable,
                appends: array.appends,
            })
        })
        .collect::<Result<Vec<_>, &'static str>>()?;
    let frame_words = phase
        .frames
        .into_iter()
        .map(|frame| {
            let fields = frames
                .fields_at(frame.value as usize)
                .ok_or("phase frame lacks fields")?;
            if fields.len() != frame.fields.len() || fields.len() != frame.arrays.len() {
                return Err("phase frame span differs");
            }
            let scalar_paths = frame
                .fields
                .iter()
                .enumerate()
                .filter_map(|(at, word)| word.map(|word| (vec![at], word)))
                .collect::<Vec<_>>();
            let array_bindings = fields
                .iter()
                .zip(&frame.arrays)
                .filter_map(|(field, array)| match (field.lane(), array) {
                    (Lane::Managed { at }, Some(array)) => Some((at, *array)),
                    _ => None,
                })
                .collect();
            Ok(FrameWords::from(OwnedFrameWords {
                value: frame.value,
                maximum_arity: fields.len(),
                scalar_node_count: fields.len(),
                scalar_paths,
                fields: frame.fields,
                arrays: frame.arrays,
                array_bindings,
            }))
        })
        .collect::<Result<Vec<_>, &'static str>>()?;
    let bindings = |value| {
        frame_words
            .iter()
            .find(|frame| frame.value == value)
            .map_or_else(
                || Ok(Vec::new()),
                |frame| scalar_bindings(frames, value, &frame.scalar_paths),
            )
    };
    let scalar_inputs = bindings(tail.carrier().get())?;
    let scalar_outputs = phase
        .result
        .frame()
        .map_or_else(|| Ok(Vec::new()), bindings)?;
    let sites = phase.sites.iter().copied().collect::<BTreeSet<_>>();
    let boundary_words = register_bound(plan, aggregates, &sites)?
        .checked_add(phase.words)
        .and_then(|n| n.checked_add(3))
        .ok_or("phase boundary words overflow")?;
    let boundary_work = phase
        .phase
        .as_ref()
        .map_or(super::tail::FIXED_OVERHEAD, |schedule| {
            schedule.boundary_work
        });
    Ok(Proof {
        nested_managed: false,
        arrays,
        frames: frame_words,
        actions: phase.actions,
        sites: phase.sites,
        scalar_words: phase.scalar_words,
        scalar_inputs,
        scalar_captures: Vec::new(),
        scalar_outputs,
        words: phase.words,
        boundary_words,
        boundary_work,
        result: phase.result,
        cold_aliases: Vec::new(),
        cold_scalars: Vec::new(),
        cold_frames: Vec::new(),
        append_limits: Vec::new(),
        prepaid_sites: phase.prepaid_sites,
        phase: phase.phase,
        exits: phase.exits,
        fresh_values: phase.fresh_values,
        boundary_values: phase.boundary_values,
    })
}

fn derive_leaf(
    plan: &Plan,
    tail: &NativeTailLoop,
    frames: &NativeBufferFrames,
    aggregates: &NativeScalarAggregates,
    calls: &[Option<NativePreparedLibraryCall>],
) -> Result<Proof, &'static str> {
    derive_inner(plan, tail, frames, aggregates, calls, None)
}

pub(in crate::native_artifact) fn derive_tree(
    plan: &Plan,
    tail: &NativeTailLoop,
    frames: &NativeBufferFrames,
    aggregates: &NativeScalarAggregates,
    calls: &[Option<NativePreparedLibraryCall>],
    tree: &super::numeric_tree::Proof,
) -> Result<Proof, &'static str> {
    derive_inner(plan, tail, frames, aggregates, calls, Some(tree))
}

fn derive_inner(
    plan: &Plan,
    tail: &NativeTailLoop,
    frames: &NativeBufferFrames,
    aggregates: &NativeScalarAggregates,
    calls: &[Option<NativePreparedLibraryCall>],
    tree: Option<&super::numeric_tree::Proof>,
) -> Result<Proof, &'static str> {
    let fields = match numeric_fields::at(frames, tail.carrier().get()) {
        Some(fields) => fields,
        None if aggregates.slot(tail.carrier()).is_some()
            || plan.values()[tail.carrier().get() as usize]
                .scalar_kind()
                .is_some() =>
        {
            Vec::new()
        }
        None => return Err("numeric carrier lacks exact untagged leaves"),
    };
    let iteration = plan
        .iterations()
        .iter()
        .position(|c| matches!(c, NativeControlLoop::Tail(t) if t.header() == tail.header()))
        .ok_or("missing leaf iteration")? as u32;
    // The parent remains an ordinary suspended loop. A body traversal cannot
    // enter/exit any child or change the current invocation identity.
    let mut sites = BTreeSet::new();
    let mut pending = vec![tail.header()];
    while let Some(site) = pending.pop() {
        if site == tail.exit() || !sites.insert(site as usize) {
            continue;
        }
        let block = &plan.blocks()[site as usize];
        if (tree.map_or(block.iteration() != Some(iteration), |tree| {
            tree.node(site as usize).is_none()
        })) || plan.activation_boundary() == Some(site)
        {
            return Err("not a leaf: child loop or activation boundary");
        }
        match block.terminator() {
            Term::TailHeader { body, .. } | Term::Scalar(Branch::Jump(body)) => pending.push(body),
            Term::Scalar(
                Branch::Branch { yes, no, .. } | Branch::LiteralBranch { yes, no, .. },
            ) => pending.extend([yes, no]),
            Term::Scalar(Branch::Reject | Branch::IntegrityFailure) => {}
            _ => return Err("unsupported leaf terminator"),
        }
    }
    if !sites.contains(&(tail.latch() as usize)) || !sites.contains(&(tail.finish() as usize)) {
        return Err("incomplete leaf boundaries");
    }
    let facts = plan.facts();
    let mut provenance = Provenance {
        facts: &facts,
        frames,
        tail,
        calls,
        carrier_fields: &fields,
        transfers: BTreeMap::new(),
        cache: BTreeMap::new(),
        active: BTreeSet::new(),
        child_roots: BTreeMap::new(),
        captures: BTreeMap::new(),
        domain: None,
    };
    let mut captured = Vec::new();
    for &site in &sites {
        let mut sources = match plan.blocks()[site].action() {
            Action::LibraryCall { operands, .. } => operands.to_vec(),
            _ => Vec::new(),
        };
        if let Some(Frame::Copy { source, .. } | Frame::Project { source, .. }) =
            frames.action(site).map(|a| a.operation())
        {
            sources.push(*source);
        }
        for source in &sources {
            if super::captured_numeric::immutable_source(plan, tail, *source).is_err() {
                continue;
            }
            let mut candidates = Vec::new();
            if width(plan.values()[source.get() as usize].definition().body()).is_some() {
                candidates.push((
                    None,
                    ArrayStorage::Capture {
                        value: source.get(),
                    },
                    plan.values()[source.get() as usize].schema(),
                ));
            } else if let Some(leaves) = numeric_fields::at(frames, source.get()) {
                for (field, leaf) in leaves.iter().enumerate() {
                    if let Lane::Managed { at: physical } = leaf.lane() {
                        candidates.push((
                            Some(field),
                            ArrayStorage::FrameCapture {
                                value: source.get(),
                                field,
                                physical,
                            },
                            leaf.schema(),
                        ));
                    }
                }
            }
            for (field, storage, schema) in candidates {
                let key = (source.get(), field);
                if provenance.captures.contains_key(&key) {
                    continue;
                }
                let root = fields.len() + captured.len();
                provenance.captures.insert(key, root);
                captured.push(Array {
                    root,
                    storage,
                    schema,
                    width: 0,
                    witness: usize::MAX,
                    writable: false,
                    appends: false,
                });
            }
        }
    }
    if let Some(tree) = tree {
        for node in tree.nodes.iter().skip(1) {
            if let Some(fields) = numeric_fields::at(frames, node.tail.carrier().get()) {
                for (ordinal, field) in fields.iter().enumerate() {
                    if matches!(field.lane(), Lane::Managed { .. }) {
                        let root = provenance
                            .array(node.tail.seed().get(), Some(ordinal))
                            .ok_or("numeric child seed has an unknown array root")?;
                        provenance
                            .child_roots
                            .insert((node.tail.carrier().get(), ordinal), root);
                    }
                }
            } else if plan.values()[node.tail.carrier().get() as usize]
                .scalar_kind()
                .is_none()
                && aggregates.slot(node.tail.carrier()).is_none()
            {
                return Err("numeric child carrier lacks exact private scalar/frame words");
            }
        }
        // A failed cyclic lookup before a later child phi was known cannot be
        // reused once the complete declared identity roots have been installed.
        provenance.cache.clear();
    }
    let mut arrays = Vec::new();
    for (ordinal, field) in fields.iter().enumerate() {
        if matches!(field.lane(), Lane::Managed { .. }) {
            if provenance.array(tail.next().get(), Some(ordinal)) != Some(ordinal) {
                return Err("array field changes, swaps, or has unknown recursive forwarding");
            }
            arrays.push(Array {
                root: ordinal,
                storage: ArrayStorage::Carrier {
                    field: ordinal,
                    physical: match field.lane() {
                        Lane::Managed { at } => at,
                        _ => unreachable!(),
                    },
                },
                schema: field.schema(),
                width: 0,
                witness: usize::MAX,
                writable: false,
                appends: false,
            });
        }
    }
    arrays.extend(captured);
    if arrays.is_empty() {
        return Err("no invariant numeric arrays");
    }
    let result = results::derive(
        plan,
        tail,
        frames,
        aggregates,
        &fields,
        &arrays,
        &mut provenance,
    )?;
    let mut actions = Vec::new();
    let mut frame_values = BTreeSet::new();
    if numeric_fields::at(frames, tail.carrier().get()).is_some() {
        frame_values.insert(tail.carrier().get());
    }
    if let Some(value) = result.frame() {
        frame_values.insert(value);
    }
    for &site in &sites {
        let action = plan.blocks()[site].action();
        if let Action::LibraryCall { template, .. } = action {
            if !calls
                .get(site)
                .and_then(Option::as_ref)
                .is_some_and(|call| call.template() == template)
            {
                return Err("selected factory is not bound to this exact source site");
            }
        }
        let operation = if let Some(frame) = frames.action(site) {
            let mut remember = |value: u32| -> Result<(), &'static str> {
                let fields = numeric_fields::at(frames, value)
                    .ok_or("numeric frame lost exact untagged leaves")?;
                for (ordinal, field) in fields.iter().enumerate() {
                    if matches!(field.lane(), Lane::Managed { .. }) {
                        let origin = provenance.array(value, Some(ordinal));
                        let array = arrays
                            .iter()
                            .find(|a| origin == Some(a.root))
                            .ok_or("frame has an unknown managed field")?;
                        // A physical owner binding forwards this exact Value;
                        // schema rebinding requires the ordinary constructor.
                        if field.schema() != array.schema {
                            return Err("numeric frame leaf requires schema rebinding");
                        }
                    }
                }
                frame_values.insert(value);
                Ok(())
            };
            match frame.operation() {
                Frame::Pack { output } => {
                    if !calls
                        .get(site)
                        .and_then(Option::as_ref)
                        .is_some_and(|call| call.supports_invariant_buffer_frame())
                    {
                        return Err("frame has nonidentity or constrained shape relations");
                    }
                    remember(output.get())?;
                }
                Frame::Copy { source, output } => {
                    remember(source.get())?;
                    remember(output.get())?;
                }
                Frame::Project { source, output, .. } => {
                    remember(source.get())?;
                    if frames.fields(*output).is_some() {
                        remember(output.get())?;
                    }
                }
                Frame::EnumPack { .. } | Frame::Observe { .. } => {
                    return Err("tagged frame is outside numeric array proof");
                }
            }
            Operation::Frame
        } else if let Some(action) = aggregates.action(site) {
            let mut values = Vec::new();
            match action {
                Aggregate::Pack { output, words } => {
                    values.push(*output);
                    values.extend(words.iter().filter_map(|word| match word {
                        NativeAggregateWord::Field(value, _) => Some(*value),
                        _ => None,
                    }));
                }
                Aggregate::Copy { source, output } | Aggregate::Project { source, output, .. } => {
                    values.extend([*source, *output]);
                }
                Aggregate::Observe { source, .. } => values.push(*source),
            }
            if !values.into_iter().all(|value| {
                results::scalar_aggregate(plan, aggregates, value)
                    // Internal scalar FSM tags/payloads keep exactly the same
                    // same-plan owner and guarded common emitter as ordinary
                    // scalar execution. No managed word enters this component.
                    || (tree.is_some() && aggregates.slot(value).is_some())
            }) {
                return Err("numeric aggregate requires exact private flat scalar fields");
            }
            Operation::Aggregate
        } else if let Action::LibraryCall {
            operands,
            output,
            template,
        } = action
        {
            let call = calls
                .get(site)
                .and_then(Option::as_ref)
                .ok_or("missing selected factory")?;
            let is_length = call.supports_unsigned_matrix_length();
            let is_read = call.supports_unsigned_matrix_element();
            let is_write = call.supports_unsigned_matrix_replace();
            let is_append = call.supports_unsigned_matrix_append();
            if !(is_length || is_read || is_write || is_append) {
                return Err("unsupported selected library effect or operation");
            }
            let source = operands.first().ok_or("missing selected array")?;
            let field = provenance
                .array(source.get(), None)
                .ok_or("array read is not an invariant carrier field")?;
            let at = arrays
                .iter()
                .position(|a| a.root == field)
                .ok_or("array outside carrier")?;
            if plan.values()[source.get() as usize].schema() != arrays[at].schema {
                return Err("array root schema differs from its selected use");
            }
            if !matches!(arrays[at].storage, ArrayStorage::Carrier { .. })
                && (is_write || is_append)
            {
                return Err("immutable captured array cannot be written");
            }
            let w =
                width(template.inputs()[0].body()).ok_or("unsupported numeric shape or width")?;
            if arrays[at].width != 0 && arrays[at].width != w {
                return Err("array field width differs");
            }
            arrays[at].width = w;
            arrays[at].witness = arrays[at].witness.min(site);
            arrays[at].writable |= is_write || is_append;
            arrays[at].appends |= is_append;
            if is_length {
                Operation::Length {
                    array: at,
                    output: output.get(),
                }
            } else if is_append {
                if operands.len() != 2
                    || plan.values()[operands[1].get() as usize]
                        .scalar_kind()
                        .and_then(|kind| kind.unsigned_maximum())
                        != Some(match w {
                            1 => u8::MAX as u64,
                            2 => u16::MAX as u64,
                            4 => u32::MAX as u64,
                            8 => u64::MAX,
                            _ => unreachable!(),
                        })
                {
                    return Err("unsupported append element kind");
                }
                Operation::Append {
                    array: at,
                    incoming: operands[1].get(),
                }
            } else if is_write {
                if operands.len() != 3
                    || plan.values()[operands[2].get() as usize].scalar_kind()
                        != Some(NativeScalarKind::U64)
                    || plan.values()[operands[1].get() as usize]
                        .scalar_kind()
                        .and_then(|k| k.unsigned_maximum())
                        .is_none()
                {
                    return Err("unsupported write operands");
                }
                Operation::Write {
                    array: at,
                    incoming: operands[1].get(),
                    index: operands[2].get(),
                }
            } else {
                if operands.len() != 2
                    || plan.values()[operands[1].get() as usize].scalar_kind()
                        != Some(NativeScalarKind::U64)
                {
                    return Err("unsupported read selector");
                }
                Operation::Read {
                    array: at,
                    index: operands[1].get(),
                    output: output.get(),
                }
            }
        } else {
            match action {
                Action::Scalar(Scalar::None | Scalar::Primitive { .. }) => continue,
                Action::Scalar(Scalar::Copy { output, .. })
                    if plan.values()[output.get() as usize].scalar_kind().is_some() =>
                {
                    continue;
                }
                Action::Scalar(Scalar::Copy { source, output })
                    if provenance.array(source.get(), None).is_some()
                        && provenance.array(source.get(), None)
                            == provenance.array(output.get(), None) =>
                {
                    Operation::ArrayCopy
                }
                _ => return Err("unsupported leaf action or managed escape"),
            }
        };
        actions.push((site, operation));
    }
    // A retained leaf with no selected content operation remains an opaque
    // immutable owner. It grants no native pointer/view and cannot be written.
    // Current-version proof still covers every structural forwarding edge.
    if arrays.iter().any(|array| {
        array.witness == usize::MAX && matches!(array.storage, ArrayStorage::Carrier { .. })
    }) && !frames
        .fields(tail.carrier())
        .is_some_and(|fields| numeric_fields::has_nested_managed(frames, fields))
    {
        // Opaque retention belongs to the new recursive managed-frame proof.
        // Keep the established flat-frame eligibility and callback controls.
        return Err("unwitnessed retained array");
    }
    if arrays.iter().any(|array| {
        array.witness == usize::MAX && (array.width != 0 || array.writable || array.appends)
    }) {
        return Err("unwitnessed retained array has content effects");
    }
    let cold_homes = if let Some(tree) = tree {
        for node in tree.nodes.iter().skip(1) {
            if let Some(fields) = numeric_fields::at(frames, node.tail.carrier().get()) {
                for (ordinal, field) in fields.iter().enumerate() {
                    if matches!(field.lane(), Lane::Managed { .. })
                        && provenance.array(node.tail.next().get(), Some(ordinal))
                            != provenance
                                .child_roots
                                .get(&(node.tail.carrier().get(), ordinal))
                                .copied()
                    {
                        return Err("numeric child recursive edge changes its array root");
                    }
                }
            }
        }
        versions::validate_tree(
            plan,
            tail,
            frames,
            &mut provenance,
            &sites,
            &actions,
            &arrays,
            &result,
            tree,
        )?
    } else {
        versions::validate(
            plan,
            tail,
            frames,
            &mut provenance,
            &sites,
            &actions,
            &arrays,
            &result,
        )?;
        versions::ColdHomes::default()
    };
    let append_limits = if let Some(tree) = tree {
        append::derive(plan, tree, &sites, &actions, &arrays)?
    } else {
        Vec::new()
    };
    let mut scalar_words = 0;
    let frame_words: Vec<FrameWords> = frame_values
        .into_iter()
        .map(|value| {
            let mut paths = Vec::new();
            scalar_paths(
                frames,
                frames.fields_at(value as usize).unwrap(),
                &mut Vec::new(),
                &mut scalar_words,
                &mut paths,
            );
            let fields = frames
                .fields_at(value as usize)
                .unwrap()
                .iter()
                .enumerate()
                .map(|(ordinal, field)| match field.lane() {
                    Lane::Scalar { .. } => paths
                        .iter()
                        .find(|(path, _)| path.as_slice() == [ordinal])
                        .map(|(_, slot)| *slot),
                    Lane::Managed { .. } | Lane::Composite { .. } | Lane::Tagged { .. } => None,
                })
                .collect();
            let flat_fields = numeric_fields::at(frames, value).unwrap();
            let array_fields = flat_fields
                .iter()
                .enumerate()
                .map(|(ordinal, field)| {
                    if matches!(field.lane(), Lane::Managed { .. }) {
                        let origin = provenance.array(value, Some(ordinal)).unwrap();
                        Some(arrays.iter().position(|a| a.root == origin).unwrap())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            let array_bindings = flat_fields
                .iter()
                .zip(&array_fields)
                .filter_map(|(field, array)| match (field.lane(), array) {
                    (Lane::Managed { at }, Some(array)) => Some((at, *array)),
                    _ => None,
                })
                .collect();
            FrameWords::from(OwnedFrameWords {
                value,
                fields,
                arrays: array_fields,
                array_bindings,
                maximum_arity: numeric_fields::maximum_arity(
                    frames,
                    frames.fields_at(value as usize).unwrap(),
                ),
                scalar_node_count: numeric_fields::logical_nodes(
                    frames,
                    frames.fields_at(value as usize).unwrap(),
                ),
                scalar_paths: paths,
            })
        })
        .collect();
    // Tree entries pass their scalar-bank pointer in argument word two.
    // A frame-free scalar tree has no scalar lanes to occupy that word, so
    // reserve its transport slot before placing the first array view there.
    // No frame allocation or access authority is implied by this padding.
    if tree.is_some() && scalar_words == 0 {
        scalar_words = 1;
    }
    let words = scalar_words
        .checked_add(
            arrays
                .len()
                .checked_mul(4)
                .ok_or("numeric view size overflows")?,
        )
        .and_then(|n| {
            n.checked_add(if arrays.iter().any(|a| a.writable) {
                arrays.len()
            } else {
                0
            })
        })
        .and_then(|n| n.checked_add(arrays.iter().filter(|a| a.appends).count()))
        .ok_or("numeric word count overflows")?;
    let boundary_words = register_bound(plan, aggregates, &sites)?
        .checked_add(words)
        .and_then(|n| n.checked_add(3))
        .ok_or("numeric boundary register count overflows")?;
    let recursive = frame_words
        .iter()
        .any(|frame: &FrameWords| frame.scalar_paths.iter().any(|(path, _)| path.len() > 1));
    let boundary_work = if recursive {
        // One entry load and three possible spill templates per admitted home;
        // the cold leave additionally loads/exports each carrier scalar. The
        // fixed allowance covers guards, fuel settlement and branch templates.
        // Keep the original flat-frame price and its exact regression receipts.
        let carrier = frame_words
            .iter()
            .find(|frame| frame.value == tail.carrier().get());
        (boundary_words as u64)
            .checked_mul(4)
            .and_then(|n| {
                n.checked_add(carrier.map_or(0, |carrier| carrier.scalar_paths.len() as u64 * 2))
            })
            .and_then(|n| n.checked_add(128))
            .ok_or("numeric boundary work overflows")?
            .max(super::tail::FIXED_OVERHEAD)
    } else {
        super::tail::FIXED_OVERHEAD
    };
    let mut scalar_inputs = Vec::new();
    let mut scalar_captures = Vec::new();
    for frame in &frame_words {
        if frame.value == tail.carrier().get() {
            scalar_inputs.extend(scalar_bindings(frames, frame.value, &frame.scalar_paths)?);
        } else if super::captured_numeric::immutable_source_id(plan, tail, frame.value).is_ok() {
            scalar_captures.extend(scalar_bindings(frames, frame.value, &frame.scalar_paths)?);
        }
    }
    let scalar_outputs = if let Some(value) = result.frame() {
        let returned = frame_words
            .iter()
            .find(|frame| frame.value == value)
            .ok_or("numeric scalar result recipe missing")?;
        scalar_bindings(frames, value, &returned.scalar_paths)?
    } else {
        Vec::new()
    };
    let mut cold_scalars = Vec::new();
    let mut cold_frames = Vec::new();
    for (header, values) in cold_homes.scalars {
        let mut bindings = Vec::new();
        let mut owners = Vec::new();
        for value in values {
            let frame = frame_words
                .iter()
                .find(|frame| frame.value == value)
                .ok_or("numeric cold frame lost its shadow recipe")?;
            bindings.extend(scalar_bindings(frames, value, &frame.scalar_paths)?);
            owners.extend(frame.array_bindings.iter().copied());
        }
        if !bindings.is_empty() {
            cold_scalars.push((header, bindings));
        }
        if !owners.is_empty() {
            cold_frames.push((header, owners));
        }
    }
    let nested_managed = frame_words.iter().any(|frame| {
        numeric_fields::has_nested_managed(frames, frames.fields_at(frame.value as usize).unwrap())
    });
    Ok(Proof {
        nested_managed,
        arrays,
        frames: frame_words,
        actions,
        sites: sites.into_iter().collect(),
        scalar_words,
        scalar_inputs,
        scalar_captures,
        scalar_outputs,
        words,
        boundary_words,
        boundary_work,
        result,
        cold_aliases: cold_homes.aliases,
        cold_scalars,
        cold_frames,
        append_limits,
        prepaid_sites: Vec::new(),
        phase: None,
        exits: Vec::new(),
        fresh_values: Vec::new(),
        boundary_values: super::numeric_plan::BoundaryValues {
            candidates: Vec::new(),
            header: Vec::new(),
            exit: Vec::new(),
        },
    })
}

/// Conservative static inventory, independent of backend instructions. Whole
/// aggregate spans may overcount a projection; no source scalar home is omitted.
fn register_bound(
    plan: &Plan,
    aggregates: &NativeScalarAggregates,
    sites: &BTreeSet<usize>,
) -> Result<usize, &'static str> {
    let mut words = BTreeSet::new();
    let span = |value, words: &mut BTreeSet<usize>| -> Result<(), &'static str> {
        let (base, count) = aggregates
            .slot(value)
            .ok_or("numeric aggregate span missing")?;
        words.extend(
            base..base
                .checked_add(count)
                .ok_or("numeric aggregate span overflows")?,
        );
        Ok(())
    };
    for &site in sites {
        if let Some(action) = aggregates.action(site) {
            match action {
                Aggregate::Pack {
                    output,
                    words: inputs,
                } => {
                    span(*output, &mut words)?;
                    for input in inputs {
                        match input {
                            NativeAggregateWord::Scalar(value) => {
                                words.insert(value.get() as usize);
                            }
                            NativeAggregateWord::Field(value, _) => span(*value, &mut words)?,
                            NativeAggregateWord::Constant(_) => {}
                        }
                    }
                }
                Aggregate::Copy { source, output } | Aggregate::Project { source, output, .. } => {
                    span(*source, &mut words)?;
                    span(*output, &mut words)?;
                }
                Aggregate::Observe { source, output, .. } => {
                    span(*source, &mut words)?;
                    words.insert(output.get() as usize);
                }
            }
        }
        let mut scalar = |value: mech_engine::native_artifact::NativeMatchValue| {
            if plan.values()[value.get() as usize].scalar_kind().is_some() {
                words.insert(value.get() as usize);
            }
        };
        match plan.blocks()[site].action() {
            Action::Scalar(Scalar::Copy { source, output }) => {
                scalar(*source);
                scalar(*output);
            }
            Action::Scalar(Scalar::Primitive {
                operands, output, ..
            })
            | Action::LibraryCall {
                operands, output, ..
            } => {
                for operand in operands {
                    scalar(*operand);
                }
                scalar(*output);
            }
            Action::Structural { output, .. } => scalar(*output),
            _ => {}
        }
        match plan.blocks()[site].terminator() {
            Term::Scalar(Branch::Branch { condition, .. }) => scalar(condition),
            Term::Scalar(Branch::LiteralBranch { left, right, .. }) => {
                scalar(left);
                scalar(right);
            }
            _ => {}
        }
    }
    Ok(words.len())
}
