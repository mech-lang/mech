use crate::*;
use byteorder::{LittleEndian, WriteBytesExt, ReadBytesExt};
use std::io::{self, Write, Read};
use std::collections::HashMap;

// 2. Type Section
// ----------------------------------------------------------------------------

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

pub struct CompileCtx {
  // pointer identity -> register index
  pub reg_map: HashMap<usize, u32>,
  pub next_reg: u32,

  // types
  pub types: TypeSection,

  // constants
  pub const_entries: Vec<ConstEntry>,
  pub const_blob: Vec<u8>,

  // instructions
  pub instrs: Vec<EncodedInstr>,

  // features
  pub features: Vec<u64>,
}

impl CompileCtx {
  pub fn new() -> Self {
    Self {
      reg_map: HashMap::new(),
      next_reg: 0,
      types: TypeSection::new(),
      const_entries: Vec::new(),
      const_blob: Vec::new(),
      instrs: Vec::new(),
      features: Vec::new(),
    }
  }

  pub fn alloc_register_for_ptr(&mut self, ptr: usize) -> u32 {
    if let Some(&r) = self.reg_map.get(&ptr) { return r; }
    let r = self.next_reg;
    self.next_reg += 1;
    self.reg_map.insert(ptr, r);
    r
  }

  pub fn ensure_feature(&mut self, fid: u64) {
    if !self.features.contains(&fid) { self.features.push(fid); }
  }

  /// add a constant (value) -> returns const_id (u32)
  pub fn add_constant(&mut self, val: &Value, vk: &ValueKind) -> u32 {
    // compute type id
    let t = self.types.get_or_intern(vk);

    // offset = end of blob
    let off = self.const_blob.len() as u64;

    /*
    match val {
      Value::F64(x) => {
        self.const_blob.write_f64::<LittleEndian>(*x).unwrap();
      }
      Value::MatrixF64 { dims: _, data_row_major } => {
        for v in data_row_major {
          self.const_blob.write_f64::<LittleEndian>(*v).unwrap();
        }
      }
    }*/
    let len = (self.const_blob.len() as u64) - off;

    let entry = ConstEntry {
      type_id: t,
      enc: ConstEncoding::Inline,
      align: 8,
      flags: 0,
      reserved: 0,
      offset: off,
      length: len,
    };
    let id = self.const_entries.len() as u32;
    self.const_entries.push(entry);
    id
  }

  pub fn emit_const_load(&mut self, dst: u32, const_id: u32) {
    self.instrs.push(EncodedInstr::ConstLoad { dst, const_id });
  }
  pub fn emit_unop(&mut self, opcode: u64, dst: u32, src: u32) {
    self.instrs.push(EncodedInstr::UnOp { opcode, dst, src });
  }
  pub fn emit_binop(&mut self, opcode: u64, dst: u32, lhs: u32, rhs: u32) {
    self.instrs.push(EncodedInstr::BinOp { opcode, dst, lhs, rhs });
  }
  pub fn emit_ret(&mut self, src: u32) {
    self.instrs.push(EncodedInstr::Ret { src })
  }
}

// Instruction encoding (fixed forms)
// ----------------------------------------------------------------------------

pub const OP_CONST_LOAD: u64 = 0x01;
pub const OP_RETURN: u64 = 0xFF;

#[derive(Debug, Clone)]
pub enum EncodedInstr {
  ConstLoad { dst: u32, const_id: u32 },           // [u64 opcode][u32 dst][u32 const_id]
  UnOp      { opcode: u64, dst: u32, src: u32 },  // [u64 opcode][u32 dst][u32 src]
  BinOp     { opcode: u64, dst: u32, lhs: u32, rhs: u32 }, // [u64 opcode][u32 dst][u32 lhs][u32 rhs]
  Ret       { src: u32 },                          // [u64 opcode][u32 src]
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


#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstEncoding { Inline = 1 }

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