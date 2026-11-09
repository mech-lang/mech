//! Arithmetic-only consumers of successfully completed, checked key arrays.
//! This certificate does not grant a slice bound, cursor bound or post-ID bound.
use super::*;
use mech_engine::native_artifact::{NativeMatchValue, NativeScalarOp};

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub(in crate::native_artifact) struct Addition {
    pub region: usize,
    pub site: usize,
    pub operands: [u32; 2],
    pub key_read: usize,
    pub key_array: usize,
    pub producer_header: usize,
}
#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Serialize)]
pub(in crate::native_artifact) struct Layout {
    pub additions: Vec<Addition>,
}
impl Context<'_> {
    fn scalar_source(&self, value: u32) -> Option<u32> {
        let mut key = (value, None);
        for _ in 0..MAX_DEPTH {
            match self.source(key) {
                Some(source) => key = source,
                None => return key.1.is_none().then_some(key.0),
            }
        }
        None
    }
}
impl Layout {
    pub(in crate::native_artifact) fn arithmetic(
        &self,
        region: usize,
        site: usize,
        op: NativeScalarOp,
        operands: &[NativeMatchValue],
    ) -> bool {
        if op != NativeScalarOp::U64Add || operands.len() != 2 {
            return false;
        }
        self.additions
            .binary_search_by_key(&(region, site), |p| (p.region, p.site))
            .ok()
            .is_some_and(|i| self.additions[i].operands == [operands[0].get(), operands[1].get()])
    }
    pub(in crate::native_artifact) fn derive(metadata: &ProgramMetadata) -> Self {
        let producers = producer_candidates(metadata);
        if producers.is_empty() {
            return Self::default();
        }
        let mut cx = Context::new(metadata);
        let mut out = Self::default();
        let mut steps = 0usize;
        for (region, r) in metadata.closed_tails.regions.iter().enumerate() {
            let Some(proof) = &r.numeric else {
                continue;
            };
            for &(read, ref operation) in &proof.actions {
                let Operation::Read { array, output, .. } = operation else {
                    continue;
                };
                let root = &proof.arrays[*array];
                if root.width != 8
                    || root.writable
                    || root.appends
                    || metadata.plan.values()[*output as usize].scalar_kind()
                        != Some(NativeScalarKind::U64)
                {
                    continue;
                }
                let Some(Origin::Returned {
                    region: producer,
                    array: keys,
                }) = cx.entry(region, *array)
                else {
                    continue;
                };
                let original = &metadata.closed_tails.regions[producer];
                if !producers
                    .iter()
                    .any(|p| p.producer == original.tail.header() as usize && p.array == keys)
                    || !cx
                        .facts
                        .strictly_dominates(original.tail.exit(), r.tail.initialize())
                        .unwrap_or(false)
                {
                    continue;
                }
                for &site in &proof.sites {
                    steps += 1;
                    if steps > 262_144 || out.additions.len() == 4096 {
                        // Refuse the entire cache on analysis exhaustion. Partial
                        // findings never broaden authority or defer analysis to runtime.
                        return Self::default();
                    }
                    let Action::Scalar(Scalar::Primitive {
                        op: Op::U64Add,
                        operands,
                        ..
                    }) = metadata.plan.blocks()[site].action()
                    else {
                        continue;
                    };
                    let [a, b] = operands.as_ref() else {
                        continue;
                    };
                    let bounded = (cx.scalar_source(a.get()) == Some(*output)
                        && cx.u64((b.get(), None), 0) == Some(1))
                        || (cx.scalar_source(b.get()) == Some(*output)
                            && cx.u64((a.get(), None), 0) == Some(1));
                    if !bounded
                        || !cx
                            .facts
                            .strictly_dominates(read as u32, site as u32)
                            .unwrap_or(false)
                    {
                        continue;
                    }
                    out.additions.push(Addition {
                        region,
                        site,
                        operands: [a.get(), b.get()],
                        key_read: read,
                        key_array: *array,
                        producer_header: original.tail.header() as usize,
                    });
                }
            }
        }
        out.additions.sort_by_key(|p| (p.region, p.site));
        out.additions.dedup_by_key(|p| (p.region, p.site));
        out
    }
}
