use crate::*;
use super::*;

#[derive(Debug)]
pub struct CompileCtx {
  // pointer identity -> register index
  pub reg_map: HashMap<usize, Register>,
  // symbol identity -> register index
  pub symbols: HashMap<u64, Register>,
  // symbol identity -> pointer identity
  pub symbol_ptrs: HashMap<u64, usize>,
  // symbol identity -> symbol name
  pub dictionary: HashMap<u64, String>,
  // mutable symbols
  pub mutable_symbols: HashSet<u64>,
  pub types: TypeSection,
  pub features: HashSet<FeatureFlag>,
  pub const_entries: Vec<ConstEntry>,
  pub const_blob: Vec<u8>,
  pub instrs: Vec<EncodedInstr>,
  pub next_reg: Register,
}

#[cfg(feature = "compiler")]
impl CompileCtx {
  pub fn new() -> Self {
    Self {
      reg_map: HashMap::new(),
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

  pub fn define_symbol(&mut self, id: usize, reg: Register, name: &str, mutable: bool) {
    let symbol_id = hash_str(name);
    self.symbols.insert(symbol_id, reg);
    self.symbol_ptrs.insert(symbol_id, id);
    self.dictionary.insert(symbol_id, name.to_string());
    if mutable {
      self.mutable_symbols.insert(symbol_id);
    }
  }

  pub fn alloc_register_for_ptr(&mut self, ptr: usize) -> Register {
    if let Some(&r) = self.reg_map.get(&ptr) { return r; }
    let r = self.next_reg;
    self.next_reg += 1;
    self.reg_map.insert(ptr, r);
    r
  }

  pub fn emit_const_load(&mut self, dst: Register, const_id: u32) {
    self.instrs.push(EncodedInstr::ConstLoad { dst, const_id });
  }
  pub fn emit_nullop(&mut self, fxn_id: u64, dst: Register) {
    self.instrs.push(EncodedInstr::NullOp { fxn_id, dst });
  }
  pub fn emit_unop(&mut self, fxn_id: u64, dst: Register, src: Register) {
    self.instrs.push(EncodedInstr::UnOp { fxn_id, dst, src });
  }
  pub fn emit_binop(&mut self, fxn_id: u64, dst: Register, lhs: Register, rhs: Register) {
    self.instrs.push(EncodedInstr::BinOp { fxn_id, dst, lhs, rhs });
  }
  pub fn emit_ternop(&mut self, fxn_id: u64, dst: Register, a: Register, b: Register, c: Register) {
    self.instrs.push(EncodedInstr::TernOp { fxn_id, dst, a, b, c });
  }
  pub fn emit_quadop(&mut self, fxn_id: u64, dst: Register, a: Register, b: Register, c: Register, d: Register) {
    self.instrs.push(EncodedInstr::QuadOp { fxn_id, dst, a, b, c, d });
  }
  pub fn emit_vararg(&mut self, fxn_id: u64, dst: Register, args: Vec<Register>) {
    self.instrs.push(EncodedInstr::VarArg { fxn_id, dst, args });
  }
  pub fn emit_ret(&mut self, src: Register) {
    self.instrs.push(EncodedInstr::Ret { src })
  }
  
  pub fn compile_const(&mut self, bytes: &[u8], value_kind: ValueKind) -> MResult<u32> {
    let type_id = self.types.get_or_intern(&value_kind);
    let align = value_kind.align();
    let next_blob_len = self.const_blob.len() as u64;
    let padded_off = align_up(next_blob_len, align as u64);
    if padded_off > next_blob_len {
      // add zero bytes padding to align the next write
      self.const_blob.resize(padded_off as usize, 0);
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

  pub fn compile(&mut self) -> MResult<Vec<u8>> {

    let header_size = ByteCodeHeader::HEADER_SIZE as u64;
    let feat_bytes_len: u64 = 4 + (self.features.len() as u64) * 8;
    let types_bytes_len: u64 = self.types.byte_len();
    let const_tbl_len: u64 = (self.const_entries.len() as u64) * ConstEntry::byte_len();
    let const_blob_len: u64 = self.const_blob.len() as u64;
    let symbols_len: u64 = (self.symbols.len() as u64) * 13; // 8 bytes for id, 1 byte for mutable, 4 for reg
    let instr_bytes_len: u64 = self.instrs.iter().map(|i| i.byte_len()).sum();
    let dict_len: u64 = self.dictionary.values().map(|s| s.len() as u64 + 12).sum(); // 8 bytes for id, 4 for string length

    let mut offset = header_size;                           // bytes in header
    let feature_off = offset; offset += feat_bytes_len;     // offset to feature section
    let types_off = offset; offset += types_bytes_len;      // offset to types section
    let const_tbl_off = offset; offset += const_tbl_len;    // offset to constant table
    let const_blob_off = offset; offset += const_blob_len;  // offset to constant blob
    let symbols_off = offset; offset += symbols_len;        // offset to symbol section
    let instr_off = offset; offset += instr_bytes_len;      // offset to instruction stream
    let dict_off = offset; offset += dict_len;              // offset to dictionary section
    
    let file_len_before_trailer = offset;
    let trailer_len = 4u64;
    let full_file_len = file_len_before_trailer + trailer_len;

    // The header!
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
    
    let mut buf = Cursor::new(Vec::<u8>::with_capacity(full_file_len as usize));

    // 1. Write the header
    header.write_to(&mut buf)?;

    // 2. Write features
    buf.write_u32::<LittleEndian>(self.features.len() as u32)?;
    for f in &self.features {
      buf.write_u64::<LittleEndian>(f.as_u64())?;
    }

    // 3. Write types
    self.types.write_to(&mut buf)?;

    // 4. write consts
    for entry in &self.const_entries {
      entry.write_to(&mut buf)?;
    }

    if !self.const_blob.is_empty() {
      buf.write_all(&self.const_blob)?;
    }

    // 5. write symbols
    for (id, reg) in &self.symbols {
      let mutable = self.mutable_symbols.contains(id);
      let entry = SymbolEntry::new(*id, mutable, *reg);
      entry.write_to(&mut buf)?;
    }

    // 6. write instructions. This is where the action is!
    for ins in &self.instrs {
      ins.write_to(&mut buf)?;
    }

    // 7. write dictionary
    for (id, name) in &self.dictionary {
      let dict_entry = DictEntry::new(*id, name);
      dict_entry.write_to(&mut buf)?;
    }

    // sanity check: the position should equal file_len_before_trailer
    let pos = buf.position();
    if pos != file_len_before_trailer {
      return Err(MechError {file: file!().to_string(),tokens: vec![],msg: format!("Buffer position mismatch: expected {}, got {}", file_len_before_trailer, pos),id: line!(),kind: MechErrorKind::GenericError("Buffer position mismatch".to_string()),});
    }

    let bytes_so_far = buf.get_ref().as_slice();
    let checksum = crc32fast::hash(bytes_so_far);
    buf.write_u32::<LittleEndian>(checksum)?;

    if buf.position() != full_file_len {
      return Err(MechError {file: file!().to_string(),tokens: vec![],msg: format!("Final buffer length mismatch: expected {}, got {}", full_file_len, buf.position()),id: line!(),kind: MechErrorKind::GenericError("Final buffer length mismatch".to_string()),});
    }

    Ok(buf.into_inner())
  }
}

#[inline]
fn align_up(offset: u64, align: u64) -> u64 {
  if align == 0 { return offset; }
  ((offset + align - 1) / align) * align
}
