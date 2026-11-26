//! Same-account fresh Core owners for complete native phase map results.
//! Every allocation is entry-only and speculative; source owners remain intact.
use super::{Error, Invocation, invalid};
use crate::native_artifact::program::prepare::readonly_numeric::{ArrayStorage, Proof};
use crate::native_artifact::program::runtime::buffer_frames::PrivateNumericArray;
use mech_core::{
    ManagedMemoryBudget, Value,
    snapshot::{F64Bits, NumericMatrixViewMut, PrivateNumericMatrix, UnsignedMatrixBuffer},
};
use mech_engine::native_artifact::NativeScalarKind;

impl Invocation<'_, '_, '_> {
    pub(super) fn numeric_entry_envelope(
        &self,
        proof: &Proof,
        array: usize,
    ) -> Result<Option<(u64, u64)>, Error> {
        if matches!(proof.arrays[array].storage, ArrayStorage::PhaseLocal { .. }) {
            let bank = proof
                .map_banks
                .iter()
                .find(|b| b.array == array)
                .ok_or_else(|| invalid("local map has no source-derived bank"))?;
            return Ok(bank.admission(self.descriptor.metadata().observation_schema_bytes));
        }
        let Some(owner) = self.numeric_array_owner(&proof.arrays[array])? else {
            return Ok(None);
        };
        let memory = self.state.library.memory_budget();
        Ok(owner
            .memory_budget_retained_bytes(&memory)
            .zip(owner.memory_budget_retained_nodes(&memory)))
    }
    pub(super) fn numeric_map_banks_envelope(&self, proof: &Proof) -> Result<(u64, u64), Error> {
        proof
            .map_banks
            .iter()
            .try_fold((0u64, 0u64), |(bytes, nodes), bank| {
                let (b, n) = bank
                    .admission(self.descriptor.metadata().observation_schema_bytes)
                    .ok_or_else(|| invalid("owned map bank admission overflows"))?;
                Ok((
                    bytes
                        .checked_add(b)
                        .ok_or_else(|| invalid("owned map byte sum overflows"))?,
                    nodes
                        .checked_add(n)
                        .ok_or_else(|| invalid("owned map node sum overflows"))?,
                ))
            })
    }
    pub(super) fn numeric_publication_bound(
        &self,
        proof: &Proof,
        array: usize,
    ) -> Result<Option<u64>, Error> {
        let current = self
            .numeric_entry_envelope(proof, array)?
            .map(|(bytes, _)| bytes);
        let fresh = proof
            .map_banks
            .iter()
            .find(|b| b.array == array)
            .and_then(|b| b.admission(self.descriptor.metadata().observation_schema_bytes))
            .map(|(bytes, _)| bytes);
        Ok(match (current, fresh) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (a, b) => a.or(b),
        })
    }
    pub(super) fn initialize_numeric_map_banks(&mut self, id: usize) -> Result<bool, Error> {
        let descriptor = self.descriptor;
        let metadata = descriptor.metadata();
        let region = &metadata.closed_tails.regions[id];
        let proof = region
            .numeric
            .as_ref()
            .ok_or_else(|| invalid("owned map entry lost proof"))?;
        if proof.map_banks.is_empty() {
            return Ok(true);
        }
        let memory = self.state.library.memory_budget();
        let result = (|| {
            let bytes = (proof.map_banks.len() as u64)
                .checked_mul(
                    (std::mem::size_of::<(usize, PrivateNumericMatrix)>() as u64)
                        .checked_add(128)
                        .ok_or_else(|| invalid("owned map staging size overflows"))?,
                )
                .and_then(|n| n.checked_add(4096))
                .ok_or_else(|| invalid("owned map staging overflows"))?;
            let Ok(_staging) = memory.reserve_capacity(bytes) else {
                return Ok(false);
            };
            let Ok(_nodes) = memory.reserve_nodes(proof.map_banks.len() as u64 + 1) else {
                return Ok(false);
            };
            let mut banks = Vec::new();
            if banks.try_reserve_exact(proof.map_banks.len()).is_err() {
                return Ok(false);
            }
            for bank in &proof.map_banks {
                let array = &proof.arrays[bank.array];
                let Some((bytes, _)) = bank.admission(metadata.observation_schema_bytes) else {
                    return Ok(false);
                };
                let Some(private) = make_bank(
                    &metadata.schemas,
                    array.schema,
                    bank.extents,
                    bank.elements,
                    bytes,
                    &memory,
                ) else {
                    return Ok(false);
                };
                banks.push((bank.array, private));
            }
            let mut locals = Vec::new();
            if locals.try_reserve_exact(proof.map_banks.len()).is_err() {
                return Ok(false);
            }
            for array in &proof.arrays {
                if let ArrayStorage::PhaseLocal { value, .. } = array.storage {
                    let key = metadata
                        .schemas
                        .entry(array.schema)
                        .ok_or_else(|| invalid("local map schema missing"))?
                        .key();
                    locals.push((value, array.schema, key));
                }
            }
            if !self
                .state
                .buffer_frames
                .prepare_map_locals(id, &locals, &memory)?
            {
                return Ok(false);
            }
            for (array, mut private) in banks {
                let pointer = match private.elements_mut() {
                    NumericMatrixViewMut::Bool(v) => v.as_mut_ptr() as u64,
                    NumericMatrixViewMut::U8(v) => v.as_mut_ptr() as u64,
                    NumericMatrixViewMut::U16(v) => v.as_mut_ptr() as u64,
                    NumericMatrixViewMut::U32(v) => v.as_mut_ptr() as u64,
                    NumericMatrixViewMut::U64(v) => v.as_mut_ptr() as u64,
                    NumericMatrixViewMut::I64(v) => v.as_mut_ptr() as u64,
                    NumericMatrixViewMut::F64(v) => v.as_mut_ptr() as u64,
                };
                let target = self.state.buffer_frames.private_array_mut(id, array)?;
                if target.is_some() {
                    return Err(invalid("owned map overwrites another private loan"));
                }
                *target = Some(PrivateNumericArray::Scalar(private));
                self.write_borrowed_word(
                    2 + proof
                        .map_bank(array)
                        .ok_or_else(|| invalid("owned map pointer home missing"))?,
                    pointer,
                )?;
            }
            if let Some(array) = proof
                .result
                .direct_array()
                .filter(|&a| proof.map_bank(a).is_some())
            {
                let fallback = self.numeric_array_owner(&proof.arrays[array])?.cloned();
                let private = self.state.buffer_frames.private_map_bank(id, array)?;
                let Ok(ticket) = self.state.arena.prepare_private_put(
                    region.tail.returned().get(),
                    private,
                    fallback.as_ref(),
                    &mut self.state.control.work,
                ) else {
                    return Ok(false);
                };
                self.state
                    .buffer_frames
                    .set_private_publication(id, ticket)?;
            }
            for (index, alias) in proof
                .cold_aliases
                .iter()
                .flat_map(|(_, aliases)| aliases)
                .enumerate()
            {
                if proof.map_bank(alias.array).is_none() {
                    continue;
                }
                let fallback = self
                    .numeric_array_owner(&proof.arrays[alias.array])?
                    .cloned();
                let private = self.state.buffer_frames.private_map_bank(id, alias.array)?;
                let Ok(ticket) = self.state.arena.prepare_private_put(
                    alias.value,
                    private,
                    fallback.as_ref(),
                    &mut self.state.control.work,
                ) else {
                    return Ok(false);
                };
                self.state
                    .buffer_frames
                    .set_private_alias(id, index, alias.value, ticket)?;
            }
            Ok(true)
        })();
        if !matches!(result, Ok(true)) {
            self.state.buffer_frames.discard_prepared_map_banks(id)?;
        }
        result
    }
}
fn make_bank(
    schemas: &std::sync::Arc<mech_core::SchemaTable>,
    schema: mech_core::SchemaId,
    extents: [u64; 2],
    elements: u64,
    bytes: u64,
    memory: &ManagedMemoryBudget,
) -> Option<PrivateNumericMatrix> {
    let mut reservation = memory.reserve_capacity(bytes).ok()?;
    let definition = schemas.get(schema)?;
    let mech_core::SchemaBody::Matrix { element, .. } = definition.body() else {
        return None;
    };
    let kind = NativeScalarKind::from_schema_body(element)?;
    let shape = mech_core::shape_for_resolved_extents(definition, &extents).ok()?;
    let count = usize::try_from(elements).ok()?;
    if count == 0 || extents[0].checked_mul(extents[1])? != elements {
        return None;
    }
    macro_rules! boxed {
        ($ty:ty, $zero:expr, $constructor:ident) => {{
            let mut cells = Vec::<$ty>::new();
            cells.try_reserve_exact(count).ok()?;
            cells.resize(count, $zero);
            Value::$constructor(schema, shape, cells.into_boxed_slice(), schemas.clone()).ok()?
        }};
    }
    macro_rules! unsigned {
        ($ty:ty, $variant:ident) => {{
            let mut cells = Vec::<$ty>::new();
            cells.try_reserve_exact(count).ok()?;
            cells.resize(count, 0);
            Value::from_unsigned_matrix_buffer(
                schema,
                shape,
                UnsignedMatrixBuffer::$variant(cells),
                schemas.clone(),
            )
            .ok()?
        }};
    }
    let value = match kind {
        NativeScalarKind::Bool => boxed!(bool, false, from_bool_matrix),
        NativeScalarKind::I64 => boxed!(i64, 0, from_i64_matrix),
        NativeScalarKind::F64 => boxed!(F64Bits, F64Bits::from_bits(0), from_f64_matrix),
        NativeScalarKind::U8 => unsigned!(u8, U8),
        NativeScalarKind::U16 => unsigned!(u16, U16),
        NativeScalarKind::U32 => unsigned!(u32, U32),
        NativeScalarKind::U64 => unsigned!(u64, U64),
    };
    value.into_private_numeric_matrix(&mut reservation).ok()
}
