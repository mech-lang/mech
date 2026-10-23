//! Bounded evidence of substitutions actually selected while emitting native
//! instructions. This is build metadata, never executable proof or authority.
use mech_engine::native_artifact::{NativeMatchValue, NativeScalarOp};

const MAX_REGIONS: usize = 256;
const MAX_SUBSTITUTIONS: usize = 4096;

#[derive(Default)]
pub(in crate::native_artifact) struct CodegenReport {
    pub source_edge_facts: EdgeFacts,
    substitutions: usize,
}
#[derive(serde::Serialize)]
pub(in crate::native_artifact) struct EdgeFacts {
    complete: bool,
    regions: Vec<Region>,
}
impl Default for EdgeFacts {
    fn default() -> Self { Self { complete: true, regions: Vec::new() } }
}
#[derive(serde::Serialize)]
struct Region {
    header: usize,
    guards: Vec<Guard>,
    arithmetic: Vec<Arithmetic>,
}
#[derive(serde::Serialize)]
struct Guard { site: usize, source: NativeMatchValue, offset: usize, ordinal: u32 }
#[derive(serde::Serialize)]
struct Arithmetic { site: usize, op: NativeScalarOp, operands: [NativeMatchValue; 2] }
#[derive(Clone, Copy)]
pub(in crate::native_artifact) struct RegionToken(usize);

impl CodegenReport {
    pub fn packed_ssa_region(&mut self, header: usize) -> Option<RegionToken> {
        if self.source_edge_facts.regions.len() == MAX_REGIONS {
            self.source_edge_facts.complete = false;
            return None;
        }
        let token = RegionToken(self.source_edge_facts.regions.len());
        self.source_edge_facts.regions.push(Region { header, guards: Vec::new(), arithmetic: Vec::new() });
        Some(token)
    }
    fn admit(&mut self, token: Option<RegionToken>) -> Option<&mut Region> {
        let token = token?;
        if self.substitutions == MAX_SUBSTITUTIONS {
            self.source_edge_facts.complete = false;
            return None;
        }
        self.substitutions += 1;
        self.source_edge_facts.regions.get_mut(token.0)
    }
    pub fn guard(&mut self, token: Option<RegionToken>, site: usize,
        source: NativeMatchValue, offset: usize, ordinal: u32)
    {
        if let Some(region) = self.admit(token) { region.guards.push(Guard { site, source, offset, ordinal }); }
    }
    pub fn arithmetic(&mut self, token: Option<RegionToken>, site: usize,
        op: NativeScalarOp, operands: [NativeMatchValue; 2])
    {
        if let Some(region) = self.admit(token) { region.arithmetic.push(Arithmetic { site, op, operands }); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn emission_receipts_are_bounded_and_report_incomplete_coverage() {
        let mut report = CodegenReport::default();
        for header in 0..MAX_REGIONS { assert!(report.packed_ssa_region(header).is_some()); }
        assert!(report.source_edge_facts.complete);
        assert!(report.packed_ssa_region(MAX_REGIONS).is_none());
        assert!(!report.source_edge_facts.complete);
        assert_eq!(report.source_edge_facts.regions.len(), MAX_REGIONS);
        // The admission lane is shared by both substitution kinds; overflow
        // cannot quietly become a receipt claiming complete site coverage.
        let mut report = CodegenReport::default();
        let region = report.packed_ssa_region(19);
        for _ in 0..MAX_SUBSTITUTIONS { assert!(report.admit(region).is_some()); }
        assert!(report.source_edge_facts.complete);
        assert!(report.admit(region).is_none());
        assert!(!report.source_edge_facts.complete);
        assert_eq!(report.substitutions, MAX_SUBSTITUTIONS);
    }
}
