//! Chunk-local composition of already admitted joining and the emitted map.
//! Original source/copy slots remain absent; a missing key is reported later.
use super::*;
use crate::native_artifact::NativeValueHandle;
use crate::native_artifact::program_ascii_map as ascii_map;
use mech_core::ManagedNodeReservation;
use mech_engine::native_artifact::{
    NativePrivateStringBuffer, NativeStringMapRegion, NativeTextLineJoinChunk,
};
#[cfg(all(test, feature = "canonical-native-program-host"))]
#[path = "program_selected_join_map_faults.rs"]
pub(super) mod faults;

pub(super) struct Prepared {
    map: Value,
    handle: NativeValueHandle,
    table: Vec<u8>,
    nodes: ManagedNodeReservation,
}
struct CopyLoan {
    source: NativeTextLineJoinChunk,
    // Six immutable geometry words followed by the generated kernel's exact
    // initialized-extent receipt. Fusing lives in one stable Box throughout
    // this loan; neither metadata nor target can move/grow until completion.
    words: [u64; 7],
    start: usize,
    count: usize,
    capacity: usize,
    target: u64,
    active: bool,
}

pub(super) struct Fusing {
    pub view: NativeSelectedStringMatrix,
    map: Value,
    handle: NativeValueHandle,
    table: Vec<u8>,
    bytes: Vec<u8>,
    next: u64,
    payload: u64,
    missing: Option<u64>,
    loan: Option<CopyLoan>,
    envelope: ManagedMemoryReservation,
    nodes: ManagedNodeReservation,
}
// The original fixed join envelope already covers this metadata and table.
// A representation change cannot silently grow beyond that admitted bound.
const _: () = assert!(core::mem::size_of::<Fusing>() + 256 <= 4096);

