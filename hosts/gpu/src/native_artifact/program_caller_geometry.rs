//! Current canonical caller holdings. Future retained-map owners must add their
//! private table/owner-lane nodes; a view never grants execution authority.
use super::{Context, Frame, NativeProgramExecutionError as Error, invalid};
use crate::native_artifact::{NativeExecutionPhase, owned_string_map::PublicationGeometry};

fn add(a: u64, b: u64) -> Result<u64, Error> {
    a.checked_add(b).ok_or_else(|| invalid("native caller node geometry overflows"))
}

pub(super) fn fixed_nodes() -> Result<u64, Error> {
    (core::mem::size_of::<Context<'_, '_>>() as u64)
        .checked_add(core::mem::size_of::<Frame>() as u64)
        .and_then(|n| n.checked_add(128))
        .ok_or_else(|| invalid("native caller fixed geometry overflows"))
}

impl Context<'_, '_> {
    pub(super) fn retained_caller_dynamic_nodes(&self, site: usize) -> Result<u64, Error> {
        add(self.library.memory_budget().retained_snapshot_nodes(),
            self.call_caches.caller_retained_nodes(&self.library)
                .map_err(|error| Error::Library {
                    origin: self.control.descriptor.plan.blocks()[site].origin(), error,
                })?)
    }

    pub(super) fn retained_caller_geometry(
        &mut self,
        site: usize,
    ) -> Result<Option<PublicationGeometry>, Error> {
        let current = u32::try_from(site).map_err(|_| invalid("caller geometry site overflows"))?;
        if self.control.previous != Some(current) || self.control.completed || self.consumed {
            return Err(invalid("caller geometry outside fresh current visit"));
        }
        self.retained_caller_inventory(site)
    }

    /// Inventory only; no visit or storage permission. The closed map entry
    /// uses this after its initializer was consumed, under that owner's proof.
    pub(super) fn retained_caller_inventory(&mut self, site: usize)
        -> Result<Option<PublicationGeometry>, Error>
    {
        // A source/window loan needs its own live-owner witness. Decline this
        // view instead of treating borrowed producer storage as empty. Ordinary
        // canonical execution remains available to the future selecting caller.
        if self.line_window.is_some() || self.window_region.is_some()
            || self.window_hashes.is_some() || self.closed_tail.is_some()
            || self.prefix.as_ref().is_some_and(|prefix| !prefix.input.eof)
        {
            return Ok(None);
        }
        let memory = self.library.memory_budget();
        self.control.work.check_memory_budget(&memory)?;
        self.control.work.charge(NativeExecutionPhase::Setup, 512)?;
        // Fixed invocation/frame/transport roots. This bound excludes variable
        // payload backing, so a long String is never priced as one node/byte.
        let fixed = fixed_nodes()?;
        let mut nodes = add(memory.retained_snapshot_nodes(), fixed)?;
        if let Some(collections) = &self.collections {
            nodes = add(nodes, collections.caller_retained_nodes(&mut self.control.work)?)?;
        }
        nodes = add(nodes, self.line_matrix_coexistence(None)?.retained_nodes)?;
        if self.borrowed_strings.is_some() {
            // Two flat metadata backings; genuine canonical pins are already
            // counted by the shared-account snapshot witness.
            nodes = add(nodes, 2)?;
        }
        if let Some(selected) = &self.selected_join {
            nodes = add(nodes, selected.caller_retained_nodes()?)?;
        }
        nodes = add(nodes, self.call_caches.caller_retained_nodes(&self.library)
            .map_err(|error| Error::Library {
                origin: self.control.descriptor.plan.blocks()[site].origin(), error,
            })?)?;
        Ok(Some(PublicationGeometry { live_bytes: memory.used_bytes(), live_nodes: nodes }))
    }
}

#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "program_caller_geometry_tests.rs"]
mod tests;
