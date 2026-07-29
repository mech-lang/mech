use crate::{BufferPositionMismatchError, FinalBufferLengthMismatchError};
use byteorder::{LittleEndian, WriteBytesExt};
use mech_core::*;
use std::collections::{HashMap, HashSet};
use std::io::{Cursor, Write};

#[derive(Debug)]
pub struct CompileCtx {
  reg_map: HashMap<usize, Register>,
  initialized_ptrs: HashSet<usize>,
  symbols: HashMap<u64, Register>,
  symbol_ptrs: HashMap<u64, usize>,
  dictionary: HashMap<u64, String>,
  mutable_symbols: HashSet<u64>,
  types: TypeSection,
  features: HashSet<FeatureFlag>,
  const_entries: Vec<ConstEntry>,
  const_blob: Vec<u8>,
  instrs: Vec<EncodedInstr>,
  next_reg: Register,
}

impl CompileCtx {
  pub fn new() -> Self {
    Self {
      reg_map: HashMap::new(),
      initialized_ptrs: HashSet::new(),
      symbols: HashMap::new(),
      mutable_symbols: HashSet::new(),
      dictionary: HashMap::new(),
      types: TypeSection::new(),
      symbol_ptrs: HashMap::new(),
      features: HashSet::new(),
      const_entries: Vec::new(),
      const_blob: Vec::new(),
      instrs: Vec::new(),
      next_reg: 0,
    }
  }

  pub fn clear(&mut self) {
    self.reg_map.clear();
    self.initialized_ptrs.clear();
    self.symbols.clear();
    self.dictionary.clear();
    self.mutable_symbols.clear();
    self.types = TypeSection::new();
    self.features.clear();
    self.const_entries.clear();
    self.const_blob.clear();
    self.instrs.clear();
    self.next_reg = 0;
  }

  pub fn compile(&mut self) -> MResult<Vec<u8>> {
    let header_size = ByteCodeHeader::HEADER_SIZE as u64;
    let feat_bytes_len = 4 + (self.features.len() as u64) * 8;
    let types_bytes_len = self.types.byte_len();
    let const_tbl_len = (self.const_entries.len() as u64) * ConstEntry::byte_len();
    let const_blob_len = self.const_blob.len() as u64;
    let symbols_len = (self.symbols.len() as u64) * 13;
    let instr_bytes_len = self.instrs.iter().map(|instruction| instruction.byte_len()).sum();
    let dict_len = self.dictionary.values().map(|name| name.len() as u64 + 12).sum();

    let mut offset = header_size;
    let feature_off = offset;
    offset += feat_bytes_len;
    let types_off = offset;
    offset += types_bytes_len;
    let const_tbl_off = offset;
    offset += const_tbl_len;
    let const_blob_off = offset;
    offset += const_blob_len;
    let symbols_off = offset;
    offset += symbols_len;
    let instr_off = offset;
    offset += instr_bytes_len;
    let dict_off = offset;
    offset += dict_len;

    let file_len_before_trailer = offset;
    let full_file_len = file_len_before_trailer + 4;
    let header = ByteCodeHeader {
      magic: *b"MECH",
      version: 1,
      mech_ver: parse_version_to_u16(env!("CARGO_PKG_VERSION")).unwrap(),
      flags: 0,
      reg_count: self.next_reg,
      instr_count: self.instrs.len() as u32,
      feature_count: self.features.len() as u32,
      feature_off,
      types_count: self.types.entries.len() as u32,
      types_off,
      const_count: self.const_entries.len() as u32,
      const_tbl_off,
      const_tbl_len,
      const_blob_off,
      const_blob_len,
      symbols_len,
      symbols_off,
      instr_off,
      instr_len: instr_bytes_len,
      dict_len,
      dict_off,
      reserved: 0,
    };

    let mut buffer = Cursor::new(Vec::with_capacity(full_file_len as usize));
    header.write_to(&mut buffer)?;

    buffer.write_u32::<LittleEndian>(self.features.len() as u32)?;
    for feature in &self.features {
      buffer.write_u64::<LittleEndian>(feature.as_u64())?;
    }

    self.types.write_to(&mut buffer)?;

    for entry in &self.const_entries {
      entry.write_to(&mut buffer)?;
    }
    if !self.const_blob.is_empty() {
      buffer.write_all(&self.const_blob)?;
    }

    for (id, register) in &self.symbols {
      SymbolEntry::new(*id, self.mutable_symbols.contains(id), *register).write_to(&mut buffer)?;
    }

    for instruction in &self.instrs {
      instruction.write_to(&mut buffer)?;
    }

    for (id, name) in &self.dictionary {
      DictEntry::new(*id, name).write_to(&mut buffer)?;
    }

    let position = buffer.position();
    if position != file_len_before_trailer {
      return Err(
        MechError::new(
          BufferPositionMismatchError {
            expected: file_len_before_trailer,
            got: position,
          },
          None,
        )
        .with_compiler_loc(),
      );
    }

    let checksum = crc32fast::hash(buffer.get_ref().as_slice());
    buffer.write_u32::<LittleEndian>(checksum)?;

    if buffer.position() != full_file_len {
      return Err(
        MechError::new(
          FinalBufferLengthMismatchError {
            expected: full_file_len,
            got: buffer.position(),
          },
          None,
        )
        .with_compiler_loc(),
      );
    }

    Ok(buffer.into_inner())
  }

  pub fn requirements(&self) -> &HashSet<FeatureFlag> {
    &self.features
  }

