//! Borrowed structural descent for activation-proved fixed scalar bindings.
//!
//! The private proof covers only zero-parameter Enum/Tuple trees and exact
//! Bool/Index/U64/F64 leaves. It retains the immutable source root and creates
//! drafts only for selected scalar bindings. General patterns keep their
//! existing owned materialization path.

use super::super::{
    ActivatedMatchArm, ActivatedMatchPattern, ActivatedPatternBinding, ResidentRegion,
};
use crate::resident::budget::{self, KernelCostEstimate, PreparedKernel, ResidentBudgetMeter};
use mech_core::{
    CurrentMemoryFootprint, ResidentKernelError, ResidentShape, ResidentValueKind,
    ResidentValueRef, SchemaBody, SchemaId, SchemaKey, SchemaTable, Value, ValueData,
    ValueDataDraft,
};
use std::sync::Arc;

const MAX_DEPTH: usize = 32;
const HEADER_WORK: u64 = 64;

type Pattern =
    crate::CollectionPattern<ActivatedPatternBinding, super::super::ActivatedPatternValue>;

#[derive(Clone, Copy, Debug)]
pub(super) enum Scalar {
    Bool,
    Index,
    U64,
    F64,
}

impl Scalar {
    fn from_body(body: &SchemaBody) -> Option<Self> {
        Some(match body {
            SchemaBody::Bool => Self::Bool,
            SchemaBody::Index => Self::Index,
            SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64) => Self::U64,
            SchemaBody::FloatingPoint(mech_core::FloatWidth::W64) => Self::F64,
            _ => return None,
        })
    }

    fn matches(self, data: &ValueData) -> bool {
        matches!(
            (self, data),
            (Self::Bool, ValueData::Bool(_))
                | (Self::Index, ValueData::Index(_))
                | (Self::U64, ValueData::U64(_))
                | (Self::F64, ValueData::F64(_))
        )
    }

    fn draft(self, data: &ValueData) -> Option<ValueDataDraft> {
        Some(match (self, data) {
            (Self::Bool, ValueData::Bool(value)) => ValueDataDraft::Bool(*value),
            (Self::Index, ValueData::Index(value)) => ValueDataDraft::Index(*value),
            (Self::U64, ValueData::U64(value)) => ValueDataDraft::U64(*value),
            (Self::F64, ValueData::F64(value)) => ValueDataDraft::F64(*value),
            _ => return None,
        })
    }

    fn width(self) -> u64 {
        match self {
            Self::Bool => 1,
            Self::Index | Self::U64 | Self::F64 => 8,
        }
    }

    fn accepts_region(self, region: ResidentRegion) -> bool {
        region.shape == ResidentShape::SCALAR
            && region.len == 1
            && (region.kind == ResidentValueKind::Snapshot
                || matches!(
                    (self, region.kind),
                    (Self::Bool, ResidentValueKind::Bool)
                        | (Self::Index, ResidentValueKind::Index)
                        | (Self::F64, ResidentValueKind::F64)
                ))
    }
}

fn supported_tree(body: &SchemaBody, depth: usize) -> bool {
    if depth >= MAX_DEPTH {
        return false;
    }
    if Scalar::from_body(body).is_some() {
        return true;
    }
    match body {
        SchemaBody::Tuple(children) => children.iter().all(|body| supported_tree(body, depth + 1)),
        SchemaBody::Enum { variants, .. } => variants.iter().all(|variant| {
            variant
                .payload
                .as_ref()
                .is_none_or(|body| supported_tree(body, depth + 1))
        }),
        _ => false,
    }
}

#[derive(Debug)]
enum ScalarPatternTree {
    Wildcard,
    Bind {
        binding: ActivatedPatternBinding,
        scalar: Scalar,
    },
    Enum {
        ordinal: u32,
        payload: Option<Box<Self>>,
    },
    Tuple(Box<[Self]>),
}

impl ScalarPatternTree {
    fn qualify(
        pattern: &Pattern,
        body: &SchemaBody,
        schemas: &SchemaTable,
        depth: usize,
    ) -> Option<Self> {
        if depth >= MAX_DEPTH {
            return None;
        }
        Some(match pattern {
            Pattern::Wildcard => Self::Wildcard,
            Pattern::Bind {
                schema: binding, ..
            } => {
                let scalar = Scalar::from_body(body)?;
                let target = schemas.get(binding.schema)?;
                if !target.dimension_parameters().is_empty()
                    || target.body() != body
                    || !scalar.accepts_region(binding.region)
                {
                    return None;
                }
                Self::Bind {
                    binding: *binding,
                    scalar,
                }
            }
            Pattern::Enum { ordinal, payload } => {
                let SchemaBody::Enum { variants, .. } = body else {
                    return None;
                };
                let variant = variants.get(*ordinal as usize)?;
                let payload = match (payload.as_deref(), variant.payload.as_ref()) {
                    (None, None) => None,
                    (Some(pattern), Some(body)) => {
                        Some(Box::new(Self::qualify(pattern, body, schemas, depth + 1)?))
                    }
                    _ => return None,
                };
                Self::Enum {
                    ordinal: *ordinal,
                    payload,
                }
            }
            Pattern::Tuple(patterns) => {
                let SchemaBody::Tuple(children) = body else {
                    return None;
                };
                if patterns.len() != children.len() {
                    return None;
                }
                let mut selected = Vec::with_capacity(patterns.len());
                for (pattern, body) in patterns.iter().zip(children) {
                    selected.push(Self::qualify(pattern, body, schemas, depth + 1)?);
                }
                Self::Tuple(selected.into_boxed_slice())
            }
            Pattern::Equal(_) | Pattern::Array { .. } => return None,
        })
    }

