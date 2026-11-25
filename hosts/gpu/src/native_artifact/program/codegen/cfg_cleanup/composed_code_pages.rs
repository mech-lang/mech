//! Optional shared pages for an already admitted, immutable byte lookup table.
use crate::native_artifact::codegen::compilation_inputs::CompilationInputs;
use cranelift_codegen::{
    cursor::FuncCursor,
    ir::{InstBuilder, MemFlags, Value, types},
};
use std::collections::BTreeMap;

pub(super) struct Packed {
    pub shift: u8,
    pub pages_offset: i32,
    pub bytes: Box<[u8]>,
}

pub(super) fn selected_shift(compiler_inputs: &CompilationInputs) -> Option<u8> {
    compiler_inputs
        .var("MECH_COMPOSE_IMMUTABLE_COST_CODE_PAGE_SHIFT")
        .ok()?
        .parse::<u8>()
        .ok()
        .filter(|shift| (4..=12).contains(shift))
}

/// Retain dense storage unless a one-byte directory saves actual data bytes.
/// Every page is initialized, including unused bytes of a final partial page.
pub(super) fn pack(input: &[u8], shift: u8) -> Option<Packed> {
    if !(4..=12).contains(&shift) || input.is_empty() || input.len() > 65536 {
        return None;
    }
    let page_size = 1usize << shift;
    let mut directory = Vec::with_capacity(input.len().div_ceil(page_size));
    let mut identifiers = BTreeMap::<Vec<u8>, u8>::new();
    let mut pages = Vec::new();
    for chunk in input.chunks(page_size) {
        let mut page = vec![0; page_size];
        page[..chunk.len()].copy_from_slice(chunk);
        let identifier = if let Some(&identifier) = identifiers.get(&page) {
            identifier
        } else {
            let identifier = u8::try_from(identifiers.len()).ok()?;
            pages.extend_from_slice(&page);
            identifiers.insert(page, identifier);
            identifier
        };
        directory.push(identifier);
    }
    if directory.len() + pages.len() >= input.len() {
        return None;
    }
    let pages_offset = i32::try_from(directory.len()).ok()?;
    directory.extend_from_slice(&pages);
    Some(Packed {
        shift,
        pages_offset,
        bytes: directory.into_boxed_slice(),
    })
}

/// The caller proves `word < original_input_length`. This changes storage
/// only: neither the source index proof nor the cost-array admission changes.
pub(super) fn load_code(
    c: &mut FuncCursor<'_>,
    codes: Value,
    word: Value,
    packed: Option<&Packed>,
) -> Value {
    let flags = MemFlags::new();
    if let Some(packed) = packed {
        let page = c.ins().ushr_imm(word, i64::from(packed.shift));
        let address = c.ins().iadd(codes, page);
        let page = c.ins().uload8(types::I64, flags, address, 0);
        let page = c.ins().ishl_imm(page, i64::from(packed.shift));
        let low = c.ins().band_imm(word, (1i64 << packed.shift) - 1);
        let offset = c.ins().bor(page, low);
        let address = c.ins().iadd(codes, offset);
        c.ins()
            .uload8(types::I64, flags, address, packed.pages_offset)
    } else {
        let address = c.ins().iadd(codes, word);
        c.ins().uload8(types::I64, flags, address, 0)
    }
}

#[cfg(test)]
#[path = "composed_code_pages_tests.rs"]
mod tests;
