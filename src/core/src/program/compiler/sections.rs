use crate::*;
use super::*;

// Byetecode Compiler
// ============================================================================

// Format:
// 1. Header
// 2. Features
// 3. Types
// 4. Constants
// 5. Symbols
// 6. Instructions
// 7. Dictionary

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
  pub fn write_to(&self, w: &mut impl Write) -> MResult<()> {
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
  pub fn read_from(r: &mut impl Read) -> MResult<Self> {
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
  F32, F64, C64, R64,
  String, Bool, Atom,
  Set, Map, Table, Tuple, Record, Enum,
  VariableDefine, VariableAssign, KindDefine,
  KindAnnotation, SubscriptRange, SubscriptFormula,
  DotIndexing, Swizzle, 
  Matrix1, Matrix2, Matrix3, Matrix4,
  Matrix2x3, Matrix3x2,
  RowVector2, RowVector3, RowVector4,
  Vector2, Vector3, Vector4,
  VectorD, MatrixD, RowVectorD,
  HorzCat, VertCat,
  Compiler, PrettyPrint, Serde,
  MatMul, Transpose, Dot, Cross,
  Add, Sub, Mul, Div, Exp, Mod, Neg, OpAssign,
  LT, LTE, GT, GTE, EQ, NEQ,
  And, Or, Xor, Not,
  Convert, Assign, Access,
  Functions, Formulas,
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
      27 => Some(TypeTag::Tuple), 28 => Some(TypeTag::Reference),
      29 => Some(TypeTag::Set), 30 => Some(TypeTag::OptionT),
      _ => None,
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
  pub interner: HashMap<ValueKind, TypeId>,
  pub entries:  Vec<TypeEntry>, // index is TypeId
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

  pub fn write_to(&self, w: &mut impl Write) -> MResult<()> {
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
  pub fn write_to(&self, w: &mut impl Write) -> MResult<()> {
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

  pub fn write_to(&self, w: &mut impl Write) -> MResult<()> {
    w.write_u64::<LittleEndian>(self.id)?;
    w.write_u32::<LittleEndian>(self.reg)?;
    Ok(())
  }
}

// 6. Instruction Encoding (fixed forms)
// ----------------------------------------------------------------------------

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpCode {
  ConstLoad = 0x01,
  NullOp    = 0x10,
  Unop      = 0x20,
  Binop     = 0x30,
  Ternop    = 0x40,
  Quadop    = 0x50,
  VarArg    = 0x60,
  Return    = 0xFF,
}

impl OpCode {
  pub fn from_u8(num: u8) -> Option<OpCode> {
    match num {
      0x01 => Some(OpCode::ConstLoad),
      0x10 => Some(OpCode::NullOp),
      0x20 => Some(OpCode::Unop),
      0x30 => Some(OpCode::Binop),
      0x40 => Some(OpCode::Ternop),
      0x50 => Some(OpCode::Quadop),
      0x60 => Some(OpCode::VarArg),
      0xFF => Some(OpCode::Return),
      _    => None,
    }
  }
}

#[derive(Debug, Clone)]
pub enum EncodedInstr {
  ConstLoad { dst: u32, const_id: u32 },                               // [u64 opcode][u32 dst][u32 const_id]
  NullOp    { fxn_id: u64, dst: u32 },                                 // [u64 opcode][u64 fxn_id][u32 dst]
  UnOp      { fxn_id: u64, dst: u32, src: u32 },                       // [u64 opcode][u32 dst][u32 src]
  BinOp     { fxn_id: u64, dst: u32, lhs: u32, rhs: u32 },             // [u64 opcode][u32 dst][u32 lhs][u32 rhs]
  TernOp    { fxn_id: u64, dst: u32, a: u32, b: u32, c: u32 },         // [u64 opcode][u32 dst][u32 a][u32 b][u32 c]
  QuadOp    { fxn_id: u64, dst: u32, a: u32, b: u32, c: u32, d: u32 }, // [u64 opcode][u32 dst][u32 a][u32 b][u32 c][u32 d]
  VarArg    { fxn_id: u64, dst: u32, args: Vec<u32> },                 // [u64 opcode][u64 fxn_id][u32 dst][u32 arg_count][u32 args...]
  Ret       { src: u32 },                                              // [u64 opcode][u32 src]
}

impl EncodedInstr {
  pub fn byte_len(&self) -> u64 {
    match self {
      EncodedInstr::ConstLoad{..} => 1 + 4 + 4,
      EncodedInstr::NullOp{..}    => 1 + 8 + 4,
      EncodedInstr::UnOp{..}      => 1 + 8 + 4 + 4,
      EncodedInstr::BinOp{..}     => 1 + 8 + 4 + 4 + 4,
      EncodedInstr::TernOp{..}    => 1 + 8 + 4 + 4 + 4 + 4,
      EncodedInstr::QuadOp{..}    => 1 + 8 + 4 + 4 + 4 + 4 + 4,
      EncodedInstr::VarArg{ args, .. } => 1 + 8 + 4 + 4 + (4 * args.len() as u64),
      EncodedInstr::Ret{..}       => 1 + 4,
    }
  }
  pub fn write_to(&self, w: &mut impl Write) -> MResult<()> {
    match self {
      EncodedInstr::ConstLoad{ dst, const_id } => {
        w.write_u8(OpCode::ConstLoad as u8)?;
        w.write_u32::<LittleEndian>(*dst)?;
        w.write_u32::<LittleEndian>(*const_id)?;
      }
      EncodedInstr::NullOp{ fxn_id, dst } => {
        w.write_u8(OpCode::NullOp as u8)?;
        w.write_u64::<LittleEndian>(*fxn_id)?;
        w.write_u32::<LittleEndian>(*dst)?;
      }
      EncodedInstr::UnOp{ fxn_id, dst, src } => {
        w.write_u8(OpCode::Unop as u8)?;
        w.write_u64::<LittleEndian>(*fxn_id)?;
        w.write_u32::<LittleEndian>(*dst)?;
        w.write_u32::<LittleEndian>(*src)?;
      }
      EncodedInstr::BinOp{ fxn_id, dst, lhs, rhs } => {
        w.write_u8(OpCode::Binop as u8)?;
        w.write_u64::<LittleEndian>(*fxn_id)?;
        w.write_u32::<LittleEndian>(*dst)?;
        w.write_u32::<LittleEndian>(*lhs)?;
        w.write_u32::<LittleEndian>(*rhs)?;
      }
      EncodedInstr::TernOp{ fxn_id, dst, a, b, c } => {
        w.write_u8(OpCode::Ternop as u8)?;
        w.write_u64::<LittleEndian>(*fxn_id)?;
        w.write_u32::<LittleEndian>(*dst)?;
        w.write_u32::<LittleEndian>(*a)?;
        w.write_u32::<LittleEndian>(*b)?;
        w.write_u32::<LittleEndian>(*c)?;
      }
      EncodedInstr::QuadOp{ fxn_id, dst, a, b, c, d } => {
        w.write_u8(OpCode::Quadop as u8)?;
        w.write_u64::<LittleEndian>(*fxn_id)?;
        w.write_u32::<LittleEndian>(*dst)?;
        w.write_u32::<LittleEndian>(*a)?;
        w.write_u32::<LittleEndian>(*b)?;
        w.write_u32::<LittleEndian>(*c)?;
        w.write_u32::<LittleEndian>(*d)?;
      }
      EncodedInstr::VarArg{ fxn_id, dst, args } => {
        w.write_u8(OpCode::VarArg as u8)?;
        w.write_u64::<LittleEndian>(*fxn_id)?;
        w.write_u32::<LittleEndian>(*dst)?;
        w.write_u32::<LittleEndian>(args.len() as u32)?;
        for a in args {
          w.write_u32::<LittleEndian>(*a)?;
        }
      }
      EncodedInstr::Ret{ src } => {
        w.write_u8(OpCode::Return as u8)?;
        w.write_u32::<LittleEndian>(*src)?;
      }
    }
    Ok(())
  }
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

  pub fn write_to(&self, w: &mut impl Write) -> MResult<()> {
    w.write_u64::<LittleEndian>(self.id)?;
    let name_bytes = self.name.as_bytes();
    w.write_u32::<LittleEndian>(name_bytes.len() as u32)?;
    w.write_all(name_bytes)?;
    Ok(())
  }
}