    pub(super) fn matches(&self, data: &ValueData) -> bool {
        match (self, data) {
            (Self::Wildcard, _) => true,
            (Self::Bind { scalar, .. }, data) => scalar.matches(data),
            (Self::Enum { ordinal, payload }, ValueData::Enum(value))
                if *ordinal == value.ordinal() =>
            {
                match (payload.as_deref(), value.payload()) {
                    (None, None) => true,
                    (Some(pattern), Some(data)) => pattern.matches(data),
                    _ => false,
                }
            }
            (Self::Tuple(patterns), ValueData::Tuple(values)) if patterns.len() == values.len() => {
                patterns
                    .iter()
                    .zip(values)
                    .all(|(pattern, data)| pattern.matches(data))
            }
            _ => false,
        }
    }

    fn visit_bindings<E>(
        &self,
        visit: &mut impl FnMut(ActivatedPatternBinding, Scalar) -> Result<(), E>,
    ) -> Result<(), E> {
        match self {
            Self::Wildcard => Ok(()),
            Self::Bind { binding, scalar } => visit(*binding, *scalar),
            Self::Enum { payload, .. } => payload
                .as_deref()
                .map_or(Ok(()), |pattern| pattern.visit_bindings(visit)),
            Self::Tuple(patterns) => {
                for pattern in patterns {
                    pattern.visit_bindings(visit)?;
                }
                Ok(())
            }
        }
    }

    pub(super) fn visit_regions<E>(
        &self,
        visit: &mut impl FnMut(ResidentRegion) -> Result<(), E>,
    ) -> Result<(), E> {
        self.visit_bindings(&mut |binding, _| visit(binding.region))
    }

    pub(super) fn bind_values<E>(
        &self,
        data: &ValueData,
        visit: &mut impl FnMut(ActivatedPatternBinding, ValueDataDraft) -> Result<(), E>,
    ) -> Result<(), E> {
        // Called only after matches() succeeds for this same immutable root.
        match (self, data) {
            (Self::Wildcard, _) => Ok(()),
            (Self::Bind { binding, scalar }, data) => visit(
                *binding,
                scalar
                    .draft(data)
                    .expect("proved scalar leaf remains immutable"),
            ),
            (Self::Enum { payload, .. }, ValueData::Enum(value)) => {
                match (payload.as_deref(), value.payload()) {
                    (None, None) => Ok(()),
                    (Some(pattern), Some(data)) => pattern.bind_values(data, visit),
                    _ => unreachable!("matched enum payload remains immutable"),
                }
            }
            (Self::Tuple(patterns), ValueData::Tuple(values)) => {
                for (pattern, data) in patterns.iter().zip(values) {
                    pattern.bind_values(data, visit)?;
                }
                Ok(())
            }
            _ => unreachable!("matched structure remains immutable"),
        }
    }

    fn nodes(&self) -> Option<u64> {
        let children = match self {
            Self::Wildcard | Self::Bind { .. } => 0,
            Self::Enum { payload, .. } => payload.as_deref().map_or(Some(0), Self::nodes)?,
            Self::Tuple(patterns) => patterns
                .iter()
                .try_fold(0_u64, |n, pattern| n.checked_add(pattern.nodes()?))?,
        };
        children.checked_add(1)
    }

    /// Covers the two borrowed walks and the selected scalar draft/finalizer
    /// work. There is no whole-source payload/schema/draft clone on this path.
    fn cost(
        &self,
        prior: mech_core::snapshot::ValueFootprint,
        metadata: KernelCostEstimate,
    ) -> Result<KernelCostEstimate, ResidentKernelError> {
        let fail = || ResidentKernelError::InvalidShape;
        let mut temporary = prior.retained_bytes;
        let mut retained = prior.node_count;
        let mut copied = 0_u64;
        let mut compute = self
            .nodes()
            .ok_or_else(fail)?
            .checked_mul(4 * HEADER_WORK)
            .ok_or_else(fail)?;
        self.visit_bindings(&mut |binding, scalar| {
            let draft = core::mem::size_of::<ValueDataDraft>() as u64;
            copied = copied.checked_add(scalar.width()).ok_or_else(fail)?;
            compute = compute
                .checked_add(draft)
                .and_then(|n| n.checked_add(scalar.width()))
                .ok_or_else(fail)?;
            if binding.region.kind == ResidentValueKind::Snapshot {
                let witness = CurrentMemoryFootprint {
                    logical_elements: 1,
                    payload_bytes: core::mem::size_of::<ValueData>() as u64,
                    encoded_bytes: scalar.width(),
                    retained_nodes: 2,
                    // The finalizer borrows the existing shared schema arena.
                    schema_bytes: 0,
                    shape_parameter_count: 0,
                    ..CurrentMemoryFootprint::default()
                };
                let final_bytes = mech_core::canonical_snapshot_finalization_bytes(witness)
                    .map_err(|_| fail())?
                    .checked_add(Value::memory_budget_claim_metadata_bytes())
                    .ok_or_else(fail)?;
                let draft_bytes =
                    mech_core::canonical_snapshot_draft_bytes(witness).map_err(|_| fail())?;
                temporary = temporary
                    .checked_add(final_bytes)
                    .and_then(|n| n.checked_add(draft_bytes))
                    .ok_or_else(fail)?;
                retained = retained.checked_add(2).ok_or_else(fail)?;
                compute = compute
                    .checked_add(final_bytes)
                    .and_then(|n| n.checked_add(4 * HEADER_WORK))
                    .ok_or_else(fail)?;
            }
            Ok::<_, ResidentKernelError>(())
        })?;
        Ok(budget::resident_cost! {
            comparison_work: metadata.comparison_work().checked_add(self.nodes().ok_or_else(fail)?).ok_or_else(fail)?,
            compute_work: metadata.compute_work().checked_add(compute).ok_or_else(fail)?,
            temporary_bytes: temporary,
            cloned_bytes: copied,
            retained_nodes: retained,
            ..KernelCostEstimate::default()
        })
    }
}