pub(in crate::native_artifact::program_runtime) struct Mapped {
    pub map: Value,
    pub handle: NativeValueHandle,
    pub owner: NativePrivateStringBuffer,
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
        _finalization: u64,
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
        join_peak: u64,
        join_nodes: u64,
    ) -> Result<Option<Prepared>, Error> {
        #[cfg(all(test, feature = "canonical-native-program-host"))]
        if faults::disabled() {
            return Ok(None);
        }
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
        {
            return Ok(None);
        }
        // The original join's complete work debit precedes this leaf. Its
        // scalar/metadata envelope below covers both current-slot reads; do not
        // debit that same work again inside an exactly exhausted call scope.
        let Some((map, handle)) = self
            .arena
            .copy_current_prepaid(proof.map, &self.control.work)?
        else {
            return Ok(None);
        };
        let mech_engine::native_artifact::NativeCollectionResult::TextJoin(sink) =
            descriptor.plan.collections()[proof.collection as usize].result()
        else {
            return Ok(None);
        };
        let Some((final_separator, _)) = self
            .arena
            .copy_current_prepaid(sink.separator().get(), &self.control.work)?
        else {
            return Ok(None);
        };
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
        // Optional estimates use a wider integer so an unfundable optimization
        // declines instead of introducing a new source overflow/failure.
        let bound = u128::from(descriptor.scalar_bytes) * 16
            + 16384
            + u128::from(view.payload_bytes()) * 56
            + view.count() as u128 * 128
            + u128::from(chunks) * 2048
            + entries.entries().len() as u128 * 256;
        if bound > u128::from(compute) {
            return Ok(None);
        }
        let memory = self.library.memory_budget();
        let Some(map_bytes) = map.memory_budget_retained_bytes(&memory) else {
            return Ok(None);
        };
        // The newly traversed map is an input to this composed operation.
        // Include its FULL bound, even though its genuine global ownership is
        // already shared with the arena. Tight cases keep the ordinary join.
        let peak = u128::from(join_peak) + u128::from(map_bytes);
        let nodes = u128::from(join_nodes) + entries.entries().len() as u128 * 2 + 4;
        if self
            .limits
            .max_temporary_bytes
            .is_none_or(|limit| peak > u128::from(limit))
            || self
                .limits
                .max_retained_nodes
                .is_none_or(|limit| nodes > u128::from(limit))
        {
            return Ok(None);
        }
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
        if table.try_reserve_exact(ascii_map::BYTES).is_err()
            || table.capacity() != ascii_map::BYTES
        {
            return Ok(None);
        }
        // Materialize the byte representation once while this exact immutable
        // map is admitted. Every high-bit cell is missing; valid zero remains
        // distinct. The original fixed envelope still covers 256 table bytes.
        table.resize(
            ascii_map::BYTES,
            crate::native_artifact::program_abi::JOIN_MAP_MISSING_BYTE,
        );
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
                table[ascii_map::HEADER + key.as_bytes()[0] as usize] = value.as_bytes()[0];
            }
        }
        ascii_map::prepare(&mut table);
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
        #[cfg(all(test, feature = "canonical-native-program-host"))]
        faults::before_prepare(self, site)?;
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
            let source = fusing
                .view
                .take_consuming_ascii_join_chunk(fusing.next)
                .map_err(|e| library_at(descriptor, site, e))?;
            if let Some(source) = source {
                if source.following() <= fusing.next {
                    return Err(invalid("joined-map source loan made no progress"));
                }
                let start = fusing.bytes.len();
                let count = usize::try_from(source.payload_bytes())
                    .map_err(|_| invalid("joined-map source payload exceeds address space"))?;
                let end = start
                    .checked_add(count)
                    .ok_or_else(|| invalid("joined-map output extent overflow"))?;
                if end > fusing.bytes.capacity() {
                    return Err(invalid("joined-map output loan exceeds admitted capacity"));
                }
                let active = fusing.missing.is_none();
                let mut words = [0; 7];
                words[..6].copy_from_slice(source.geometry());
                // SAFETY: start is the initialized length of this Vec and end
                // was checked against its admitted capacity. This synchronous
                // loan retains the allocation without growth until completion.
                let output = unsafe { fusing.bytes.as_mut_ptr().add(start) } as u64;
                fusing.loan = Some(CopyLoan {
                    source,
                    words,
                    start,
                    count,
                    capacity: fusing.bytes.capacity(),
                    target: output,
                    active,
                });
                let loan = fusing.loan.as_mut().unwrap();
                // The target remains at its old initialized length. Generated
                // code owns only this checked spare-capacity extent; it must
                // initialize ALL of it, even after the first missing mapping.
                // The owning chunk keeps immutable bytes/offsets alive. The
                // Box, loan metadata and Vec allocation stay fixed until the
                // matching completion callback ends this synchronous loan.
                Some([
                    loan.words.as_mut_ptr() as u64,
                    count as u64,
                    fusing.table.as_ptr() as u64,
                    output,
                    u64::from(active),
                ])
            } else {
                None
            }
        };
        if let Some(words) = words {
            self.write_window_arguments(words)?;
            #[cfg(all(test, feature = "canonical-native-program-host"))]
            faults::after_prepare(self, site)?;
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
        #[cfg(all(test, feature = "canonical-native-program-host"))]
        if faults::before_finish(self, site)? {
            return Ok(None);
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
        let loan = fusing
            .loan
            .take()
            .ok_or_else(|| invalid("joined-map completion without a loan"))?;
        if loan.words[..6] != loan.source.geometry()[..]
            || loan.words[super::super::super::program_abi::JOIN_MAP_INITIALIZED_WORD]
                != loan.count as u64
            || args[0] > if loan.active { loan.count as u64 } else { 0 }
            || fusing.bytes.len() != loan.start
        {
            return Err(invalid(
                "joined-map result differs from its exact initialized loan",
            ));
        }
        let end = loan
            .start
            .checked_add(loan.count)
            .ok_or_else(|| invalid("joined-map initialized extent overflow"))?;
        if end > fusing.bytes.capacity() {
            return Err(invalid("joined-map initialized extent exceeds its backing"));
        }
        if fusing.bytes.capacity() != loan.capacity
            || unsafe { fusing.bytes.as_mut_ptr().add(loan.start) } as u64 != loan.target
        {
            return Err(invalid(
                "joined-map initialized backing changed during its loan",
            ));
        }
        // SAFETY: this exact owned source/geometry and non-overlapping output
        // extent were checked before the only emitted call. The private kernel
        // publishes its extent only after every selected row has been copied,
        // including the raw suffix after a missing key. No source/metadata/
        // target owner moved or resized while that synchronous loan was live.
        // Invalid/short/replayed completions leave the old length untouched.
        unsafe { fusing.bytes.set_len(end) };
        fusing.next = loan.source.following();
        if loan.active && args[0] < loan.count as u64 {
            fusing.missing = Some(add(loan.start as u64, args[0])?);
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
        let call = descriptor.calls[site]
            .as_ref()
            .ok_or_else(|| invalid("joined map lost its prepared join factory"))?;
        // The original full join envelope was admitted before allocation. Move
        // only the actual unique backing's capacity claim to the private owner;
        // its unused canonical-construction capacity now retires. No frozen
        // intermediate, shape, canonical nodes, import or ownership conversion
        // is constructed merely to consume it at the next proved operation.
        let capacity = text.capacity() as u64;
        let backing = envelope.split_capacity(capacity)?;
        let owner = call
            .finish_private_string_buffer(text, backing, &self.library)
            .map_err(|e| library(self, site, e))?;
        drop(envelope);
        let memory = self.library.memory_budget();
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
                owner,
                count: payload,
                processed: missing.unwrap_or(payload),
                slot: descriptor.selected_joins.regions[region].output().get(),
                initialized: false,
            }),
        });
        #[cfg(all(test, feature = "canonical-native-program-host"))]
        faults::after_mapped(self)?;
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
