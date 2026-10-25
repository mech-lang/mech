//! Chunk-local composition of already admitted joining and the emitted map.
//! Original source/copy slots remain absent; a missing key is reported later.
use super::*;
use crate::native_artifact::NativeValueHandle;
use mech_core::{ManagedNodeReservation, snapshot::OwnedSnapshotBytes};
use mech_engine::native_artifact::NativeStringMapRegion;

pub(super) struct Prepared {
    map: Value,
    handle: NativeValueHandle,
    table: Vec<u16>,
    nodes: ManagedNodeReservation,
}
pub(super) struct Fusing {
    pub view: NativeSelectedStringMatrix,
    map: Value,
    handle: NativeValueHandle,
    table: Vec<u16>,
    bytes: Vec<u8>,
    next: u64,
    payload: u64,
    missing: Option<u64>,
    loan: Option<(usize, usize, bool)>,
    envelope: ManagedMemoryReservation,
    finalization: u64,
    nodes: ManagedNodeReservation,
}
pub(in crate::native_artifact::program_runtime) struct Mapped {
    pub map: Value,
    pub handle: NativeValueHandle,
    pub owner: OwnedSnapshotBytes,
    pub count: u64,
    pub processed: u64,
    slot: u32,
    initialized: bool,
}
impl Fusing {
    pub(super) fn new(
        prepared: Prepared,
        view: NativeSelectedStringMatrix,
        target: String,
        envelope: ManagedMemoryReservation,
        finalization: u64,
    ) -> Result<Box<Self>, Error> {
        let next = view
            .consuming_join_first()
            .ok_or_else(|| invalid("joined map lacks contiguous geometry"))?;
        let payload = view.payload_bytes();
        Ok(Box::new(Self {
            view,
            map: prepared.map,
            handle: prepared.handle,
            table: prepared.table,
            bytes: target.into_bytes(),
            next,
            payload,
            missing: None,
            loan: None,
            envelope,
            finalization,
            nodes: prepared.nodes,
        }))
    }
}
impl Context<'_, '_> {
    pub(super) fn prepare_join_map(
        &mut self,
        site: usize,
        region: usize,
        view: &NativeSelectedStringMatrix,
        separator: &Value,
        target: &String,
        compute: u64,
    ) -> Result<Option<Prepared>, Error> {
        let descriptor = self.control.descriptor;
        let Some(proof) = descriptor
            .selected_joins
            .maps
            .get(region)
            .and_then(Option::as_ref)
        else {
            return Ok(None);
        };
        if view.payload_bytes() == 0
            || !view.can_consume_ascii_join_chunks()
            || !matches!(separator.data(), ValueData::String(s) if s.is_empty())
            || self
                .arena
                .current_retained_bytes(proof.map, &mut self.control.work)?
                .is_none()
        {
            return Ok(None);
        }
        let handle = self
            .arena
            .current_handle(proof.map, &mut self.control.work)?;
        let map = self.arena.copy_current(proof.map, &mut self.control.work)?;
        let mech_engine::native_artifact::NativeCollectionResult::TextJoin(sink) =
            descriptor.plan.collections()[proof.collection as usize].result()
        else {
            return Ok(None);
        };
        if self
            .arena
            .current_retained_bytes(sink.separator().get(), &mut self.control.work)?
            .is_none()
        {
            return Ok(None);
        }
        let final_separator = self
            .arena
            .copy_current(sink.separator().get(), &mut self.control.work)?;
        if !matches!(final_separator.data(), ValueData::String(s) if s.is_empty()) {
            return Ok(None);
        }
        let ValueData::Map(entries) = map.data() else {
            return Ok(None);
        };
        if entries.entries().len() > 128 {
            return Ok(None);
        }
        let chunks = view
            .consuming_join_chunk_bound()
            .ok_or_else(|| invalid("joined map lost physical chunks"))?;
        let bound = add(
            add(mul(descriptor.scalar_bytes, 16)?, 16384)?,
            add(
                add(
                    mul(view.payload_bytes(), 56)?,
                    mul(view.count() as u64, 128)?,
                )?,
                add(
                    mul(chunks, 2048)?,
                    mul(entries.entries().len() as u64, 256)?,
                )?,
            )?,
        )?;
        if bound > compute {
            return Ok(None);
        }
        let memory = self.library.memory_budget();
        let call = descriptor.calls[site]
            .as_ref()
            .ok_or_else(|| invalid("joined map lost its selected join"))?;
        if !call
            .selected_matrix_join_chunks_ready(view, separator, target, &memory)
            .map_err(|e| library(self, site, e))?
        {
            return Ok(None);
        }
        let mut table = Vec::new();
        if table.try_reserve_exact(128).is_err() || table.capacity() != 128 {
            return Ok(None);
        }
        table.resize(128, 256u16);
        for entry in entries.entries() {
            let (ValueData::String(key), ValueData::String(value)) =
                (entry.key().data(), entry.value())
            else {
                return Ok(None);
            };
            if key.len() == 1 && key.is_ascii() {
                if value.len() != 1 || !value.is_ascii() {
                    return Ok(None);
                }
                table[key.as_bytes()[0] as usize] = value.as_bytes()[0] as u16;
            }
        }
        // Optional temporary nodes replace the canonical output's two nodes.
        // A failed optional claim leaves the original join fully available.
        let Ok(nodes) = memory.reserve_nodes(2) else {
            return Ok(None);
        };
        Ok(Some(Prepared {
            map,
            handle,
            table,
            nodes,
        }))
    }

