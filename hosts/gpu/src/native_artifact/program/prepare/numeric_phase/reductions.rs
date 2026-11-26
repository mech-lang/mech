//! Exact synchronous scalar range-map/sum/extraction children. Summaries retain
//! their original source path and a cold edge before the original range prepare.
use super::super::numeric_maps::{Layout, Region};
use super::*;
use mech_core::{DimensionExpr as D, Schema, SchemaBody};
use mech_engine::native_artifact::{
    NativeCollectionPrimitive as Primitive, NativeCollectionSource as Source,
};

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct Reduction {
    pub region: Region,
    pub prefix: Vec<u32>,
    pub entry: u32,
    pub prepare: u32,
    pub after: u32,
    pub first: u32,
    pub last: u32,
    pub inclusive: bool,
    pub maximum: u64,
    pub constant_count: bool,
}
fn constant(plan: &Plan, value: u32) -> Option<u64> {
    plan.constants()
        .iter()
        .find(|c| c.value().get() == value)
        .and_then(|c| {
            if let mech_core::ValueData::U64(n) = c.constant().data() {
                Some(*n)
            } else {
                None
            }
        })
}
/// Prove every positive cardinality up to maximum, without sampling shapes.
/// Plain independent parameters have constant bounds; relations and repeated
/// axis parameters retain the original factories at the original source.
fn row_shapes(schema: &Schema, maximum: u64, constant_count: bool) -> bool {
    if mech_core::shape_for_resolved_extents(schema, &[1, maximum]).is_err() {
        return false;
    }
    if constant_count {
        return true;
    }
    let SchemaBody::Matrix { dimensions, .. } = schema.body() else {
        return false;
    };
    if dimensions.len() != 2 {
        return false;
    }
    let mut used = BTreeSet::new();
    for (axis, d) in dimensions.iter().enumerate() {
        let upper = if axis == 0 { 1 } else { maximum };
        match d {
            D::Constant(1) if upper == 1 => {}
            D::Parameter(id) => {
                if !used.insert(id.get()) {
                    return false;
                }
                let Some(p) = schema.dimension_parameters().get(id.get() as usize) else {
                    return false;
                };
                if !matches!(p.lower_bound(),D::Constant(n) if *n<=1)
                    || !matches!(p.upper_bound(), None | Some(D::Constant(_)))
                    || matches!(p.upper_bound(),Some(D::Constant(n)) if *n<upper)
                {
                    return false;
                }
            }
            _ => return false,
        }
    }
    used.len() == schema.dimension_parameters().len()
}
fn candidate(plan: &Plan, region: Region) -> Option<Reduction> {
    let sum = region.reduction.as_ref()?;
    if region.rectangle.is_some() || !region.range {
        return None;
    }
    let NativeControlLoop::Collection(child) = &plan.iterations()[region.iteration as usize] else {
        return None;
    };
    let Source::U64Range(cursor) = child.source() else {
        return None;
    };
    let [first, last] = cursor.operands();
    let inclusive = match cursor.template().primitive() {
        Primitive::RangeExclusive => false,
        Primitive::RangeInclusive => true,
        _ => return None,
    };
    let count = constant(plan, first.get())
        .zip(constant(plan, last.get()))
        .and_then(|(a, b)| b.checked_sub(a)?.checked_add(u64::from(inclusive)));
    let maximum = count.filter(|&n| n != 0).or_else(|| {
        region
            .matrices
            .iter()
            .flat_map(
                |m| match plan.values()[m.value as usize].definition().body() {
                    SchemaBody::Matrix { dimensions, .. } => dimensions
                        .iter()
                        .filter_map(|d| {
                            if let D::Constant(n) = d {
                                Some(*n)
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>(),
                    _ => Vec::new(),
                },
            )
            .filter(|&n| n != 0)
            .max()
    })?;
    if maximum > isize::MAX as u64 / 8 {
        return None;
    }
    let constant_count = count == Some(maximum);
    let definition = &plan.collections()[region.collection as usize];
    let source_schema = cursor.template().output();
    if !row_shapes(source_schema, maximum, constant_count)
        || !row_shapes(
            plan.values()[region.output as usize].definition(),
            maximum,
            constant_count,
        )
    {
        return None;
    }
    let Action::LibraryCall {
        template: sum_call, ..
    } = plan.blocks()[sum.sum as usize].action()
    else {
        return None;
    };
    let Action::LibraryCall {
        template: select, ..
    } = plan.blocks()[sum.select as usize].action()
    else {
        return None;
    };
    if !row_shapes(&sum_call.inputs()[0], maximum, constant_count)
        || mech_core::shape_for_resolved_extents(sum_call.output(), &[1, 1]).is_err()
        || mech_core::shape_for_resolved_extents(&select.inputs()[0], &[1, 1]).is_err()
    {
        return None;
    }
    let Term::Scalar(Branch::Jump(after)) = plan.blocks()[sum.select as usize].terminator() else {
        return None;
    };
    let mut prefix = Vec::new();
    let entry = if cursor.before_collection_begin() {
        cursor.prepare()
    } else {
        definition.begin()
    };
    let mut at = entry;
    let mut seen = BTreeSet::new();
    while at != child.header() {
        if prefix.len() == 32 || !seen.insert(at) || plan.activation_boundary() == Some(at) {
            return None;
        }
        let b = &plan.blocks()[at as usize];
        if b.iteration() != child.parent() {
            return None;
        }
        match b.action() {
            Action::PrepareU64Range { iteration }
                if *iteration == region.iteration && at == cursor.prepare() => {}
            Action::CollectionBegin { collection }
                if *collection == region.collection && at == definition.begin() => {}
            Action::Initialize { iteration }
                if *iteration == region.iteration && at == child.initialize() => {}
            Action::Scalar(Scalar::None) => {}
            _ => return None,
        }
        let Term::Scalar(Branch::Jump(next)) = b.terminator() else {
            return None;
        };
        prefix.push(at);
        at = next;
    }
    if !prefix.contains(&definition.begin())
        || !prefix.contains(&child.initialize())
        || !prefix.contains(&cursor.prepare())
    {
        return None;
    }
    Some(Reduction {
        entry,
        prepare: cursor.prepare(),
        after,
        first: first.get(),
        last: last.get(),
        inclusive,
        maximum,
        constant_count,
        prefix,
        region,
    })
}
pub(super) fn inventory(plan: &Plan) -> Result<Vec<Reduction>, &'static str> {
    Ok(Layout::derive(plan)?
        .regions
        .into_iter()
        .filter_map(|r| candidate(plan, r))
        .collect())
}
impl Reduction {
    pub fn source_sites(&self) -> impl Iterator<Item = u32> + '_ {
        self.prefix
            .iter()
            .copied()
            .chain(self.region.sites.iter().copied())
            .chain(
                self.region
                    .reduction
                    .iter()
                    .flat_map(|r| r.finish_path.iter().chain(&r.continuation).copied()),
            )
    }
}

/// Source reconstruction only. The enclosing phase separately proves current
/// roots and lends their views; this record grants no storage authority.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct Fold {
    pub source: Reduction,
    pub arrays: Vec<usize>,
    pub budget: FoldBudget,
}
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct FoldBudget {
    pub schema_bytes: u64,
    pub compute: u64,
    /// Eligibility credit only: never charged or granted to the native body.
    pub replay: u64,
    pub temporary: u64,
    pub nodes: u64,
    pub output: u64,
    pub elements: u64,
    pub cloned: u64,
    pub comparison: u64,
    pub setup: u64,
    pub native_step: u64,
}
impl Reduction {
    pub fn scalar_values(&self) -> impl Iterator<Item = u32> + '_ {
        [self.first, self.last]
            .into_iter()
            .chain(self.region.captures.iter().copied())
            .chain(self.region.reduction.iter().map(|r| r.scalar))
    }
    pub fn budget(
        &self,
        plan: &Plan,
        calls: &[Option<NativePreparedLibraryCall>],
        visits: &[crate::native_artifact::iteration_abi::NativeIterationVisit],
    ) -> Option<FoldBudget> {
        let range = calls.get(self.prepare as usize)?.as_ref()?;
        if range.template()
            != plan
                .prepared_library_template(self.prepare as usize)
                .ok()??
        {
            return None;
        }
        let schema_bytes = range.schema_context_allocation_bound_bytes()?;
        // Check the selected range's exact scalar signature and ordinary shape
        // resolver. All positive dynamic cardinalities have the row-shape proof.
        let end = self.maximum.checked_sub(u64::from(self.inclusive))?;
        if range.selected_u64_range_cardinality(0, end).ok()? != self.maximum {
            return None;
        }
        let reduction = super::super::numeric_reduction_budget::bound(
            calls,
            schema_bytes,
            &self.region,
            self.maximum,
        )?;
        let mut source_work = 0u64;
        for site in self.source_sites() {
            // A canonical scalar path can also have an admitted group. Twice
            // the complete source inventory, including each 512-unit site
            // floor, bounds the group's visit/frontier price as well.
            source_work = source_work
                .checked_add(visits.get(site as usize)?.work)?
                .checked_add(super::super::tail::OVERHEAD)?;
            if let Some(template) = plan.prepared_library_template(site as usize).ok()? {
                if calls.get(site as usize)?.as_ref()?.template() != template {
                    return None;
                }
            }
        }
        source_work = source_work.checked_mul(2)?;
        let inventory = (plan.values().len() as u64)
            .checked_add(plan.blocks().len() as u64)?
            .checked_add(plan.iterations().len() as u64)?
            .checked_add(plan.collections().len() as u64)?;
        let mut read_work = 0u64;
        let mut read_replay = 0u64;
        for read in &self.region.reads {
            let call = calls.get(read.site as usize)?.as_ref()?;
            let work = call.scalar_matrix_element_native_work()?;
            read_work = read_work.checked_add(work)?;
            read_replay = read_replay
                .checked_add(work)?
                .checked_add(inventory.checked_mul(8192)?)?
                .checked_add(schema_bytes.checked_mul(128)?)?
                .checked_add(65536)?;
        }
        let native_step = (self.region.steps.len() as u64)
            .checked_mul(1024)?
            .checked_add((self.region.body.len() as u64).checked_mul(128)?)?
            .checked_add(5120)?;
        let bank_bytes = self.maximum.checked_mul(8)?;
        let temporary = reduction
            .temporary
            .checked_add(bank_bytes.checked_mul(16)?)?
            .checked_add(range.binding_metadata_bytes().checked_mul(128)?)?
            .checked_add(schema_bytes.checked_mul(32)?)?
            .checked_add(65536)?;
        let output = reduction.output.max(bank_bytes.checked_add(temporary)?);
        let setup = reduction
            .setup
            .checked_add(range.binding_work().checked_mul(128)?)?
            .checked_add(temporary.checked_mul(16)?)?;
        let traversal = source_work
            .checked_add(read_work)?
            .checked_add(native_step)?
            .checked_add(8192)?;
        let compute = self
            .maximum
            .checked_add(1)?
            .checked_mul(traversal)?
            .checked_add(setup)?
            .checked_add(reduction.commit)?;
        let probe = inventory
            .checked_mul(8192)?
            .checked_add(schema_bytes.checked_mul(128)?)?
            .checked_add(range.binding_work().checked_mul(1024)?)?
            .checked_add(temporary.checked_mul(128)?)?
            .checked_add(65536)?;
        let replay = self
            .maximum
            .checked_add(1)?
            .checked_mul(source_work.checked_add(read_replay)?.checked_add(8192)?)?
            .checked_add(probe)?
            .checked_add(reduction.replay)?;
        Some(FoldBudget {
            schema_bytes,
            compute,
            replay,
            temporary,
            nodes: reduction
                .nodes
                .checked_add(self.maximum.checked_mul(4)?)?
                .checked_add(256)?,
            output,
            elements: self.maximum.max(1),
            cloned: reduction.cloned.checked_add(bank_bytes.checked_mul(8)?)?,
            comparison: reduction.comparison,
            setup,
            native_step,
        })
    }
}
