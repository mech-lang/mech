//! One phase account, actual traversal debits, and a sufficient outer-header
//! envelope. This schedule is reconstructed from canonical source on image load.
use super::*;
use crate::native_artifact::iteration_abi::NativeIterationVisit;
use crate::native_artifact::program::prepare::tail::{FIXED_OVERHEAD, OVERHEAD};
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields, rename = "mech.native.image.Debit")]
pub(in crate::native_artifact) struct Debit {
    pub iteration: u32,
    pub header: usize,
    pub initialize: usize,
    pub latch: usize,
    pub finish: usize,
    pub exit: usize,
    pub parent: Option<u32>,
    pub counter: usize,
    pub first_visit: Option<usize>,
    pub compute: u64,
    pub comparison: u64,
    pub trips: Option<bounds::Trips>,
}
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields, rename = "mech.native.image.Schedule")]
pub(in crate::native_artifact) struct Schedule {
    pub loops: Vec<Debit>,
    pub cold: Vec<ColdChild>,
    pub boundary_work: u64,
    pub compute_bound: Bound,
    pub comparison_bound: Bound,
    pub checked_children: bool,
}
impl Schedule {
    pub fn derive(
        forest: &Forest,
        trips: Vec<bounds::Trips>,
        visits: &[NativeIterationVisit],
        actions: &[(usize, super::super::numeric_plan::Operation)],
        result_fields: usize,
        counter: usize,
        boundary_work: u64,
    ) -> Result<Self, &'static str> {
        let mut loops = Vec::new();
        for (index, loop_) in forest.loops.iter().enumerate() {
            let mut compute = if index == 0 {
                boundary_work
            } else {
                FIXED_OVERHEAD
            };
            let mut comparison = 0u64;
            for &site in &loop_.sites {
                compute = compute
                    .checked_add(visits[site].work)
                    .and_then(|n| n.checked_add(OVERHEAD))
                    .ok_or("phase traversal price overflows")?;
                if actions.iter().any(|(s, a)| {
                    *s == site && matches!(a, super::super::numeric_plan::Operation::Write { .. })
                }) {
                    comparison = comparison
                        .checked_add(1)
                        .ok_or("phase comparison price overflows")?;
                }
            }
            if index == 0 {
                compute = compute
                    .checked_add(
                        (result_fields as u64)
                            .checked_mul(64)
                            .ok_or("phase result price overflows")?,
                    )
                    .ok_or("phase result price overflows")?;
            }
            loops.push(Debit {
                iteration: loop_.iteration,
                header: loop_.header,
                initialize: loop_.initialize,
                latch: loop_.latch,
                finish: loop_.finish,
                exit: loop_.exit,
                parent: loop_.parent,
                counter: counter
                    .checked_add(index)
                    .ok_or("phase counter home overflows")?,
                first_visit: if forest.checked_children && index > 0 {
                    Some(
                        counter
                            .checked_add(forest.loops.len())
                            .and_then(|word| word.checked_add(index - 1))
                            .ok_or("child entry home overflows")?,
                    )
                } else {
                    None
                },
                compute,
                comparison,
                trips: trips
                    .iter()
                    .find(|p| p.iteration == loop_.iteration)
                    .cloned(),
            });
        }
        if forest.checked_children {
            return Ok(Self {
                compute_bound: Bound::Constant(loops[0].compute),
                comparison_bound: Bound::Constant(loops[0].comparison),
                loops,
                cold: forest.cold.clone(),
                boundary_work,
                checked_children: true,
            });
        }
        let mut envelopes = BTreeMap::<u32, (Bound, Bound)>::new();
        for debit in loops.iter().rev() {
            let mut compute = Bound::Constant(debit.compute);
            let mut comparison = Bound::Constant(debit.comparison);
            for child in loops.iter().filter(|l| l.parent == Some(debit.iteration)) {
                let count = &child
                    .trips
                    .as_ref()
                    .ok_or("phase child lacks its trip proof")?
                    .bound;
                let (work, compare) = &envelopes[&child.iteration];
                compute = Bound::add(compute, Bound::multiply(count.clone(), work.clone()));
                comparison =
                    Bound::add(comparison, Bound::multiply(count.clone(), compare.clone()));
            }
            envelopes.insert(debit.iteration, (compute, comparison));
        }
        let (compute_bound, comparison_bound) = envelopes.remove(&loops[0].iteration).unwrap();
        Ok(Self {
            loops,
            cold: forest.cold.clone(),
            boundary_work,
            compute_bound,
            comparison_bound,
            checked_children: false,
        })
    }

    pub fn words(&self) -> usize {
        self.loops.len()
            + self
                .loops
                .iter()
                .filter(|loop_| loop_.first_visit.is_some())
                .count()
    }
}