    pub(in crate::native_artifact::program_runtime) fn join_map_is_fusing(&self) -> bool {
        self.selected_join
            .as_ref()
            .is_some_and(|s| matches!(s.phase, Phase::Fusing(_)))
    }
    pub(super) fn join_map_boundary(&self, site: usize, state: &State) -> Result<(), Error> {
        let proof = self
            .control
            .descriptor
            .selected_joins
            .maps
            .get(state.region)
            .and_then(Option::as_ref)
            .ok_or_else(|| invalid("joined map lost its use proof"))?;
        let Phase::Mapped(mapped) = &state.phase else {
            return Err(invalid("unfinished joined map escaped its join"));
        };
        let expected = if mapped.initialized {
            Some(proof.iteration)
        } else {
            state.parent
        };
        if proof.sites.binary_search(&(site as u32)).is_err() || self.control.current != expected {
            return Err(invalid(
                "joined map escaped its proved copy/initializer path",
            ));
        }
        Ok(())
    }
    pub(in crate::native_artifact::program_runtime) fn move_joined_map(
        &mut self,
        source: u32,
        output: u32,
    ) -> Result<bool, Error> {
        let Some(state) = self.selected_join.as_mut() else {
            return Ok(false);
        };
        let Phase::Mapped(mapped) = &mut state.phase else {
            return Ok(false);
        };
        if mapped.initialized || mapped.slot != source {
            return Err(invalid("joined map copied another source generation"));
        }
        // Match the original Copy's replacement point for a previous lexical
        // invocation; the new private owner is never installed as that Value.
        self.arena.retire_current(output, &mut self.control.work)?;
        mapped.slot = output;
        Ok(true)
    }
    pub(in crate::native_artifact::program_runtime) fn initialize_joined_map(
        &mut self,
        iteration: u32,
        source: u32,
    ) -> Result<Option<u64>, Error> {
        let Some(state) = self.selected_join.as_mut() else {
            return Ok(None);
        };
        let Phase::Mapped(mapped) = &mut state.phase else {
            return Ok(None);
        };
        let proof = self.control.descriptor.selected_joins.maps[state.region]
            .as_ref()
            .ok_or_else(|| invalid("joined map initializer lost its use proof"))?;
        if mapped.initialized
            || proof.iteration != iteration
            || proof.source != source
            || mapped.slot != source
            || self.control.current != Some(iteration)
        {
            return Err(invalid("joined map initialized outside its exact consumer"));
        }
        mapped.initialized = true;
        Ok(Some(mapped.count))
    }
    pub(in crate::native_artifact::program_runtime) fn take_joined_map(
        &mut self,
        region: &NativeStringMapRegion,
    ) -> Result<Option<Mapped>, Error> {
        let Some(state) = self.selected_join.as_ref() else {
            return Ok(None);
        };
        if !matches!(state.phase, Phase::Mapped(_)) {
            return Err(invalid("String map reached an unfinished join"));
        }
        let proof = self.control.descriptor.selected_joins.maps[state.region]
            .as_ref()
            .ok_or_else(|| invalid("joined map header lost its proof"))?;
        if proof.collection != region.collection() || proof.iteration != region.iteration() {
            return Err(invalid("joined map reached another String-map header"));
        }
        let State {
            phase: Phase::Mapped(mapped),
            ..
        } = self.selected_join.take().unwrap()
        else {
            unreachable!()
        };
        if !mapped.initialized
            || mapped.slot != region.source().get()
            || self
                .arena
                .current_handle(region.map().get(), &mut self.control.work)?
                != mapped.handle
            || !self
                .arena
                .get(mapped.handle, &mut self.control.work)?
                .shares_frozen_storage(&mapped.map)
        {
            return Err(invalid("joined map source/map generation changed"));
        }
        Ok(Some(mapped))
    }