/// Runtime owns no mutable projection plan. Counts and construction bounds
/// are cached at activation so cost preparation cannot walk an unadmitted tree.
#[derive(Debug)]
pub(super) struct BorrowedPattern {
    tree: ScalarPatternTree,
    cost: KernelCostEstimate,
    regions: Box<[ResidentRegion]>,
}

impl BorrowedPattern {
    fn qualify(
        pattern: &Pattern,
        body: &SchemaBody,
        schemas: &SchemaTable,
        depth: usize,
    ) -> Option<Self> {
        let tree = ScalarPatternTree::qualify(pattern, body, schemas, depth)?;
        let cost = tree
            .cost(
                mech_core::snapshot::ValueFootprint::zero(),
                KernelCostEstimate::default(),
            )
            .ok()?;
        let mut binding_count = 0_usize;
        tree.visit_regions(&mut |_| {
            binding_count = binding_count.checked_add(1).ok_or(())?;
            Ok::<_, ()>(())
        })
        .ok()?;
        let mut regions = Vec::with_capacity(binding_count);
        tree.visit_regions(&mut |region| {
            regions.push(region);
            Ok::<_, ()>(())
        })
        .ok()?;
        Some(Self {
            tree,
            cost,
            regions: regions.into_boxed_slice(),
        })
    }

    pub(super) fn matches(&self, data: &ValueData) -> bool {
        self.tree.matches(data)
    }

    pub(super) fn visit_regions<E>(
        &self,
        visit: &mut impl FnMut(ResidentRegion) -> Result<(), E>,
    ) -> Result<(), E> {
        for &region in &self.regions {
            visit(region)?;
        }
        Ok(())
    }

    pub(super) fn bind_values<E>(
        &self,
        data: &ValueData,
        visit: &mut impl FnMut(ActivatedPatternBinding, ValueDataDraft) -> Result<(), E>,
    ) -> Result<(), E> {
        self.tree.bind_values(data, visit)
    }

