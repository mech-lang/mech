//! Fixed private storage for a source-closed generator. No runtime addresses.
use super::super::readonly_numeric::ArrayStorage;
use super::*;

pub(in crate::native_artifact) const MAX_INPUTS: usize = 16;
pub(in crate::native_artifact) const MAX_LOOPS: usize = 16;
#[derive(Clone, Debug, serde::Serialize)]
pub(in crate::native_artifact) struct Bank {
    pub offset: u64,
    pub extents: [u64; 2],
}
#[derive(Clone, Debug, serde::Serialize)]
pub(in crate::native_artifact) struct Input {
    pub value: u32,
    pub extents: [u64; 2],
}
#[derive(Clone, Debug, serde::Serialize)]
pub(in crate::native_artifact) struct Layout {
    pub banks: Vec<Bank>,
    /// Materialized map and ordinary arithmetic outputs, never old versions.
    pub values: BTreeMap<u32, usize>,
    /// Each writable child root receives a fresh copy before its source body.
    pub children: Vec<(usize, usize, usize)>,
    pub inputs: Vec<Input>,
    pub loops: Vec<u32>,
    pub elements: u64,
}
impl Layout {
    pub fn derive(
        plan: &Plan,
        region: &Region,
        tails: &tail::Layout,
    ) -> Result<Self, &'static str> {
        region
            .count
            .checked_add(1)
            .ok_or("generator root traversal count overflow")?;
        region.extents[0]
            .checked_mul(region.extents[1])
            .and_then(|n| n.checked_mul(8))
            .filter(|&n| n <= isize::MAX as u64)
            .ok_or("generator output byte span is not representable")?;
        let mut out = Self {
            banks: Vec::new(),
            values: BTreeMap::new(),
            children: Vec::new(),
            inputs: Vec::new(),
            loops: vec![region.iteration],
            elements: 0,
        };
        for map in &region.maps {
            let bank = out.push(map.extents)?;
            out.values.insert(map.output, bank);
            out.values.insert(map.region.output, bank);
            out.loops.push(map.region.iteration);
        }
        for child in &region.children {
            let selected = &tails.regions[child.region];
            let proof = selected
                .numeric
                .as_ref()
                .ok_or("generator bank lost numeric child")?;
            for (array, view) in proof.arrays.iter().enumerate() {
                if !view.writable {
                    continue;
                }
                if view.appends
                    || !matches!(
                        view.storage,
                        ArrayStorage::Carrier { .. } | ArrayStorage::TaggedCarrier { .. }
                    )
                {
                    return Err("generator writable bank is not a fixed child carrier");
                }
                let schema = plan
                    .values()
                    .iter()
                    .find(|v| v.schema() == view.schema)
                    .ok_or("generator child bank schema is absent")?;
                let extents = matrix(schema.definition().body())
                    .ok_or("generator child bank is not fixed F64")?;
                let bank = out.push(extents)?;
                out.children.push((child.region, array, bank));
            }
            out.loops.push(child.trips.iteration);
        }
        let binary = numeric_binary::Layout::derive(plan)?;
        for &site in &region.order {
            let Action::LibraryCall {
                template, output, ..
            } = plan.blocks()[site as usize].action()
            else {
                continue;
            };
            if binary.at(site as usize).is_some()
                || template.primitive() == Primitive::MatrixTranspose
            {
                let extents = matrix(plan.values()[output.get() as usize].definition().body())
                    .ok_or("generator operation bank is not a fixed matrix")?;
                let bank = out.push(extents)?;
                out.values.insert(output.get(), bank);
            }
        }
        let local = region.local_values.iter().copied().collect::<BTreeSet<_>>();
        let facts = plan.facts();
        let mut reads = BTreeMap::new();
        for &site in &region.sites {
            facts.parallel_action_reads(site, &mut |value| {
                reads.insert(value.get(), value);
                Ok(())
            })?;
        }
        for (&value, _) in &reads {
            if local.contains(&value) {
                continue;
            }
            if let Some(extents) = matrix(plan.values()[value as usize].definition().body()) {
                out.inputs.push(Input { value, extents });
            }
        }
        out.loops[1..].sort_unstable();
        out.loops.dedup();
        if out.inputs.len() > MAX_INPUTS || out.loops.len() > MAX_LOOPS {
            return Err("generator fixed input/loop bank exceeds its representation");
        }
        out.elements
            .checked_mul(8)
            .filter(|&n| n <= isize::MAX as u64)
            .ok_or("generator private byte span is not representable")?;
        Ok(out)
    }
    fn push(&mut self, extents: [u64; 2]) -> Result<usize, &'static str> {
        let count = extents[0]
            .checked_mul(extents[1])
            .ok_or("generator bank cardinality overflow")?;
        let offset = self.elements;
        self.elements = self
            .elements
            .checked_add(count)
            .ok_or("generator bank offset overflow")?;
        let index = self.banks.len();
        self.banks.push(Bank { offset, extents });
        Ok(index)
    }
}

/// Issued synchronously by an ordinary collection loan. Only completion and
/// traversal counts are writable; every pointer and extent remains immutable.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(in crate::native_artifact) struct MatrixView {
    pub input: u64,
    pub count: u64,
    pub rows: u64,
    pub columns: u64,
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(in crate::native_artifact) struct Frame {
    pub output: u64,
    pub output_elements: u64,
    pub scratch: u64,
    pub scratch_elements: u64,
    pub epoch: u64,
    pub issued: u64,
    pub completed: u64,
    pub source_count: u64,
    pub inputs: [MatrixView; MAX_INPUTS],
    pub traversals: [u64; MAX_LOOPS],
}
