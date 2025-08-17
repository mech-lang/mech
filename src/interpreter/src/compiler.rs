// 2. Constant Pool
// For each constant:
//   Type: u8
//   Data: variable length depending on type
// 3. Instructions
// For each instruction:
//   Opcode: u8
//   Operands: variable length depending on opcode
// 4. Footer
// Checksum: u32 (simple sum of all previous bytes modulo 2^32)
// End marker: 0x45 4e 44 21 (END!)

use crate::*;
use byteorder::{LittleEndian, WriteBytesExt, ReadBytesExt};
use std::io::{self, Write, Read};

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
  Custom = 0xFFFF,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeatureFlag {
  Builtin(FeatureKind),
  Custom(u64),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeatureSection {
  pub count: u32,
  pub entries: Vec<FeatureFlag>,
}

#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ByteCodeHeader {
  magic:        [u8; 4],       // e.g., b"MECH"
  version:        u8,          // bytecode format version
  mech_ver:       u16,         // Mech language version
  flags:          u16,         // reserved/feature bit
  reg_count:      u32,         // total virtual registers used
  instr_count:    u32,         // number of instructions
  feature_count:  u32,         // number of feature flags  
  feature_off:    u64,         // offset to feature flags (array of u64)

  const_count:    u32,         // number of constants (entries
  const_tbl_off:  u64,         // offset to constant table (array of entries)
  const_tbl_len:  u64,         // bytes in constant table area (entries only)
  const_blob_off: u64,         // offset to raw constant blob data
  const_blob_len: u64,         // bytes in blob (payloads
                               
  instr_off:      u64,         // offset to instruction stream
  instr_len:      u64,         // bytes of instruction stream
            
  feat_off:       u64,         // offset to feature section
  feat_len:       u64,         // bytes of feature section

  checksum:       u32,         // optional (CRC32/xxHash), or 0 if unused
  reserved:       u32,         // pad/alignment
}

impl ByteCodeHeader {
  /// Header byte size when serialized. This is the number of bytes `write_to` will write.
  /// (Computed from the sum of sizes of each field written in little-endian.)
  pub const HEADER_SIZE: usize = 4  // magic
    + 1   // version
    + 2   // mech_ver
    + 2   // flags
    + 4   // reg_count
    + 4   // instr_count
    + 4   // feature_count
    + 8   // feature_off
    + 4   // const_count
    + 8   // const_tbl_off
    + 8   // const_tbl_len
    + 8   // const_blob_off
    + 8   // const_blob_len
    + 8   // instr_off
    + 8   // instr_len
    + 8   // feat_off
    + 8   // feat_len
    + 4   // checksum
    + 4;  // reserved

  /// Serialize header into `w` using little-endian encoding.
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

    // constants table / blob
    w.write_u32::<LittleEndian>(self.const_count)?;
    w.write_u64::<LittleEndian>(self.const_tbl_off)?;
    w.write_u64::<LittleEndian>(self.const_tbl_len)?;
    w.write_u64::<LittleEndian>(self.const_blob_off)?;
    w.write_u64::<LittleEndian>(self.const_blob_len)?;

    // instructions
    w.write_u64::<LittleEndian>(self.instr_off)?;
    w.write_u64::<LittleEndian>(self.instr_len)?;

    // feature section (alternative/extra)
    w.write_u64::<LittleEndian>(self.feat_off)?;
    w.write_u64::<LittleEndian>(self.feat_len)?;

    // footer
    w.write_u32::<LittleEndian>(self.checksum)?;
    w.write_u32::<LittleEndian>(self.reserved)?;
    Ok(())
  }

  /// Read a header from `r`. Expects the same layout as `write_to`.
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

    let const_count = r.read_u32::<LittleEndian>()?;
    let const_tbl_off = r.read_u64::<LittleEndian>()?;
    let const_tbl_len = r.read_u64::<LittleEndian>()?;
    let const_blob_off = r.read_u64::<LittleEndian>()?;
    let const_blob_len = r.read_u64::<LittleEndian>()?;

    let instr_off = r.read_u64::<LittleEndian>()?;
    let instr_len = r.read_u64::<LittleEndian>()?;

    let feat_off = r.read_u64::<LittleEndian>()?;
    let feat_len = r.read_u64::<LittleEndian>()?;

    let checksum = r.read_u32::<LittleEndian>()?;
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
      const_count,
      const_tbl_off,
      const_tbl_len,
      const_blob_off,
      const_blob_len,
      instr_off,
      instr_len,
      feat_off,
      feat_len,
      checksum,
      reserved,
    })
  }

  /// Quick check: does the header magic match the expected magic?
  pub fn validate_magic(&self, expected: &[u8;4]) -> bool {
    &self.magic == expected
  }
}


#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TypeTag {
  U8=1, U16, U32, U64, U128, I8, I16, I32, I64, I128,
  F32, F64, ComplexNumber, RationalNumber, String, Bool, Id, Index, Empty, Any,
  Matrix, EnumTag, Record, Map, Atom, Table, Tuple, Reference, Set, OptionT,
}

#[derive(Debug)]
pub struct TypeEntry {
  pub tag: TypeTag,
  pub bytes: Vec<u8>, // encoded payload per rules above
}
pub type TypeId = u32;

#[derive(Default)]
pub struct TypeSection {
  // canonicalize by structural equality of ValueKind
  interner: std::collections::HashMap<ValueKind, TypeId>,
  entries:  Vec<TypeEntry>, // index is TypeId
}
    

impl TypeSection {
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
    use byteorder::{LittleEndian, WriteBytesExt};
    w.write_u32::<LittleEndian>(self.entries.len() as u32)?;
    for e in &self.entries {
      w.write_u16::<LittleEndian>(e.tag as u16)?;
      w.write_u16::<LittleEndian>(0)?; // flags
      w.write_u32::<LittleEndian>(1)?; // aux_count (not strictly used; kept for forward compatibility)
      w.write_u32::<LittleEndian>(e.bytes.len() as u32)?;
      w.write_all(&e.bytes)?;
    }
    Ok(())
  }

  pub fn byte_len(&self) -> u64 {
    // 4 + sum( tag(2)+flags(2)+aux(4)+len(4)+bytes )
    4 + self.entries.iter().map(|e| 12 + e.bytes.len() as u64).sum::<u64>()
  }
}

fn encode_value_kind(ts: &mut TypeSection, vk: &ValueKind) -> (TypeTag, Vec<u8>) {
  use byteorder::{LittleEndian, WriteBytesExt};
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

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstEncoding { Inline=1 }

#[derive(Debug, Clone, PartialEq, Eq)]
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
