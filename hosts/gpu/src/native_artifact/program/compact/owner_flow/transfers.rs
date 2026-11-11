//! Move dead aggregate owner lanes using the existing field/path proof.
use super::{OwnerFlow, contains};
use crate::native_artifact::program::compact::{flow::Flow, guards::Guards};

impl OwnerFlow {
    /// Transfer one held source reference after the shared aggregate succeeds.
    /// A duplicate source may feed several destinations, but only one can take
    /// its existing reference. Other aliases retain independently as before.
    pub fn transfers(&self, flow: &Flow, guards: &Guards) -> Vec<Vec<(usize, usize)>> {
        self.copies
            .iter()
            .enumerate()
            .map(|(site, copies)| {
                let mut moved = Vec::new();
                let mut sources = Vec::new();
                for &(destination, source) in copies {
                    if sources.contains(&source)
                        || self.definitions[site].contains(&source)
                        || self.post_uses[site].contains(&source)
                        || !contains(&self.live_in[site], source)
                    {
                        continue;
                    }
                    let source_slot = self.slots[source];
                    if self.live_after(site, source_slot)
                        && !self.last_use_with_guards(flow, guards, site, source_slot)
                    {
                        continue;
                    }
                    sources.push(source);
                    moved.push((self.slots[destination], source_slot));
                }
                moved
            })
            .collect()
    }
}
