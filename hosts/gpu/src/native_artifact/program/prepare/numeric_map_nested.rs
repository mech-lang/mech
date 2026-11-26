//! Exact scalar fold summaries inside an otherwise acyclic scalar map.
//! Child matrices share the enclosing admitted immutable view inventory.
use super::super::numeric_phase;
use super::*;

pub(super) const MAX_CHILDREN: usize = 8;
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct Reduction {
    pub source: numeric_phase::Reduction,
    pub matrices: Vec<usize>,
}
impl Reduction {
    pub(super) fn admit(
        plan: &Plan,
        facts: &Facts<'_>,
        initialize: u32,
        parent: Option<u32>,
        source: numeric_phase::Reduction,
        matrices: &mut Vec<Matrix>,
    ) -> Result<Option<Self>, &'static str> {
        let mut mapping = Vec::new();
        for matrix in &source.region.matrices {
            let Action::LibraryCall { operands, .. } =
                plan.blocks()[matrix.witness as usize].action()
            else {
                return Ok(None);
            };
            if !immutable_before(facts, operands[0], initialize, parent)? {
                return Ok(None);
            }
            let at = if let Some(at) = matrices.iter().position(|m| m.value == matrix.value) {
                at
            } else {
                if matrices.len() == MAX_MATRICES {
                    return Ok(None);
                }
                matrices.push(matrix.clone());
                matrices.len() - 1
            };
            if matrices[at].width != matrix.width {
                return Ok(None);
            }
            mapping.push(at);
        }
        Ok(Some(Self {
            source,
            matrices: mapping,
        }))
    }
    pub fn output(&self, plan: &Plan) -> Value {
        let Action::LibraryCall { output, .. } =
            plan.blocks()[self.source.region.reduction.as_ref().unwrap().select as usize].action()
        else {
            unreachable!("proved scalar extraction")
        };
        *output
    }
    pub fn elides(&self, value: u32, plan: &Plan) -> bool {
        let Loop::Collection(child) = &plan.iterations()[self.source.region.iteration as usize]
        else {
            return false;
        };
        self.source
            .region
            .reduction
            .as_ref()
            .is_some_and(|r| r.elided_values.contains(&value))
            || child.source().semantic_value().get() == value
    }
    pub fn reads(&self, plan: &Plan) -> Vec<Value> {
        let Loop::Collection(child) = &plan.iterations()[self.source.region.iteration as usize]
        else {
            unreachable!()
        };
        let Source::U64Range(cursor) = child.source() else {
            unreachable!()
        };
        let mut values = cursor.operands().to_vec();
        for &site in &self.source.region.body {
            values.extend(
                scalar_reads(plan, site)
                    .into_iter()
                    .filter(|v| self.source.region.captures.contains(&v.get())),
            );
        }
        values.sort_by_key(|v| v.get());
        values.dedup();
        values
    }
}
pub(in crate::native_artifact) fn reads(
    plan: &Plan,
    site: u32,
    summaries: &[Reduction],
) -> Vec<Value> {
    summaries
        .iter()
        .find(|r| r.source.entry == site)
        .map_or_else(|| scalar_reads(plan, site), |r| r.reads(plan))
}

/// Pure nonserialized funds, independently rebuilt with the selected calls
/// and fully composed source visit prices on every descriptor/image path.
#[derive(Clone, Copy, Debug, Default)]
pub(in crate::native_artifact) struct Bound {
    pub compute: u64,
    pub replay: u64,
    pub temporary: u64,
    pub nodes: u64,
    pub output: u64,
    pub elements: u64,
    pub cloned: u64,
    pub comparison: u64,
    pub native_step: u64,
}
#[derive(Debug, Default)]
pub(in crate::native_artifact) struct Funding {
    headers: Vec<Option<Bound>>,
}
impl Funding {
    pub fn derive(
        plan: &Plan,
        layout: &Layout,
        calls: &[Option<mech_engine::native_artifact::NativePreparedLibraryCall>],
        visits: &[crate::native_artifact::iteration_abi::NativeIterationVisit],
    ) -> Self {
        let mut result = Self::default();
        for region in layout.regions.iter().filter(|r| !r.nested.is_empty()) {
            let bound = (|| {
                let mut out = Bound::default();
                for child in &region.nested {
                    let b = child.source.budget(plan, calls, visits)?;
                    out.compute = out.compute.checked_add(b.compute)?;
                    out.replay = out.replay.checked_add(b.replay)?;
                    out.temporary = out.temporary.max(b.temporary);
                    out.nodes = out.nodes.max(b.nodes);
                    out.output = out.output.max(b.output);
                    out.elements = out.elements.max(b.elements);
                    out.cloned = out.cloned.checked_add(b.cloned)?;
                    out.comparison = out.comparison.checked_add(b.comparison)?;
                    out.native_step = out
                        .native_step
                        .checked_add(b.native_step.checked_mul(child.source.maximum)?)?;
                }
                let frames = (region.nested.len() as u64)
                    .checked_mul(std::mem::size_of::<
                        crate::native_artifact::program::runtime::NativeNumericMapFrame,
                    >() as u64)?;
                out.temporary = out.temporary.checked_add(frames)?;
                Some(out)
            })();
            if result.headers.is_empty() {
                result.headers.resize(plan.blocks().len(), None);
            }
            result.headers[region.header as usize] = bound;
        }
        result
    }
    pub fn at(&self, region: &Region) -> Option<Bound> {
        if region.nested.is_empty() {
            Some(Bound::default())
        } else {
            self.headers.get(region.header as usize).copied().flatten()
        }
    }
}
