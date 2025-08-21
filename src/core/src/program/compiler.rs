use crate::*;
use byteorder::{LittleEndian, WriteBytesExt, ReadBytesExt};
use std::io::{self, Write, Read, SeekFrom, Seek, Cursor};
use std::fs::File;
use std::path::Path;
use std::hash::{Hash, Hasher};
use std::collections::{HashMap, HashSet};

// Byetecode Compiler
// ============================================================================

// Format:
// 1. Header
// 2. Features
// 3. Types
// 4. Constants
// 5. Symbols
// 6. Instructions
// 7. Dictionary (optional for human-readable names)

// Compilation Context
// ----------------------------------------------------------------------------

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ParsedProgram {
  pub header: ByteCodeHeader,
  pub features: Vec<u64>,
  pub types: TypeSection,
  pub const_entries: Vec<ParsedConstEntry>,
  pub const_blob: Vec<u8>,
  pub instr_bytes: Vec<u8>,
  pub symbols: HashMap<u64, Register>,
  pub instrs: Vec<DecodedInstr>,
  pub dictionary: HashMap<u64, String>,
}

impl ParsedProgram {

  pub fn from_bytes(bytes: &Vec<u8>) -> MResult<ParsedProgram> {
    load_program_from_bytes(bytes)
  }

  pub fn validate(&self) -> MResult<()> {
    // Check magic number
    if !self.header.validate_magic(b"MECH") {
      return Err(MechError {file: file!().to_string(), tokens: vec![], msg: "Invalid magic number".to_string(), id: line!(), kind: MechErrorKind::GenericError("Invalid magic number".to_string())});
    }
    // Check version number
    if self.header.version != 1 {
      return Err(MechError {file: file!().to_string(), tokens: vec![], msg: "Unsupported bytecode version".to_string(), id: line!(), kind: MechErrorKind::GenericError("Unsupported bytecode version".to_string())});
    }
    // Check mech version
    if self.header.mech_ver != parse_version_to_u16(env!("CARGO_PKG_VERSION")).unwrap() {
      return Err(MechError {file: file!().to_string(), tokens: vec![], msg: "Incompatible Mech version".to_string(), id: line!(), kind: MechErrorKind::GenericError("Incompatible Mech version".to_string())});
    }
    Ok(())
  }

  pub fn decode_const_entries(&self) -> MResult<Vec<Value>> {
    let mut out = Vec::with_capacity(self.const_entries.len());
    let blob_len = self.const_blob.len() as u64;

    for ce in &self.const_entries {
      // Only support Inline encoding for now
      if ce.enc != ConstEncoding::Inline as u8 {
        return Err(MechError {file: file!().to_string(),tokens: vec![],msg: "Unsupported constant encoding".to_string(),id: line!(),kind: MechErrorKind::GenericError("Unsupported constant encoding".to_string())});
      }

      // Bounds check
      if ce.offset.checked_add(ce.length).is_none() {
          return Err(MechError {file: file!().to_string(),tokens: vec![],msg: "Constant entry out of bounds".to_string(),id: line!(),kind: MechErrorKind::GenericError("Constant entry out of bounds".to_string())});
      }
      let end = ce.offset + ce.length;
      if end > blob_len {
        return Err(MechError {file: file!().to_string(),tokens: vec![],msg: "Constant entry out of bounds".to_string(),id: line!(),kind: MechErrorKind::GenericError("Constant entry out of bounds".to_string())});
      }

      // Alignment check (if your alignment semantics differ, change this)
      if !check_alignment(ce.offset, ce.align) {
        return Err(MechError {file: file!().to_string(),tokens: vec![],msg: "Constant entry alignment error".to_string(),id: line!(),kind: MechErrorKind::GenericError("Constant entry alignment error".to_string())});
      }

      // Copy bytes out (we clone into Vec<u8> to own data)
      let start = ce.offset as usize;
      let len = ce.length as usize;
      let data = self.const_blob[start .. start + len].to_vec();

      // get the type from the id
      let ty = &self.types.entries[ce.type_id as usize];
      let val: Value = match ty.tag {
        #[cfg(feature = "bool")]
        TypeTag::Bool => {
          if data.len() != 1 {
            return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "Bool const entry must be 1 byte".to_string(), id: line!(), kind: MechErrorKind::GenericError("Bool const entry must be 1 byte".to_string())});
          }
          let value = data[0] != 0;
          Value::Bool(Ref::new(value))
        },
        #[cfg(feature = "u8")]
        TypeTag::U8 => {
          if data.len() != 1 {
            return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "U8 const entry must be 1 byte".to_string(), id: line!(), kind: MechErrorKind::GenericError("U8 const entry must be 1 byte".to_string())});
          }
          let value = data[0];
          Value::U8(Ref::new(value))
        },
        #[cfg(feature = "f64")]
        TypeTag::F64 => {
          if data.len() != 8 {
            return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "F64 const entry must be 8 bytes".to_string(), id: line!(), kind: MechErrorKind::GenericError("F64 const entry must be 8 bytes".to_string())});
          }
          let value = f64::from_le_bytes(data.try_into().unwrap());
          Value::F64(Ref::new(F64::new(value)))
        },
        #[cfg(feature = "i64")]
        TypeTag::I64 => {
          if data.len() != 8 {
            return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "I64 const entry must be 8 bytes".to_string(), id: line!(), kind: MechErrorKind::GenericError("I64 const entry must be 8 bytes".to_string())});
          }
          let value = i64::from_le_bytes(data.try_into().unwrap());
          Value::I64(Ref::new(value))
        }
        // Add more types as needed
        _ => return Err(MechError{file: file!().to_string(), tokens: vec![], msg: format!("Unsupported constant type {:?}", ty.tag), id: line!(), kind: MechErrorKind::GenericError(format!("Unsupported constant type {:?}", ty.tag))}),
      };
      out.push(val);
    }
    Ok(out)
  }
}