    pub(super) fn cost(
        &self,
        prior: mech_core::snapshot::ValueFootprint,
        metadata: KernelCostEstimate,
    ) -> Result<KernelCostEstimate, ResidentKernelError> {
        Ok(budget::resident_cost! {
            comparison_work: self.cost.comparison_work().checked_add(metadata.comparison_work()).ok_or(ResidentKernelError::InvalidShape)?,
            compute_work: self.cost.compute_work().checked_add(metadata.compute_work()).ok_or(ResidentKernelError::InvalidShape)?,
            temporary_bytes: self.cost.temporary_bytes().checked_add(prior.retained_bytes).ok_or(ResidentKernelError::InvalidShape)?,
            cloned_bytes: self.cost.cloned_bytes(),
            retained_nodes: self.cost.retained_nodes().checked_add(prior.node_count).ok_or(ResidentKernelError::InvalidShape)?,
            ..KernelCostEstimate::default()
        })
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct MetadataBound {
    bytes: u64,
    nodes: u64,
}

impl MetadataBound {
    fn add(self, other: Self) -> Option<Self> {
        Some(Self {
            bytes: self.bytes.checked_add(other.bytes)?,
            nodes: self.nodes.checked_add(other.nodes)?,
        })
    }

    fn slice<T>(len: usize) -> Option<Self> {
        Some(Self {
            bytes: u64::try_from(len)
                .ok()?
                .checked_mul(core::mem::size_of::<T>() as u64)?,
            nodes: u64::from(len != 0),
        })
    }

    // ArcInner uses two atomic counters before the aligned data. Round both
    // boundaries, covering any padding rather than assuming a 16-byte header.
    fn owner<T>() -> Option<Self> {
        let align = core::mem::align_of::<T>().max(core::mem::align_of::<usize>()) as u64;
        let round = |n: u64| n.checked_add(align - 1).map(|n| n / align * align);
        Some(Self {
            bytes: round(
                round(2 * core::mem::size_of::<usize>() as u64)?
                    .checked_add(core::mem::size_of::<T>() as u64)?,
            )?,
            nodes: 1,
        })
    }

    fn construction(self) -> Option<Self> {
        // Each exact-capacity Vec may coexist with its packed Box during
        // conversion. Tree nodes, region slices and the final Arc are included
        // in both populations, conservatively covering all nested conversions.
        Some(Self {
            bytes: self.bytes.checked_mul(2)?,
            nodes: self.nodes.checked_mul(2)?,
        })
    }
}

// Counts only allocations added by this route. Original plan/schema/pattern
// containers remain independently owned and are never counted as copied here.
fn pattern_metadata_bound(
    pattern: &Pattern,
    body: &SchemaBody,
    schemas: &SchemaTable,
    depth: usize,
) -> Option<(MetadataBound, usize)> {
    if depth >= MAX_DEPTH {
        return None;
    }
    match pattern {
        Pattern::Wildcard => Some((MetadataBound::default(), 0)),
        Pattern::Bind {
            schema: binding, ..
        } => {
            let scalar = Scalar::from_body(body)?;
            let target = schemas.get(binding.schema)?;
            (target.dimension_parameters().is_empty()
                && target.body() == body
                && scalar.accepts_region(binding.region))
            .then_some((MetadataBound::default(), 1))
        }
        Pattern::Enum { ordinal, payload } => {
            let SchemaBody::Enum { variants, .. } = body else {
                return None;
            };
            match (
                payload.as_deref(),
                variants.get(*ordinal as usize)?.payload.as_ref(),
            ) {
                (None, None) => Some((MetadataBound::default(), 0)),
                (Some(pattern), Some(body)) => {
                    let (bound, bindings) =
                        pattern_metadata_bound(pattern, body, schemas, depth + 1)?;
                    Some((
                        bound.add(MetadataBound {
                            bytes: core::mem::size_of::<ScalarPatternTree>() as u64,
                            nodes: 1,
                        })?,
                        bindings,
                    ))
                }
                _ => None,
            }
        }
        Pattern::Tuple(patterns) => {
            let SchemaBody::Tuple(children) = body else {
                return None;
            };
            if patterns.len() != children.len() {
                return None;
            }
            let mut bound = MetadataBound::slice::<ScalarPatternTree>(patterns.len())?;
            let mut bindings = 0_usize;
            for (pattern, body) in patterns.iter().zip(children) {
                let (child, count) = pattern_metadata_bound(pattern, body, schemas, depth + 1)?;
                bound = bound.add(child)?;
                bindings = bindings.checked_add(count)?;
            }
            Some((bound, bindings))
        }
        Pattern::Equal(_) | Pattern::Array { .. } => None,
    }
}

#[derive(Debug)]
struct BorrowedScalarMetadata {
    entries: Box<[(super::super::ActivatedNodeIndex, BorrowedScalarMatchPlan)]>,
    bound: MetadataBound,
    // One ticket follows this actual shared owner through preflight plans,
    // cloned ActivatedPlans, and execution-local source handles.
    _reservation: Option<mech_core::ManagedMemoryReservation>,
}

fn admit_metadata_bound(
    bound: MetadataBound,
    base: mech_core::ResourceDemand,
    limits: mech_core::MemoryBudgetLimits,
    node: mech_core::NodeId,
) -> Result<(), super::super::ResidentActivationError> {
    let overflow = || super::super::ResidentActivationError::RegionSizeOverflow;
    let construction = bound.construction().ok_or_else(overflow)?;
    let demand = mech_core::ResourceDemand {
        turn_peak_bytes: base
            .turn_peak_bytes
            .checked_add(construction.bytes)
            .ok_or_else(overflow)?,
        retained_nodes: base
            .retained_nodes
            .checked_add(construction.nodes)
            .ok_or_else(overflow)?,
        ..base
    };
    if let Some(violation) = mech_core::evaluate_aggregate_memory_budget(
        mech_core::MemoryObjectOwner::NodeScratch { node, ordinal: 0 },
        demand,
        0,
        limits,
    )
    .first()
    {
        return Err(
            super::super::ResidentActivationError::ResidentMemoryPlanRejected {
                error: mech_core::MemoryPlanError::TargetLimitExceeded {
                    violation: violation.clone(),
                },
            },
        );
    }
    Ok(())
}

/// Optional plan-level owner keeps the existing Match/step/arm layouts intact.
/// Plans without an eligible structural pattern allocate no new metadata.
#[derive(Clone, Debug, Default)]
pub(in crate::resident::general) struct BorrowedScalarMatchSources {
    metadata: Option<Arc<BorrowedScalarMetadata>>,
}

impl BorrowedScalarMatchSources {
    pub(in crate::resident::general) fn qualify<'a>(
        matches: impl Clone
        + Iterator<
            Item = (
                super::super::ActivatedNodeIndex,
                mech_core::NodeId,
                SchemaId,
                &'a [ActivatedMatchArm],
            ),
        >,
        schemas: &SchemaTable,
        account: Option<&mech_core::ManagedMemoryBudget>,
        base: mech_core::ResourceDemand,
        limits: mech_core::MemoryBudgetLimits,
    ) -> Result<Self, super::super::ResidentActivationError> {
        let overflow = || super::super::ResidentActivationError::RegionSizeOverflow;
        let mut retained = MetadataBound::owner::<BorrowedScalarMetadata>().ok_or_else(overflow)?;
        let mut count = 0_usize;
        let mut first_node = None;
        let mut previous = None;
        // This pass borrows only existing activated patterns. No Arc/tree/Vec
        // is constructed until every eligible owner has a checked total bound.
        for (node, artifact_node, schema, arms) in matches.clone() {
            if previous.is_some_and(|previous| previous >= node.get()) {
                return Err(overflow());
            }
            previous = Some(node.get());
            if let Some(bound) = BorrowedScalarMatchPlan::metadata_bound(schema, schemas, arms) {
                retained = retained.add(bound).ok_or_else(overflow)?;
                count = count.checked_add(1).ok_or_else(overflow)?;
                first_node.get_or_insert(artifact_node);
            }
        }
        if count == 0 {
            return Ok(Self::default());
        }
        retained =
            retained
                .add(
                    MetadataBound::slice::<(
                        super::super::ActivatedNodeIndex,
                        BorrowedScalarMatchPlan,
                    )>(count)
                    .ok_or_else(overflow)?,
                )
                .ok_or_else(overflow)?;
        admit_metadata_bound(retained, base, limits, first_node.unwrap())?;
        let construction = retained.construction().ok_or_else(overflow)?;
        let mut reservation = account
            .map(|account| account.reserve_capacity(construction.bytes))
            .transpose()
            .map_err(|error| super::super::ResidentActivationError::MemoryRuntime { error })?;
        let mut entries = Vec::with_capacity(count);
        for (node, _, schema, arms) in matches {
            if BorrowedScalarMatchPlan::metadata_bound(schema, schemas, arms).is_some() {
                entries.push((
                    node,
                    BorrowedScalarMatchPlan::materialize(schema, schemas, arms)
                        .ok_or_else(overflow)?,
                ));
            }
        }
        let entries = entries.into_boxed_slice();
        // All exact-capacity construction vectors have been consumed; only
        // the bounded final owner and its tree/region slices remain.
        if let Some(reservation) = &mut reservation {
            reservation
                .resize_capacity(retained.bytes)
                .map_err(|error| super::super::ResidentActivationError::MemoryRuntime { error })?;
        }
        let metadata = Arc::new(BorrowedScalarMetadata {
            entries,
            bound: retained,
            _reservation: reservation,
        });
        debug_assert_eq!(metadata.bound, retained);
        Ok(Self {
            metadata: Some(metadata),
        })
    }

    pub(super) fn get(
        &self,
        node: super::super::ActivatedNodeIndex,
    ) -> Result<Option<&BorrowedScalarMatchPlan>, ResidentKernelError> {
        let Some(metadata) = &self.metadata else {
            return Ok(None);
        };
        let entries = &metadata.entries;
        // The activation-owned lookup is metadata work, bounded and admitted
        // before binary search. It does not compare any language payload.
        let work = u64::from(entries.len().max(1).ilog2())
            .checked_add(2)
            .and_then(|n| n.checked_mul(HEADER_WORK))
            .ok_or(ResidentKernelError::InvalidShape)?;
        let cost = budget::resident_cost! { compute_work: work, ..KernelCostEstimate::default() };
        PreparedKernel::new((), cost).admit_control()?.into_plan();
        let Ok(ordinal) = entries.binary_search_by_key(&node.get(), |(node, _)| node.get()) else {
            return Ok(None);
        };
        Ok(Some(&entries[ordinal].1))
    }

    #[cfg(test)]
    pub(super) fn is_empty(&self) -> bool {
        self.metadata.is_none()
    }
}

pub(super) struct BorrowedScalarRoot {
    value: Value,
    footprint: mech_core::snapshot::ValueFootprint,
}

impl core::ops::Deref for BorrowedScalarRoot {
    type Target = Value;
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl BorrowedScalarRoot {
    pub(super) fn live_footprint(&self) -> Result<(u64, u64), ResidentKernelError> {
        Ok((
            self.footprint
                .retained_bytes
                .checked_add(core::mem::size_of::<Self>() as u64)
                .ok_or(ResidentKernelError::InvalidShape)?,
            self.footprint
                .node_count
                .checked_add(1)
                .ok_or(ResidentKernelError::InvalidShape)?,
        ))
    }
}

#[derive(Debug)]
pub(super) struct BorrowedScalarMatchPlan {
    schema: SchemaId,
    key: SchemaKey,
    // Activation owns all pattern containers. Runtime clones only this Arc.
    patterns: Box<[Option<BorrowedPattern>]>,
}

impl BorrowedScalarMatchPlan {
    fn metadata_bound(
        schema: SchemaId,
        schemas: &SchemaTable,
        arms: &[ActivatedMatchArm],
    ) -> Option<MetadataBound> {
        let definition = schemas.entry(schema)?;
        if !definition.schema().dimension_parameters().is_empty()
            || !supported_tree(definition.schema().body(), 0)
        {
            return None;
        }
        let mut retained = MetadataBound::slice::<Option<BorrowedPattern>>(arms.len())?;
        let mut structural = false;
        for arm in arms {
            match &arm.pattern {
                ActivatedMatchPattern::Structural { pattern, .. } => {
                    structural = true;
                    let (nested, bindings) =
                        pattern_metadata_bound(pattern, definition.schema().body(), schemas, 0)?;
                    retained = retained
                        .add(nested)?
                        .add(MetadataBound::slice::<ResidentRegion>(bindings)?)?;
                }
                ActivatedMatchPattern::Wildcard => {}
                _ => return None,
            }
        }
        structural.then_some(retained)
    }