  fn alloc_register_for_ptr(&mut self, pointer: usize) -> Register {
    if let Some(&register) = self.reg_map.get(&pointer) {
      return register;
    }
    let register = self.next_reg;
    self.next_reg += 1;
    self.reg_map.insert(pointer, register);
    register
  }

  fn register_for_ptr_with_initialization_status(
    &mut self,
    pointer: usize,
  ) -> (Register, bool) {
    let register = self.alloc_register_for_ptr(pointer);
    let needs_initialization = self.initialized_ptrs.insert(pointer);
    (register, needs_initialization)
  }

  fn compile_const(&mut self, bytes: &[u8], value_kind: ValueKind) -> MResult<u32> {
    let type_id = self.types.get_or_intern(&value_kind);
    let align = value_kind.align();
    let next_blob_len = self.const_blob.len() as u64;
    let padded_offset = align_up(next_blob_len, align as u64);
    if padded_offset > next_blob_len {
      self.const_blob.resize(padded_offset as usize, 0);
    }
    self.features.insert(FeatureFlag::Builtin(value_kind.to_feature_kind()));
    let offset = self.const_blob.len() as u64;
    self.const_blob.extend_from_slice(bytes);
    let length = (self.const_blob.len() as u64) - offset;
    let entry = ConstEntry {
      type_id,
      enc: ConstEncoding::Inline,
      align: align as u8,
      flags: 0,
      reserved: 0,
      offset,
      length,
    };
    let const_id = self.const_entries.len() as u32;
    self.const_entries.push(entry);
    Ok(const_id)
  }
}

impl BytecodeCompilerContext for CompileCtx {
  fn register_for_ptr_with_initialization_status(
    &mut self,
    pointer: usize,
  ) -> (Register, bool) {
    CompileCtx::register_for_ptr_with_initialization_status(self, pointer)
  }

  fn compile_const(&mut self, bytes: &[u8], kind: ValueKind) -> MResult<u32> {
    CompileCtx::compile_const(self, bytes, kind)
  }

  fn define_symbol(
    &mut self,
    pointer: usize,
    register: Register,
    name: &str,
    mutable: bool,
  ) {
    let symbol_id = hash_str(name);
    self.symbols.insert(symbol_id, register);
    self.symbol_ptrs.insert(symbol_id, pointer);
    self.dictionary.insert(symbol_id, name.to_string());
    if mutable {
      self.mutable_symbols.insert(symbol_id);
    }
  }

  fn require(&mut self, requirement: FeatureFlag) {
    self.features.insert(requirement);
  }

  fn emit_const_load(&mut self, destination: Register, constant: u32) {
    self.instrs.push(EncodedInstr::ConstLoad { dst: destination, const_id: constant });
  }

  fn emit_nullop(&mut self, function: u64, destination: Register) {
    self.instrs.push(EncodedInstr::NullOp { fxn_id: function, dst: destination });
  }

  fn emit_unop(&mut self, function: u64, destination: Register, source: Register) {
    self.instrs.push(EncodedInstr::UnOp {
      fxn_id: function,
      dst: destination,
      src: source,
    });
  }

  fn emit_binop(
    &mut self,
    function: u64,
    destination: Register,
    lhs: Register,
    rhs: Register,
  ) {
    self.instrs.push(EncodedInstr::BinOp {
      fxn_id: function,
      dst: destination,
      lhs,
      rhs,
    });
  }

  fn emit_ternop(
    &mut self,
    function: u64,
    destination: Register,
    a: Register,
    b: Register,
    c: Register,
  ) {
    self.instrs.push(EncodedInstr::TernOp {
      fxn_id: function,
      dst: destination,
      a,
      b,
      c,
    });
  }

  fn emit_quadop(
    &mut self,
    function: u64,
    destination: Register,
    a: Register,
    b: Register,
    c: Register,
    d: Register,
  ) {
    self.instrs.push(EncodedInstr::QuadOp {
      fxn_id: function,
      dst: destination,
      a,
      b,
      c,
      d,
    });
  }

  fn emit_varop(
    &mut self,
    function: u64,
    destination: Register,
    arguments: Vec<Register>,
  ) {
    self.instrs.push(EncodedInstr::VarArg {
      fxn_id: function,
      dst: destination,
      args: arguments,
    });
  }

  fn emit_ret(&mut self, source: Register) {
    self.instrs.push(EncodedInstr::Ret { src: source });
  }
}

#[inline]
fn align_up(offset: u64, align: u64) -> u64 {
  if align == 0 {
    return offset;
  }
  ((offset + align - 1) / align) * align
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn compile_context_initializes_pointer_register_once() {
    let mut context = CompileCtx::new();
    let pointer_a = 100usize;
    let pointer_b = 200usize;

    let (register_a, initializes_a) =
      context.register_for_ptr_with_initialization_status(pointer_a);
    assert!(initializes_a);

    let (register_a_again, initializes_a_again) =
      context.register_for_ptr_with_initialization_status(pointer_a);
    assert_eq!(register_a_again, register_a);
    assert!(!initializes_a_again);

    let (register_b, initializes_b) =
      context.register_for_ptr_with_initialization_status(pointer_b);
    assert_ne!(register_b, register_a);
    assert!(initializes_b);

    context.clear();

    let (register_a_after_clear, initializes_a_after_clear) =
      context.register_for_ptr_with_initialization_status(pointer_a);
    assert_eq!(register_a_after_clear, 0);
    assert!(initializes_a_after_clear);
  }
}