fn check_alignment(offset: u64, align: u8) -> bool {
  // treat align==0 as invalid
  let align_val = align as u64;
  if align_val == 0 { return false; }
  (offset % align_val) == 0
}

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
  pub types: TypeSection,
  pub features: HashSet<FeatureFlag>,
  pub const_entries: Vec<ConstEntry>,
  pub const_blob: Vec<u8>,
  pub instrs: Vec<EncodedInstr>,
  pub next_reg: Register,
}

impl CompileCtx {
  pub fn new() -> Self {
    Self {
      reg_map: HashMap::new(),
      symbols: HashMap::new(),
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
    self.types = TypeSection::new();
    self.features.clear();
    self.const_entries.clear();
    self.const_blob.clear();
    self.instrs.clear();
    self.next_reg = 0;
  }

  pub fn define_symbol(&mut self, id: usize, reg: Register, name: &str) {
    let symbol_id = hash_str(name);
    self.symbols.insert(symbol_id, reg);
    self.symbol_ptrs.insert(symbol_id, id);
    self.dictionary.insert(symbol_id, name.to_string());
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
  pub fn emit_unop(&mut self, opcode: u64, dst: Register, src: Register) {
    self.instrs.push(EncodedInstr::UnOp { opcode, dst, src });
  }
  pub fn emit_binop(&mut self, opcode: u64, dst: Register, lhs: Register, rhs: Register) {
    self.instrs.push(EncodedInstr::BinOp { opcode, dst, lhs, rhs });
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
    let symbols_len: u64 = (self.symbols.len() as u64) * 12; // 8 bytes for id, 4 for reg
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
      let entry = SymbolEntry::new(*id, *reg);
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

// 1. Header
// ----------------------------------------------------------------------------

#[repr(C)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ByteCodeHeader {
  pub magic:        [u8; 4],   // e.g., b"MECH"
  pub version:        u8,      // bytecode format version
  pub mech_ver:       u16,     // Mech language version
  pub flags:          u16,     // reserved/feature bit
  pub reg_count:      u32,     // total virtual registers used
  pub instr_count:    u32,     // number of instructions
  
  pub feature_count:  u32,     // number of feature flags  
  pub feature_off:    u64,     // offset to feature flags (array of u64)

  pub types_count:    u32,     // number of types
  pub types_off:      u64,     // offset to type section

  pub const_count:    u32,     // number of constants (entries
  pub const_tbl_off:  u64,     // offset to constant table (array of entries)
  pub const_tbl_len:  u64,     // bytes in constant table area (entries only)
  pub const_blob_off: u64,     // offset to raw constant blob data
  pub const_blob_len: u64,     // bytes in blob (payloads

  pub symbols_len:    u64,     // number of symbols
  pub symbols_off:    u64,     // offset to symbol section
                               
  pub instr_off:      u64,     // offset to instruction stream
  pub instr_len:      u64,     // bytes of instruction stream

  pub dict_off:       u64,     // offset to dictionary
  pub dict_len:       u64,     // bytes in dictionary

  pub reserved:       u32,     // pad/alignment
}

impl ByteCodeHeader {
  // Header byte size when serialized. This is the number of bytes `write_to` will write.
  // (Computed from the sum of sizes of each field written in little-endian.)
  pub const HEADER_SIZE: usize = 4  // magic
    + 1   // version
    + 2   // mech_ver
    + 2   // flags
    + 4   // reg_count
    + 4   // instr_count
    + 4   // feature_count
    + 8   // feature_off
    + 4   // types_count
    + 8   // types_off
    + 4   // const_count
    + 8   // const_tbl_off
    + 8   // const_tbl_len
    + 8   // const_blob_off
    + 8   // const_blob_len
    + 8   // symbols_len
    + 8   // symbosl_off
    + 8   // instr_off
    + 8   // instr_len
    + 8   // dict_off
    + 8   // dict_len
    + 4;  // reserved

  // Serialize header using little-endian encoding.
  pub fn write_to(&self, w: &mut impl Write) -> io::Result<()> {
    // magic (4 bytes)
    w.write_all(&self.magic)?;

    // small fields
    w.write_u8(self.version)?;
    w.write_u16::<LittleEndian>(self.mech_ver)?;
    w.write_u16::<LittleEndian>(self.flags)?;

    // counts
    w.write_u32::<LittleEndian>(self.reg_count)?;
    w.write_u32::<LittleEndian>(self.instr_count)?;

    // features (count + offset)
    w.write_u32::<LittleEndian>(self.feature_count)?;
    w.write_u64::<LittleEndian>(self.feature_off)?;

    // types
    w.write_u32::<LittleEndian>(self.types_count)?;
    w.write_u64::<LittleEndian>(self.types_off)?;

    // constants table / blob
    w.write_u32::<LittleEndian>(self.const_count)?;
    w.write_u64::<LittleEndian>(self.const_tbl_off)?;
    w.write_u64::<LittleEndian>(self.const_tbl_len)?;
    w.write_u64::<LittleEndian>(self.const_blob_off)?;
    w.write_u64::<LittleEndian>(self.const_blob_len)?;

    // symbols
    w.write_u64::<LittleEndian>(self.symbols_len)?;
    w.write_u64::<LittleEndian>(self.symbols_off)?;

    // instructions
    w.write_u64::<LittleEndian>(self.instr_off)?;
    w.write_u64::<LittleEndian>(self.instr_len)?;

    // dictionary
    w.write_u64::<LittleEndian>(self.dict_off)?;
    w.write_u64::<LittleEndian>(self.dict_len)?;

    // footer
    w.write_u32::<LittleEndian>(self.reserved)?;
    Ok(())
  }

  // Read a header. Expects the same layout as `write_to`.
  pub fn read_from(r: &mut impl Read) -> io::Result<Self> {
    let mut magic = [0u8; 4];
    r.read_exact(&mut magic)?;

    let version = r.read_u8()?;
    let mech_ver = r.read_u16::<LittleEndian>()?;
    let flags = r.read_u16::<LittleEndian>()?;

    let reg_count = r.read_u32::<LittleEndian>()?;
    let instr_count = r.read_u32::<LittleEndian>()?;

    let feature_count = r.read_u32::<LittleEndian>()?;
    let feature_off = r.read_u64::<LittleEndian>()?;

    let types_count = r.read_u32::<LittleEndian>()?;
    let types_off = r.read_u64::<LittleEndian>()?;

    let const_count = r.read_u32::<LittleEndian>()?;
    let const_tbl_off = r.read_u64::<LittleEndian>()?;
    let const_tbl_len = r.read_u64::<LittleEndian>()?;
    let const_blob_off = r.read_u64::<LittleEndian>()?;
    let const_blob_len = r.read_u64::<LittleEndian>()?;

    let symbols_len = r.read_u64::<LittleEndian>()?;
    let symbols_off = r.read_u64::<LittleEndian>()?;

    let instr_off = r.read_u64::<LittleEndian>()?;
    let instr_len = r.read_u64::<LittleEndian>()?;

    let dict_off = r.read_u64::<LittleEndian>()?;
    let dict_len = r.read_u64::<LittleEndian>()?;

    let reserved = r.read_u32::<LittleEndian>()?;

    Ok(Self {
      magic,
      version,
      mech_ver,
      flags,
      reg_count,
      instr_count,
      feature_count,
      feature_off,
      types_count,
      types_off,
      const_count,
      const_tbl_off,
      const_tbl_len,
      const_blob_off,
      const_blob_len,
      instr_off,
      instr_len,
      symbols_len,
      symbols_off,
      dict_off,
      dict_len,
      reserved,
    })
  }

  // Quick check: does the header magic match the expected magic?
  pub fn validate_magic(&self, expected: &[u8;4]) -> bool {
    &self.magic == expected
  }
}

// 2. Features
// ----------------------------------------------------------------------------

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FeatureKind {
  I8=1, I16, I32, I64, I128,
  U8, U16, U32, U64, U128,
  F32, F64,
  String,
  Bool,
  Complex,
  Rational,
  Matrix1, Matrix2, Matrix3, Matrix4,
  Matrix2x3, Matrix3x2,
  RowVector2, RowVector3, RowVector4,
  Vector2, Vector3, Vector4,
  Add, Sub, Mul, Div, Mod,
  MatMul,
  Custom = 0xFFFF,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FeatureFlag {
  Builtin(FeatureKind),
  Custom(u64),
}

impl FeatureFlag {
  pub fn as_u64(&self) -> u64 {
    match self {
      FeatureFlag::Builtin(f) => *f as u64,
      FeatureFlag::Custom(c) => *c,
    }
  }
}

// 3. Type Section
// ----------------------------------------------------------------------------

#[repr(u16)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum TypeTag {
  U8=1, U16, U32, U64, U128, I8, I16, I32, I64, I128,
  F32, F64, ComplexNumber, RationalNumber, String, Bool, Id, Index, Empty, Any,
  Matrix, EnumTag, Record, Map, Atom, Table, Tuple, Reference, Set, OptionT,
}

impl TypeTag {
  pub fn from_u16(tag: u16) -> Option<Self> {
    match tag {
      1 => Some(TypeTag::U8), 2 => Some(TypeTag::U16), 3 => Some(TypeTag::U32),
      4 => Some(TypeTag::U64), 5 => Some(TypeTag::U128), 6 => Some(TypeTag::I8),
      7 => Some(TypeTag::I16), 8 => Some(TypeTag::I32), 9 => Some(TypeTag::I64),
      10 => Some(TypeTag::I128), 11 => Some(TypeTag::F32), 12 => Some(TypeTag::F64),
      13 => Some(TypeTag::ComplexNumber), 14 => Some(TypeTag::RationalNumber),
      15 => Some(TypeTag::String), 16 => Some(TypeTag::Bool),
      17 => Some(TypeTag::Id), 18 => Some(TypeTag::Index),
      19 => Some(TypeTag::Empty), 20 => Some(TypeTag::Any),
      21 => Some(TypeTag::Matrix), 22 => Some(TypeTag::EnumTag),
      23 => Some(TypeTag::Record), 24 => Some(TypeTag::Map),
      25 => Some(TypeTag::Atom), 26 => Some(TypeTag::Table),
      27 => Some(TypeTag::Tuple), _ => None,
    }
  }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TypeEntry {
  pub tag: TypeTag,
  pub bytes: Vec<u8>,
}
impl TypeEntry {
  pub fn byte_len(&self) -> u64 {
    2 + self.bytes.len() as u64
  }
}

pub type TypeId = u32;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct TypeSection {
  interner: HashMap<ValueKind, TypeId>,
  entries:  Vec<TypeEntry>, // index is TypeId
}
    
impl TypeSection {

  pub fn new() -> Self {
    Self { interner: HashMap::new(), entries: Vec::new() }
  }

  pub fn get_or_intern(&mut self, vk: &ValueKind) -> TypeId {
    if let Some(id) = self.interner.get(vk) { return *id; }
    // recursively intern children and build payload
    let (tag, mut bytes) = encode_value_kind(self, vk);
    let id = self.entries.len() as u32;
    self.entries.push(TypeEntry { tag, bytes });
    self.interner.insert(vk.clone(), id);
    id
  }

  pub fn write_to(&self, w: &mut impl std::io::Write) -> std::io::Result<()> {
    w.write_u32::<LittleEndian>(self.entries.len() as u32)?;
    for e in &self.entries {
      w.write_u16::<LittleEndian>(e.tag as u16)?;
      w.write_u16::<LittleEndian>(0)?;
      w.write_u32::<LittleEndian>(1)?;
      w.write_u32::<LittleEndian>(e.bytes.len() as u32)?;
      w.write_all(&e.bytes)?;
    }
    Ok(())
  }

  pub fn byte_len(&self) -> u64 {
    4 + self.entries.iter().map(|e| 12 + e.bytes.len() as u64).sum::<u64>()
  }
}

fn encode_value_kind(ts: &mut TypeSection, vk: &ValueKind) -> (TypeTag, Vec<u8>) {
  let mut b = Vec::new();
  let tag = match vk {
    ValueKind::U8 => TypeTag::U8, ValueKind::U16 => TypeTag::U16, ValueKind::U32 => TypeTag::U32,
    ValueKind::U64 => TypeTag::U64, ValueKind::U128 => TypeTag::U128,
    ValueKind::I8 => TypeTag::I8, ValueKind::I16 => TypeTag::I16, ValueKind::I32 => TypeTag::I32,
    ValueKind::I64 => TypeTag::I64, ValueKind::I128 => TypeTag::I128,
    ValueKind::F32 => TypeTag::F32, ValueKind::F64 => TypeTag::F64,
    ValueKind::ComplexNumber => TypeTag::ComplexNumber,
    ValueKind::RationalNumber => TypeTag::RationalNumber,
    ValueKind::String => TypeTag::String,
    ValueKind::Bool => TypeTag::Bool,
    ValueKind::Id => TypeTag::Id,
    ValueKind::Index => TypeTag::Index,
    ValueKind::Empty => TypeTag::Empty,
    ValueKind::Any => TypeTag::Any,

    ValueKind::Matrix(elem, dims) => {
      let elem_id = ts.get_or_intern(elem);
      b.write_u32::<LittleEndian>(elem_id).unwrap();
      b.write_u32::<LittleEndian>(dims.len() as u32).unwrap();
      for &d in dims { b.write_u32::<LittleEndian>(d as u32).unwrap(); }
      TypeTag::Matrix
    }

    ValueKind::Enum(space) => {
      b.write_u64::<LittleEndian>(*space).unwrap();
      TypeTag::EnumTag
    }

    ValueKind::Record(fields) => {
      b.write_u32::<LittleEndian>(fields.len() as u32).unwrap();
      for (name, ty) in fields {
        let name_bytes = name.as_bytes();
        b.write_u32::<LittleEndian>(name_bytes.len() as u32).unwrap();
        b.extend_from_slice(name_bytes);
        let tid = ts.get_or_intern(ty);
        b.write_u32::<LittleEndian>(tid).unwrap();
      }
      TypeTag::Record
    }

    ValueKind::Map(k,v) => {
      let kid = ts.get_or_intern(k);
      let vid = ts.get_or_intern(v);
      b.write_u32::<LittleEndian>(kid).unwrap();
      b.write_u32::<LittleEndian>(vid).unwrap();
      TypeTag::Map
    }

    ValueKind::Atom(id) => {
      b.write_u64::<LittleEndian>(*id).unwrap();
      TypeTag::Atom
    }

    ValueKind::Table(cols, pk_col) => {
      b.write_u32::<LittleEndian>(cols.len() as u32).unwrap();
      for (name, ty) in cols {
        let name_b = name.as_bytes();
        b.write_u32::<LittleEndian>(name_b.len() as u32).unwrap();
        b.extend_from_slice(name_b);
        let tid = ts.get_or_intern(ty);
        b.write_u32::<LittleEndian>(tid).unwrap();
      }
      b.write_u32::<LittleEndian>(*pk_col as u32).unwrap();
      TypeTag::Table
    }

    ValueKind::Tuple(elems) => {
      b.write_u32::<LittleEndian>(elems.len() as u32).unwrap();
      for t in elems {
        let tid = ts.get_or_intern(t);
        b.write_u32::<LittleEndian>(tid).unwrap();
      }
      TypeTag::Tuple
    }

    ValueKind::Reference(inner) => {
      let id = ts.get_or_intern(inner);
      b.write_u32::<LittleEndian>(id).unwrap();
      TypeTag::Reference
    }

    ValueKind::Set(elem, max) => {
      let id = ts.get_or_intern(elem);
      b.write_u32::<LittleEndian>(id).unwrap();
      match max {
        Some(m) => { b.push(1); use byteorder::WriteBytesExt; b.write_u32::<LittleEndian>(*m as u32).unwrap(); }
        None => { b.push(0); }
      }
      TypeTag::Set
    }

    ValueKind::Option(inner) => {
      let id = ts.get_or_intern(inner);
      b.write_u32::<LittleEndian>(id).unwrap();
      TypeTag::OptionT
    }
  };
  (tag, b)
}

// 4. Constants
// ----------------------------------------------------------------------------

#[repr(u8)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ConstEncoding { 
  Inline = 1 
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ConstEntry {
  pub type_id: u32,
  pub enc:     ConstEncoding,
  pub align:   u8,
  pub flags:   u8,
  pub reserved:u16,
  pub offset:  u64,
  pub length:  u64,
}

impl ConstEntry {
  pub fn write_to(&self, w: &mut impl std::io::Write) -> std::io::Result<()> {
    w.write_u32::<LittleEndian>(self.type_id)?;
    w.write_u8(self.enc as u8)?;
    w.write_u8(self.align)?;
    w.write_u8(self.flags)?;
    w.write_u8(0)?; // pad to 4 bytes for the small fields
    w.write_u64::<LittleEndian>(self.offset)?;
    w.write_u64::<LittleEndian>(self.length)?;
    Ok(())
  }
  pub fn byte_len() -> u64 { 4 + 1 + 1 + 1 + 1 + 8 + 8 } // = 24 bytes
}

pub trait CompileConst {
  fn compile_const(&self, ctx: &mut CompileCtx) -> MResult<u32>;
}

impl CompileConst for Value {

  fn compile_const(&self, ctx: &mut CompileCtx) -> MResult<u32> {
    let mut payload = Vec::<u8>::new();

    match self {
      #[cfg(feature = "bool")]
      Value::Bool(x) => payload.write_u8(if *x.borrow() { 1 } else { 0 })?,
      #[cfg(feature = "string")]
      Value::String(x) => {
        let string_brrw = x.borrow();
        let bytes = string_brrw.as_bytes();
        payload.write_u32::<LittleEndian>(bytes.len() as u32)?;
        payload.extend_from_slice(bytes);
      },
      #[cfg(feature = "u8")]
      Value::U8(x) => payload.write_u8(*x.borrow())?,
      #[cfg(feature = "u16")]
      Value::U16(x) => payload.write_u16::<LittleEndian>(*x.borrow())?,
      #[cfg(feature = "u32")]
      Value::U32(x) => payload.write_u32::<LittleEndian>(*x.borrow())?,
      #[cfg(feature = "u64")]
      Value::U64(x) => payload.write_u64::<LittleEndian>(*x.borrow())?,
      #[cfg(feature = "u128")]
      Value::U128(x) => payload.write_u128::<LittleEndian>(*x.borrow())?,
      #[cfg(feature = "i8")]
      Value::I8(x) => payload.write_i8(*x.borrow())?,
      #[cfg(feature = "i16")]
      Value::I16(x) => payload.write_i16::<LittleEndian>(*x.borrow())?,
      #[cfg(feature = "i32")]
      Value::I32(x) => payload.write_i32::<LittleEndian>(*x.borrow())?,
      #[cfg(feature = "i64")]
      Value::I64(x) => payload.write_i64::<LittleEndian>(*x.borrow())?,
      #[cfg(feature = "i128")]
      Value::I128(x) => payload.write_i128::<LittleEndian>(*x.borrow())?,
      #[cfg(feature = "f32")]
      Value::F32(x) => payload.write_f32::<LittleEndian>(x.borrow().0)?,
      #[cfg(feature = "f64")]
      Value::F64(x) => payload.write_f64::<LittleEndian>(x.borrow().0)?,
      #[cfg(feature = "atom")]
      Value::Atom(x) => payload.write_u64::<LittleEndian>(*x)?,
      #[cfg(feature = "index")]
      Value::Index(x) => payload.write_u64::<LittleEndian>(*x.borrow() as u64)?,
      #[cfg(feature = "complex")]
      Value::ComplexNumber(x) => {
        let c = x.borrow();
        payload.write_f64::<LittleEndian>(c.0.re)?;
        payload.write_f64::<LittleEndian>(c.0.im)?;
      },
      #[cfg(feature = "rational")]
      Value::RationalNumber(x) => {
        let r = x.borrow();
        payload.write_i64::<LittleEndian>(*r.numer())?;
        payload.write_i64::<LittleEndian>(*r.denom())?;
      },
      #[cfg(all(feature = "matrix", feature = "f64"))]
      Value::MatrixF64(x) => todo!(), //{return x.compile_const(ctx);}
      _ => todo!(),
    }
    ctx.compile_const(&payload, self.kind())
  }
}

// 5. Symbol Table
// ----------------------------------------------------------------------------

pub struct SymbolEntry {
  pub id: u64,          // unique identifier for the symbol
  pub reg: Register,    // register index this symbol maps to
}

impl SymbolEntry {

  pub fn new(id: u64, reg: Register) -> Self {
    Self { id, reg }
  }

  pub fn write_to(&self, w: &mut impl Write) -> io::Result<()> {
    w.write_u64::<LittleEndian>(self.id)?;
    w.write_u32::<LittleEndian>(self.reg)?;
    Ok(())
  }
}

// 6. Instruction Encoding (fixed forms)
// ----------------------------------------------------------------------------

pub const OP_CONST_LOAD: u64 = 0x01;
pub const OP_RETURN: u64 = 0xFF;

#[derive(Debug, Clone)]
pub enum EncodedInstr {
  ConstLoad { dst: u32, const_id: u32 },                   // [u64 opcode][u32 dst][u32 const_id]
  UnOp      { opcode: u64, dst: u32, src: u32 },           // [u64 opcode][u32 dst][u32 src]
  BinOp     { opcode: u64, dst: u32, lhs: u32, rhs: u32 }, // [u64 opcode][u32 dst][u32 lhs][u32 rhs]
  Ret       { src: u32 },                                  // [u64 opcode][u32 src]
}

impl EncodedInstr {
  pub fn byte_len(&self) -> u64 {
    match self {
      EncodedInstr::ConstLoad{..} => 8 + 4 + 4,
      EncodedInstr::UnOp{..}      => 8 + 4 + 4,
      EncodedInstr::BinOp{..}     => 8 + 4 + 4 + 4,
      EncodedInstr::Ret{..}       => 8 + 4,
    }
  }
  pub fn write_to(&self, w: &mut impl Write) -> io::Result<()> {
    match self {
      EncodedInstr::ConstLoad{ dst, const_id } => {
        w.write_u64::<LittleEndian>(OP_CONST_LOAD)?;
        w.write_u32::<LittleEndian>(*dst)?;
        w.write_u32::<LittleEndian>(*const_id)?;
      }
      EncodedInstr::UnOp{ opcode, dst, src } => {
        w.write_u64::<LittleEndian>(*opcode)?;
        w.write_u32::<LittleEndian>(*dst)?;
        w.write_u32::<LittleEndian>(*src)?;
      }
      EncodedInstr::BinOp{ opcode, dst, lhs, rhs } => {
        w.write_u64::<LittleEndian>(*opcode)?;
        w.write_u32::<LittleEndian>(*dst)?;
        w.write_u32::<LittleEndian>(*lhs)?;
        w.write_u32::<LittleEndian>(*rhs)?;
      }
      EncodedInstr::Ret{ src } => {
        w.write_u64::<LittleEndian>(OP_RETURN)?;
        w.write_u32::<LittleEndian>(*src)?;
      }
    }
    Ok(())
  }
}

// Load Program
// ----------------------------------------------------------------------------

pub fn load_program_from_file(path: impl AsRef<Path>) -> MResult<ParsedProgram> {
  let path = path.as_ref();
  let mut f = File::open(path)?;

  // total length for bounds checks
  let total_len = f.metadata()?.len();

  // Verify CRC trailer first; ensures fully readable, too.
  verify_crc_trailer_seek(&mut f, total_len)?;

  // Parse from the start
  f.seek(SeekFrom::Start(0))?;
  load_program_from_reader(&mut f, total_len)
}

pub fn load_program_from_bytes(bytes: &Vec<u8>) -> MResult<ParsedProgram> {
  let total_len = bytes.len() as u64;

  let mut cur = Cursor::new(bytes);
  verify_crc_trailer_seek(&mut cur, total_len)?;

  // Parse from the start
  cur.seek(SeekFrom::Start(0))?;
  load_program_from_reader(&mut cur, total_len)
}

fn load_program_from_reader<R: Read + Seek>(r: &mut R, total_len: u64) -> MResult<ParsedProgram> {
  r.seek(SeekFrom::Start(0))?;
  let mut header_buf = vec![0u8; ByteCodeHeader::HEADER_SIZE];
  r.read_exact(&mut header_buf)?;

  // 1) read header blob
  let mut header_cursor = Cursor::new(&header_buf[..]);
  let header = ByteCodeHeader::read_from(&mut header_cursor)?;

  // quick magic check
  if !header.validate_magic(&(*b"MECH")) {
    return Err(MechError {
      file: file!().to_string(),
      tokens: vec![],
      msg: format!("Invalid magic in bytecode header: expected 'MECH', got {:?}", header.magic),
      id: line!(),
      kind: MechErrorKind::GenericError("Invalid magic".to_string()),
    });
  }

  // 2. read features
  let mut features = Vec::new();
  if header.feature_off != 0 && header.feature_off + 4 <= total_len.saturating_sub(4) {
    r.seek(SeekFrom::Start(header.feature_off))?;
    let c = r.read_u32::<LittleEndian>()? as usize;
    for _ in 0..c {
      let v = r.read_u64::<LittleEndian>()?;
      features.push(v);
    }
  }

  // 3. read types
  let mut types = TypeSection::new();
  if header.types_off != 0 && header.types_off + 4 <= total_len.saturating_sub(4) {
    r.seek(SeekFrom::Start(header.types_off))?;
    let types_count = r.read_u32::<LittleEndian>()? as usize;
    for _ in 0..types_count {
      let tag = r.read_u16::<LittleEndian>()?;
      let _reserved = r.read_u16::<LittleEndian>()?; // reserved, always 0
      let _version = r.read_u32::<LittleEndian>()?; // version, always 1
      let bytes_len = r.read_u32::<LittleEndian>()? as usize;
      let mut bytes = vec![0u8; bytes_len];
      r.read_exact(&mut bytes)?;
      if let Some(tag) = TypeTag::from_u16(tag) {
        types.entries.push(TypeEntry { tag, bytes });
      } else {
        return Err(MechError {
          file: file!().to_string(),
          tokens: vec![],
          msg: format!("Unknown type tag: {}", tag),
          id: line!(),
          kind: MechErrorKind::GenericError("Unknown type tag".to_string()),
        });
      }
    }
  }

  // 4. read const table
  let mut const_entries = Vec::new();
  if header.const_tbl_off != 0 && header.const_tbl_len > 0 {
    r.seek(SeekFrom::Start(header.const_tbl_off))?;
    let mut tbl_bytes = vec![0u8; header.const_tbl_len as usize];
    r.read_exact(&mut tbl_bytes)?;
    let cur = Cursor::new(&tbl_bytes[..]);
    const_entries = parse_const_entries(cur, header.const_count as usize)?;
  }

  // read const blob
  let mut const_blob = vec![];
  if header.const_blob_off != 0 && header.const_blob_len > 0 {
    r.seek(SeekFrom::Start(header.const_blob_off))?;
    const_blob.resize(header.const_blob_len as usize, 0);
    r.read_exact(&mut const_blob)?;
  }

  // 5. read symbols
  let mut symbols = HashMap::new();
  if header.symbols_off != 0 && header.symbols_len > 0 {
    r.seek(SeekFrom::Start(header.symbols_off))?;
    let mut symbols_bytes = vec![0u8; header.symbols_len as usize];
    r.read_exact(&mut symbols_bytes)?;
    let mut cur = Cursor::new(&symbols_bytes[..]);
    for _ in 0..(header.symbols_len / 12) {
      let id = cur.read_u64::<LittleEndian>()?;
      let reg = cur.read_u32::<LittleEndian>()?;
      let entry = SymbolEntry::new(id, reg);
      symbols.insert(id, reg);
    }
  }

  // 6. read instr bytes
  let mut instr_bytes = vec![];
  if header.instr_off != 0 && header.instr_len > 0 {
    r.seek(SeekFrom::Start(header.instr_off))?;
    instr_bytes.resize(header.instr_len as usize, 0);
    r.read_exact(&mut instr_bytes)?;
  }

  // 7. read dictionary
  let mut dictionary = HashMap::new();
  if header.dict_off != 0 && header.dict_len > 0 {
    r.seek(SeekFrom::Start(header.dict_off))?;
    let mut dict_bytes = vec![0u8; header.dict_len as usize];
    r.read_exact(&mut dict_bytes)?;
    let mut cur = Cursor::new(&dict_bytes[..]);
    while cur.position() < dict_bytes.len() as u64 {
      let id = cur.read_u64::<LittleEndian>()?;
      let name_len = cur.read_u32::<LittleEndian>()? as usize;
      let mut name_bytes = vec![0u8; name_len];
      cur.read_exact(&mut name_bytes)?;
      let name = String::from_utf8(name_bytes).map_err(|_| MechError {
        file: file!().to_string(),
        tokens: vec![],
        msg: "Invalid UTF-8 in dictionary entry".to_string(),
        id: line!(),
        kind: MechErrorKind::GenericError("Invalid UTF-8".to_string()),
      })?;
      dictionary.insert(id, name);
    }
  }

  // decode instructions
  let instrs = decode_instructions(Cursor::new(&instr_bytes[..]))?;
  
  Ok(ParsedProgram { header, features, types, const_entries, const_blob, instr_bytes, symbols, instrs, dictionary })
}

pub fn parse_version_to_u16(s: &str) -> Option<u16> {
  let parts: Vec<&str> = s.split('.').collect();
  if parts.len() != 3 { return None; }

  let major = parts[0].parse::<u16>().ok()?;
  let minor = parts[1].parse::<u16>().ok()?;
  let patch = parts[2].parse::<u16>().ok()?; // parse to u16 to check bounds easily

  if major > 0b111 { return None; }    // 3 bits => 0..7
  if minor > 0b1_1111 { return None; } // 5 bits => 0..31
  if patch > 0xFF { return None; }     // 8 bits => 0..255

  // Pack: major in bits 15..13, minor in bits 12..8, patch in bits 7..0
  let encoded = (major << 13) | (minor << 8) | patch;
  Some(encoded as u16)
}

pub fn decode_version_from_u16(v: u16) -> (u16, u16, u16) {
  let major = (v >> 13) & 0b111;
  let minor = (v >> 8) & 0b1_1111;
  let patch = v & 0xFF;
  (major, minor, patch)
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ParsedConstEntry {
  pub type_id: u32,
  pub enc: u8,
  pub align: u8,
  pub flags: u8,
  pub reserved: u8,
  pub offset: u64,
  pub length: u64,
}

fn parse_const_entries(mut cur: Cursor<&[u8]>, count: usize) -> io::Result<Vec<ParsedConstEntry>> {
  let mut out = Vec::with_capacity(count);
  for _ in 0..count {
    let type_id = cur.read_u32::<LittleEndian>()?;
    let enc = cur.read_u8()?;
    let align = cur.read_u8()?;
    let flags = cur.read_u8()?;
    let reserved = cur.read_u8()?;
    let offset = cur.read_u64::<LittleEndian>()?;
    let length = cur.read_u64::<LittleEndian>()?;
    out.push(ParsedConstEntry { type_id, enc, align, flags, reserved, offset, length });
  }
  Ok(out)
}

pub fn verify_crc_trailer_seek<R: Read + Seek>(r: &mut R, total_len: u64) -> MResult<()> {
  if total_len < 4 {
    return Err(MechError {
      file: file!().to_string(),
      tokens: vec![],
      msg: "File too short to contain CRC trailer".to_string(),
      id: line!(),
      kind: MechErrorKind::GenericError("File too short".to_string()),
    });
  }

  // Read expected CRC from the last 4 bytes
  r.seek(SeekFrom::Start(total_len - 4))?;
  let expected_crc = r.read_u32::<LittleEndian>()?;

  // Compute CRC over the prefix (everything except the last 4 bytes).
  r.seek(SeekFrom::Start(0))?;
  let payload_len = (total_len - 4) as usize;
  let mut buf = vec![0u8; payload_len];
  r.read_exact(&mut buf)?;

  let file_crc = crc32fast::hash(&buf);
  if file_crc != expected_crc {
    Err(MechError {
      file: file!().to_string(),
      tokens: vec![],
      msg: format!("CRC mismatch: expected {}, got {}", expected_crc, file_crc),
      id: line!(),
      kind: MechErrorKind::GenericError("CRC mismatch".to_string()),
    })
  } else {
    Ok(())
  }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum DecodedInstr {
  ConstLoad { dst: u32, const_id: u32 },
  UnOp { opcode: u64, dst: u32, src: u32 },
  BinOp { opcode: u64, dst: u32, lhs: u32, rhs: u32 },
  Ret { src: u32 },
  Unknown { opcode: u64, rest: Vec<u8> }, // unknown opcode or dynamic form
}

fn decode_instructions(mut cur: Cursor<&[u8]>) -> io::Result<Vec<DecodedInstr>> {
  let mut out = Vec::new();
  while (cur.position() as usize) < cur.get_ref().len() {
    // read opcode (u64)
    let pos_before = cur.position();
    // if remaining < 8, can't read opcode
    let rem = cur.get_ref().len() - pos_before as usize;
    if rem < 8 {
      // leftover junk — treat as unknown and break
      let mut rest = vec![];
      let start = pos_before as usize;
      rest.extend_from_slice(&cur.get_ref()[start..]);
      out.push(DecodedInstr::Unknown { opcode: 0, rest });
      break;
    }
    let opcode = cur.read_u64::<LittleEndian>()?;
    match opcode {
      OP_CONST_LOAD => {
        // need 4+4 bytes
        let dst = cur.read_u32::<LittleEndian>()?;
        let const_id = cur.read_u32::<LittleEndian>()?;
        out.push(DecodedInstr::ConstLoad { dst, const_id });
      }
      OP_RETURN => {
        let src = cur.read_u32::<LittleEndian>()?;
        out.push(DecodedInstr::Ret { src });
      }
      unknown => {
        // Unknown opcode; we don't know how many args — try to be safe:
        // We'll return the rest of the bytes as `rest` and stop decoding.
        let start = cur.position() as usize;
        let rest = cur.get_ref()[start..].to_vec();
        out.push(DecodedInstr::Unknown { opcode: unknown, rest });
        break;
      }
    }
  }
  Ok(out)
}

// 7. Dictionary
// ----------------------------------------------------------------------------

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DictEntry {
  pub id: u64,          // unique identifier for the dictionary entry
  pub name: String,     // name of the entry
} 

impl DictEntry {
  pub fn new(id: u64, name: &str) -> Self {
    Self { id, name: name.to_string() }
  }

  pub fn write_to(&self, w: &mut impl Write) -> io::Result<()> {
    w.write_u64::<LittleEndian>(self.id)?;
    let name_bytes = self.name.as_bytes();
    w.write_u32::<LittleEndian>(name_bytes.len() as u32)?;
    w.write_all(name_bytes)?;
    Ok(())
  }
}

#[inline]
fn align_up(offset: u64, align: u64) -> u64 {
  if align == 0 { return offset; }
  ((offset + align - 1) / align) * align
}