    fn materialize(
        schema: SchemaId,
        schemas: &SchemaTable,
        arms: &[ActivatedMatchArm],
    ) -> Option<Self> {
        let definition = schemas.entry(schema)?;
        let mut patterns = Vec::with_capacity(arms.len());
        for arm in arms {
            patterns.push(match &arm.pattern {
                ActivatedMatchPattern::Structural { pattern, .. } => Some(
                    BorrowedPattern::qualify(pattern, definition.schema().body(), schemas, 0)?,
                ),
                ActivatedMatchPattern::Wildcard => None,
                _ => return None,
            });
        }
        Some(Self {
            schema,
            key: definition.key(),
            patterns: patterns.into_boxed_slice(),
        })
    }

    pub(super) fn pattern(&self, ordinal: usize) -> Option<&BorrowedPattern> {
        self.patterns.get(ordinal)?.as_ref()
    }

    /// Identity mismatch receives no borrowed permit and uses the existing
    /// conservative matcher. A retained Value clone keeps its real root/ticket.
    pub(super) fn retain_root(
        &self,
        value: ResidentValueRef<'_>,
        schemas: &SchemaTable,
    ) -> Result<Option<BorrowedScalarRoot>, ResidentKernelError> {
        let ResidentValueRef::Snapshot([Some(value)]) = value else {
            return Ok(None);
        };
        let mut meter = ResidentBudgetMeter::default();
        meter.charge_compute_work(8 * HEADER_WORK)?;
        let Some(owner) = value.schemas() else {
            return Ok(None);
        };
        if value.schema() != self.schema
            || value.schema_key() != self.key
            || value.validate_against(&owner).is_err()
            || value.validate_against(schemas).is_err()
            || !value.shape().parameter_values().is_empty()
        {
            return Ok(None);
        }
        let footprint =
            budget::measure_borrowed_canonical_value_footprint(&mut meter, value, &owner)?;
        let handle = core::mem::size_of::<BorrowedScalarRoot>() as u64;
        let cost = budget::resident_cost! {
            compute_work: meter.estimate().compute_work().checked_add(handle).ok_or(ResidentKernelError::InvalidShape)?,
            temporary_bytes: footprint.retained_bytes.checked_add(handle).ok_or(ResidentKernelError::InvalidShape)?,
            retained_nodes: footprint.node_count.checked_add(1).ok_or(ResidentKernelError::InvalidShape)?,
            ..KernelCostEstimate::default()
        };
        PreparedKernel::new((), cost).admit_control()?.into_plan();
        Ok(Some(BorrowedScalarRoot {
            value: value.clone(),
            footprint,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mech_core::snapshot::{SnapshotValidationContext, ValueFootprint};
    use mech_core::{IntegerWidth, SchemaDraft, SchemaTableBuilder, ValueDraft};

    fn fixture() -> (Arc<SchemaTable>, Value, BorrowedScalarMatchPlan) {
        let scalar = SchemaBody::UnsignedInteger(IntegerWidth::W64);
        let mut builder = SchemaTableBuilder::new();
        let leaf = builder
            .insert(
                SchemaDraft {
                    body: scalar.clone(),
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let tuple = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Tuple(vec![scalar.clone(), scalar].into_boxed_slice()),
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let built = builder.finish().unwrap();
        let leaf = built.resolve(leaf).unwrap();
        let tuple = built.resolve(tuple).unwrap();
        let schemas = Arc::new(built.table);
        let pattern = Pattern::Tuple(
            (0..2)
                .map(|local| Pattern::Bind {
                    local,
                    schema: ActivatedPatternBinding {
                        region: ResidentRegion {
                            kind: ResidentValueKind::Snapshot,
                            offset: local as usize,
                            len: 1,
                            shape: ResidentShape::SCALAR,
                        },
                        schema: leaf,
                    },
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        );
        let pattern =
            BorrowedPattern::qualify(&pattern, schemas.get(tuple).unwrap().body(), &schemas, 0)
                .unwrap();
        let source = ValueDraft {
            schema: tuple,
            shape_values: Box::new([]),
            data: ValueDataDraft::Tuple(
                vec![ValueDataDraft::U64(17), ValueDataDraft::U64(29)].into_boxed_slice(),
            ),
        }
        .finalize(&SnapshotValidationContext::with_shared_schemas(&schemas))
        .unwrap();
        let plan = BorrowedScalarMatchPlan {
            schema: tuple,
            key: source.schema_key(),
            patterns: vec![Some(pattern)].into_boxed_slice(),
        };
        (schemas, source, plan)
    }

    #[test]
    fn borrowed_scalar_root_keeps_the_actual_managed_owner_after_source_reuse() {
        let (schemas, source, plan) = fixture();
        let account = mech_core::ManagedMemoryBudget::new(1024 * 1024);
        let required = source.memory_budget_admission_bytes(&account).unwrap();
        let mut reservation = account.reserve_capacity(required).unwrap();
        let managed = source.into_memory_budget(&mut reservation).unwrap();
        drop(reservation);
        let before = account.used_bytes();
        assert!(before > 0);
        let pointer = managed.data() as *const ValueData;
        let mut cell = [Some(managed)];
        let retained = plan
            .retain_root(ResidentValueRef::Snapshot(&cell), &schemas)
            .unwrap()
            .unwrap();
        assert_eq!(
            retained.data() as *const ValueData,
            pointer,
            "borrowed matching shares the actual immutable root"
        );
        cell[0] = None;
        assert_eq!(
            account.used_bytes(),
            before,
            "reusing the source slot cannot release the borrowed root's ticket"
        );
        let mut actual = Vec::new();
        plan.pattern(0)
            .unwrap()
            .bind_values(retained.data(), &mut |_, draft| {
                actual.push(draft);
                Ok::<_, ()>(())
            })
            .unwrap();
        assert_eq!(actual, [ValueDataDraft::U64(17), ValueDataDraft::U64(29)]);
        drop(retained);
        assert_eq!(
            account.used_bytes(),
            0,
            "the last borrowed root releases its owned import claim"
        );
    }

    #[test]
    fn borrowed_scalar_root_exact_compute_denial_has_no_retained_candidate_and_recovers() {
        let (schemas, source, proof) = fixture();
        let mut meter =
            ResidentBudgetMeter::for_closed_analysis(mech_core::RESIDENT_MAX_COMPUTE_WORK, 0);
        budget::measure_borrowed_canonical_value_footprint(&mut meter, &source, &schemas).unwrap();
        let required = meter.estimate().compute_work()
            + 8 * HEADER_WORK
            + core::mem::size_of::<BorrowedScalarRoot>() as u64;
        let mut denied = budget::detached_turn_plan(mech_core::ResourceDemand::default()).unwrap();
        denied.budget_limits.max_compute_work = Some(required - 1);
        denied.budget_limits.max_comparison_work = Some(0);
        let source = [Some(source)];
        assert!(matches!(
            budget::with_resident_turn_plan(denied.clone(), || {
                proof.retain_root(ResidentValueRef::Snapshot(&source), &schemas)
            }),
            Err(ResidentKernelError::InvalidShape)
        ));
        denied.budget_limits.max_compute_work = Some(required);
        let retained = budget::with_resident_turn_plan(denied, || {
            proof.retain_root(ResidentValueRef::Snapshot(&source), &schemas)
        })
        .unwrap()
        .unwrap();
        assert!(std::ptr::eq(
            retained.data(),
            source[0].as_ref().unwrap().data()
        ));
    }

    #[test]
    fn borrowed_scalar_bindings_require_both_finite_dimensions_before_any_write() {
        let (_, source, proof) = fixture();
        let pattern = proof.pattern(0).unwrap();
        let cost = pattern
            .cost(ValueFootprint::zero(), KernelCostEstimate::default())
            .unwrap();
        assert!(cost.compute_work() > 0 && cost.comparison_work() > 0);
        for comparison in [false, true] {
            let mut plan =
                budget::detached_turn_plan(mech_core::ResourceDemand::default()).unwrap();
            plan.budget_limits.max_compute_work =
                Some(cost.compute_work() - u64::from(!comparison));
            plan.budget_limits.max_comparison_work =
                Some(cost.comparison_work() - u64::from(comparison));
            let mut writes = Vec::new();
            let apply = |writes: &mut Vec<ValueDataDraft>| -> Result<(), ResidentKernelError> {
                PreparedKernel::new((), cost).admit_control()?.into_plan();
                assert!(pattern.matches(source.data()));
                pattern.bind_values(source.data(), &mut |_, data| {
                    writes.push(data);
                    Ok(())
                })
            };
            assert_eq!(
                budget::with_resident_turn_plan(plan.clone(), || apply(&mut writes)),
                Err(ResidentKernelError::InvalidShape)
            );
            assert!(writes.is_empty());
            plan.budget_limits.max_compute_work = Some(cost.compute_work());
            plan.budget_limits.max_comparison_work = Some(cost.comparison_work());
            budget::with_resident_turn_plan(plan, || apply(&mut writes)).unwrap();
            assert_eq!(writes, [ValueDataDraft::U64(17), ValueDataDraft::U64(29)]);
        }
    }

    #[test]
    fn borrowed_scalar_proof_rejects_composite_bindings_and_keeps_identity_mismatch_conservative() {
        let (schemas, source, proof) = fixture();
        let composite = Pattern::Bind {
            local: 0,
            schema: ActivatedPatternBinding {
                schema: source.schema(),
                region: ResidentRegion {
                    kind: ResidentValueKind::Snapshot,
                    offset: 0,
                    len: 1,
                    shape: ResidentShape::SCALAR,
                },
            },
        };
        assert!(
            BorrowedPattern::qualify(
                &composite,
                schemas.get(source.schema()).unwrap().body(),
                &schemas,
                0
            )
            .is_none()
        );
        for unsupported in [
            SchemaBody::String,
            SchemaBody::Dynamic,
            SchemaBody::Option(Box::new(SchemaBody::Bool)),
        ] {
            assert!(!supported_tree(&unsupported, 0));
        }
        let mut wrong = proof;
        wrong.key = mech_core::SchemaKey::from_bytes([0x55; 32]);
        assert!(
            wrong
                .retain_root(ResidentValueRef::Snapshot(&[Some(source)]), &schemas)
                .unwrap()
                .is_none()
        );
    }
    fn metadata_fixture() -> (
        Arc<SchemaTable>,
        SchemaId,
        Vec<ActivatedMatchArm>,
        MetadataBound,
    ) {
        let (schemas, source, _) = fixture();
        let leaf = schemas
            .entries()
            .enumerate()
            .find_map(|(index, entry)| {
                matches!(
                    entry.schema().body(),
                    SchemaBody::UnsignedInteger(IntegerWidth::W64)
                )
                .then_some(SchemaId::new(index as u32))
            })
            .unwrap();
        let region = |offset| ResidentRegion {
            kind: ResidentValueKind::Snapshot,
            offset,
            len: 1,
            shape: ResidentShape::SCALAR,
        };
        let pattern = Pattern::Tuple(
            (0..2)
                .map(|local| Pattern::Bind {
                    local,
                    schema: ActivatedPatternBinding {
                        schema: leaf,
                        region: region(local as usize),
                    },
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        );
        let body = super::super::super::ActivatedControlBlock {
            steps: Arc::from([]),
            locals: Arc::from([]),
            yield_value: super::super::super::ResidentReadLocation::Scratch(region(0)),
            yield_layout: mech_core::ResidentPortLayout {
                kind: ResidentValueKind::Snapshot,
                shape: ResidentShape::SCALAR,
                schema_id: leaf,
                schema_key: schemas.entry(leaf).unwrap().key(),
                shape_instance: schemas
                    .get(leaf)
                    .unwrap()
                    .instantiate_shape(Box::new([]))
                    .unwrap(),
                activation_fixed_shape: true,
                resolved_selector: None,
            },
            yield_identity: None,
        };
        let arms = vec![ActivatedMatchArm {
            pattern: ActivatedMatchPattern::Structural {
                pattern,
                work: 3,
                binding_count: 2,
                equality_count: 0,
                snapshot_finalization_count: 2,
                clone_depth: 2,
            },
            binding_regions: vec![region(0), region(1)].into_boxed_slice(),
            guard_regions: Box::new([]),
            guard: None,
            body,
        }];
        // This fixture adds five actual heap owners: Arc metadata, lookup slice, arm slice,
        // two-element tuple-child slice, and two-element binding-region slice.
        // Compute its expected requested capacity from those concrete objects,
        // independently of the recursive source-pattern bound calculator.
        let align = core::mem::align_of::<BorrowedScalarMetadata>()
            .max(core::mem::align_of::<usize>()) as u64;
        let header = (2 * core::mem::size_of::<usize>() as u64 + align - 1) / align * align;
        let owner = (header + core::mem::size_of::<BorrowedScalarMetadata>() as u64 + align - 1)
            / align
            * align;
        let expected = MetadataBound {
            bytes: owner
                + core::mem::size_of::<(
                    super::super::super::ActivatedNodeIndex,
                    BorrowedScalarMatchPlan,
                )>() as u64
                + core::mem::size_of::<Option<BorrowedPattern>>() as u64
                + 2 * core::mem::size_of::<ScalarPatternTree>() as u64
                + 2 * core::mem::size_of::<ResidentRegion>() as u64,
            nodes: 5,
        };
        (schemas, source.schema(), arms, expected)
    }

    fn qualify_fixture(
        schemas: &SchemaTable,
        schema: SchemaId,
        arms: &[ActivatedMatchArm],
        account: &mech_core::ManagedMemoryBudget,
        limits: mech_core::MemoryBudgetLimits,
    ) -> Result<BorrowedScalarMatchSources, super::super::super::ResidentActivationError> {
        BorrowedScalarMatchSources::qualify(
            [(
                super::super::super::ActivatedNodeIndex(7),
                mech_core::NodeId::new(7),
                schema,
                arms,
            )]
            .into_iter(),
            schemas,
            Some(account),
            mech_core::ResourceDemand::default(),
            limits,
        )
    }

    #[test]
    fn borrowed_projection_metadata_finite_capacity_denial_retry_and_shared_clone_lifetime() {
        let (schemas, schema, arms, expected) = metadata_fixture();
        let account = mech_core::ManagedMemoryBudget::new(expected.bytes * 2);
        let held = account.reserve_capacity(1).unwrap();
        let limits = mech_core::TargetMemoryProfile::current_resident_cpu()
            .unwrap()
            .limits;
        assert!(matches!(
            qualify_fixture(&schemas, schema, &arms, &account, limits),
            Err(
                super::super::super::ResidentActivationError::MemoryRuntime {
                    error: mech_core::MemoryRuntimeError::BudgetExceeded { .. },
                }
            )
        ));
        assert_eq!(
            account.used_bytes(),
            1,
            "rejected construction leaves caller capacity unchanged"
        );
        drop(held);
        let sources = qualify_fixture(&schemas, schema, &arms, &account, limits).unwrap();
        assert_eq!(sources.metadata.as_ref().unwrap().bound, expected);
        assert_eq!(
            account.used_bytes(),
            expected.bytes,
            "construction capacity shrinks after temporary vectors are consumed"
        );
        let shared = sources.clone();
        assert!(Arc::ptr_eq(
            sources.metadata.as_ref().unwrap(),
            shared.metadata.as_ref().unwrap()
        ));
        drop(sources);
        assert_eq!(
            account.used_bytes(),
            expected.bytes,
            "a cloned plan retains the same one charged owner"
        );
        drop(shared);
        assert_eq!(account.used_bytes(), 0);
    }

    #[test]
    fn borrowed_projection_metadata_later_activation_failure_releases_the_owner_and_recovers() {
        let (schemas, schema, arms, expected) = metadata_fixture();
        let account = mech_core::ManagedMemoryBudget::new(expected.bytes * 2);
        let limits = mech_core::TargetMemoryProfile::current_resident_cpu()
            .unwrap()
            .limits;
        let failed_activation = || -> Result<_, super::super::super::ResidentActivationError> {
            let sources = qualify_fixture(&schemas, schema, &arms, &account, limits)?;
            // Real subsequent backing admission shares this same configured
            // account. Failure must unwind and release the completed metadata.
            let _later_backing =
                account
                    .reserve_capacity(account.limit_bytes())
                    .map_err(|error| {
                        super::super::super::ResidentActivationError::MemoryRuntime { error }
                    })?;
            Ok(sources)
        };
        assert!(matches!(
            failed_activation(),
            Err(
                super::super::super::ResidentActivationError::MemoryRuntime {
                    error: mech_core::MemoryRuntimeError::BudgetExceeded { .. },
                }
            )
        ));
        assert_eq!(account.used_bytes(), 0);
        let retry = qualify_fixture(&schemas, schema, &arms, &account, limits).unwrap();
        assert_eq!(account.used_bytes(), expected.bytes);
        drop(retry);
        assert_eq!(account.used_bytes(), 0);
    }

    #[test]
    fn borrowed_projection_metadata_aggregate_byte_and_node_limits_precede_reservation() {
        let (schemas, schema, arms, expected) = metadata_fixture();
        let account = mech_core::ManagedMemoryBudget::new(expected.bytes * 2);
        for node_limit in [false, true] {
            let mut limits = mech_core::TargetMemoryProfile::current_resident_cpu()
                .unwrap()
                .limits;
            limits.max_temporary_bytes = Some(expected.bytes * 2 - u64::from(!node_limit));
            limits.max_retained_nodes = Some(expected.nodes * 2 - u64::from(node_limit));
            assert!(matches!(
                qualify_fixture(&schemas, schema, &arms, &account, limits),
                Err(
                    super::super::super::ResidentActivationError::ResidentMemoryPlanRejected { .. }
                )
            ));
            assert_eq!(account.used_bytes(), 0);
            limits.max_temporary_bytes = Some(expected.bytes * 2);
            limits.max_retained_nodes = Some(expected.nodes * 2);
            let accepted = qualify_fixture(&schemas, schema, &arms, &account, limits).unwrap();
            assert_eq!(accepted.metadata.as_ref().unwrap().bound, expected);
            drop(accepted);
            assert_eq!(account.used_bytes(), 0);
        }
    }

    #[test]
    fn borrowed_projection_metadata_empty_and_ineligible_sources_allocate_nothing() {
        let (schemas, schema, mut arms, _) = metadata_fixture();
        let account = mech_core::ManagedMemoryBudget::new(0);
        let limits = mech_core::TargetMemoryProfile::current_resident_cpu()
            .unwrap()
            .limits;
        let empty = BorrowedScalarMatchSources::qualify(
            [].into_iter(),
            &schemas,
            Some(&account),
            mech_core::ResourceDemand::default(),
            limits,
        )
        .unwrap();
        assert!(empty.is_empty());
        arms[0].pattern = ActivatedMatchPattern::Bind;
        let unsupported = qualify_fixture(&schemas, schema, &arms, &account, limits).unwrap();
        assert!(unsupported.is_empty());
        assert_eq!(account.used_bytes(), 0);
    }
}