    pub(in crate::native_artifact::program_runtime) fn prepare_join_map_chunk(
        &mut self,
        site: usize,
        args: [u64; 3],
    ) -> Result<Option<u64>, Error> {
        if args != [0; 3] || !self.consumed {
            return Err(invalid("joined-map copy precedes its admitted join"));
        }
        let descriptor = self.control.descriptor;
        let words = {
            let state = self
                .selected_join
                .as_mut()
                .ok_or_else(|| invalid("joined-map copy has no owner"))?;
            if descriptor.selected_joins.regions[state.region].consumer() as usize != site
                || self.control.current != state.parent
            {
                return Err(invalid("joined-map copy uses another visit"));
            }
            let Phase::Fusing(fusing) = &mut state.phase else {
                return Err(invalid("joined-map copy after completion"));
            };
            if fusing.loan.is_some() {
                return Err(invalid("joined-map copy replayed with a live span"));
            }
            let start = fusing.bytes.len();
            let following = fusing
                .view
                .append_consuming_ascii_join_chunk(fusing.next, &mut fusing.bytes)
                .map_err(|e| library_at(descriptor, site, e))?;
            if let Some(next) = following {
                if next <= fusing.next {
                    return Err(invalid("joined-map copy made no progress"));
                }
                fusing.next = next;
                let count = fusing.bytes.len() - start;
                let active = fusing.missing.is_none();
                fusing.loan = Some((start, count, active));
                // All bytes are initialized before native execution. The exact
                // fixed-capacity output cannot grow while this span is loaned.
                let pointer = unsafe { fusing.bytes.as_mut_ptr().add(start) } as u64;
                Some([
                    pointer,
                    if active { count as u64 } else { 0 },
                    fusing.table.as_ptr() as u64,
                    pointer,
                    0,
                ])
            } else {
                None
            }
        };
        if let Some(words) = words {
            self.write_window_arguments(words)?;
            return Ok(Some(1));
        }
        self.finish_join_map_copy(site)?;
        Ok(Some(0))
    }
    pub(in crate::native_artifact::program_runtime) fn finish_join_map_chunk(
        &mut self,
        site: usize,
        args: [u64; 3],
    ) -> Result<Option<u64>, Error> {
        if args[1..] != [0; 2] || !self.consumed {
            return Err(invalid("joined-map completion arguments"));
        }
        let state = self
            .selected_join
            .as_mut()
            .ok_or_else(|| invalid("joined-map completion lost its owner"))?;
        if self.control.descriptor.selected_joins.regions[state.region].consumer() as usize != site
            || self.control.current != state.parent
        {
            return Err(invalid("joined-map completion uses another visit"));
        }
        let Phase::Fusing(fusing) = &mut state.phase else {
            return Err(invalid("joined-map completion after finalization"));
        };
        let (start, count, active) = fusing
            .loan
            .take()
            .ok_or_else(|| invalid("joined-map completion without a loan"))?;
        if args[0] > if active { count as u64 } else { 0 } {
            return Err(invalid("joined-map result exceeds its initialized span"));
        }
        if active && args[0] < count as u64 {
            fusing.missing = Some(add(start as u64, args[0])?);
        }
        Ok(None)
    }
    fn finish_join_map_copy(&mut self, site: usize) -> Result<(), Error> {
        let State {
            phase: Phase::Fusing(fusing),
            region,
            parent,
            reservation,
        } = self
            .selected_join
            .take()
            .ok_or_else(|| invalid("joined map finalization lost its owner"))?
        else {
            return Err(invalid("joined map finalization phase"));
        };
        let Fusing {
            view,
            map,
            handle,
            table,
            bytes,
            payload,
            missing,
            loan,
            mut envelope,
            finalization,
            nodes,
            ..
        } = *fusing;
        if loan.is_some() || bytes.len() as u64 != payload {
            return Err(invalid("joined map final length differs"));
        }
        drop((view, table, nodes, reservation));
        let text =
            String::from_utf8(bytes).map_err(|_| invalid("joined map wrote invalid UTF8"))?;
        let descriptor = self.control.descriptor;
        let output = &descriptor.plan.values()
            [descriptor.selected_joins.regions[region].output().get() as usize];
        let canonical = SnapshotCanonicalizationBudget::new(finalization);
        let value = ValueDraft {
            schema: output.schema(),
            shape_values: Box::new([]),
            data: ValueDataDraft::String(text),
        }
        .finalize(
            &SnapshotValidationContext::with_shared_schemas(&descriptor.schemas)
                .with_canonicalization_budget(&canonical),
        )
        .map_err(Error::Snapshot)?
        .into_memory_budget(&mut envelope)
        .map_err(|e| library(self, site, e.into()))?;
        let memory = self.library.memory_budget();
        let mech_core::snapshot::StringOwnershipTransfer::Owned(owner) =
            value.try_into_owned_string(&memory)?
        else {
            return Err(invalid("private joined-map output unexpectedly shared"));
        };
        self.arena.retire_current(
            descriptor.selected_joins.regions[region].output().get(),
            &mut self.control.work,
        )?;
        self.fused_string_map_bytes = self
            .fused_string_map_bytes
            .checked_add(payload)
            .ok_or_else(|| invalid("joined-map byte counter overflow"))?;
        self.selected_join = Some(State {
            region,
            parent,
            reservation: memory.reserve_capacity(0)?,
            phase: Phase::Mapped(Mapped {
                map,
                handle,
                owner: owner.into_bytes(),
                count: payload,
                processed: missing.unwrap_or(payload),
                slot: descriptor.selected_joins.regions[region].output().get(),
                initialized: false,
            }),
        });
        Ok(())
    }
}
fn library_at(
    descriptor: &crate::native_artifact::NativeProgramDescriptor,
    site: usize,
    error: InvokeError,
) -> Error {
    Error::Library {
        origin: descriptor.plan.blocks()[site].origin(),
        error,
    }
}
