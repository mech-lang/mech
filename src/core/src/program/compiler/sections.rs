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

  F32, F64, C64, R64, Index,
  String, Bool, Atom, Set, Map, 
  
  Table, Tuple, Record, Enum,
  VariableDefine, VariableAssign, KindDefine,
  KindAnnotation, SubscriptRange, SubscriptFormula,
  
  RangeInclusive, RangeExclusive,
  DotIndexing, Swizzle, LogicalIndexing,
  Matrix1, Matrix2, Matrix3, Matrix4, Matrix2x3, 
  
  Matrix3x2, RowVector2, RowVector3, RowVector4,
  Vector2, Vector3, Vector4, VectorD, MatrixD, RowVectorD,
  
  HorzCat, VertCat,
  Compiler, PrettyPrint, Serde,
  MatMul, Transpose, Dot, Cross, Add, 
  
  Sub, Mul, Div, Exp, Mod, 
  Neg, OpAssign, LT, LTE, GT, 
  
  GTE, EQ, NEQ, And, Or, 
  Xor, Not, Convert, Assign, Access,

  Union, Intersection, Difference, Complement, Subset, 
  Superset, ProperSubset, ProperSuperset, ElementOf, NotElementOf,

  Functions, Formulas,
  Custom = 0xFFFF,
}

impl FeatureKind {

  pub fn as_string(&self) -> String {
    match self {
      FeatureKind::I8 => "i8".to_string(),
      FeatureKind::I16 => "i16".to_string(),
      FeatureKind::I32 => "i32".to_string(),
      FeatureKind::I64 => "i64".to_string(),
      FeatureKind::I128 => "i128".to_string(),
      FeatureKind::U8 => "u8".to_string(),
      FeatureKind::U16 => "u16".to_string(),
      FeatureKind::U32 => "u32".to_string(),
      FeatureKind::U64 => "u64".to_string(),
      FeatureKind::U128 => "u128".to_string(),
      FeatureKind::F32 => "f32".to_string(),
      FeatureKind::F64 => "f64".to_string(),
      FeatureKind::C64 => "c64".to_string(),
      FeatureKind::R64 => "r64".to_string(),
      FeatureKind::Index => "index".to_string(),
      FeatureKind::String => "string".to_string(),
      FeatureKind::Bool => "bool".to_string(),
      FeatureKind::Atom => "atom".to_string(),
      FeatureKind::Set => "set".to_string(),
      FeatureKind::Map => "map".to_string(),
      FeatureKind::Table => "table".to_string(),
      FeatureKind::Tuple => "tuple".to_string(),
      FeatureKind::Record => "record".to_string(),
      FeatureKind::Enum => "enum".to_string(),
      FeatureKind::VariableDefine => "variable_define".to_string(),
      FeatureKind::VariableAssign => "variable_assign".to_string(),
      FeatureKind::KindDefine => "kind_define".to_string(),
      FeatureKind::KindAnnotation => "kind_annotation".to_string(),
      FeatureKind::SubscriptRange => "subscript_range".to_string(),
      FeatureKind::SubscriptFormula => "subscript_formula".to_string(),
      FeatureKind::RangeInclusive => "range_inclusive".to_string(),
      FeatureKind::RangeExclusive => "range_exclusive".to_string(),
      FeatureKind::DotIndexing => "dot_indexing".to_string(),
      FeatureKind::Swizzle => "swizzle".to_string(),
      FeatureKind::LogicalIndexing => "logical_indexing".to_string(),
      FeatureKind::Matrix1 => "matrix1".to_string(),
      FeatureKind::Matrix2 => "matrix2".to_string(),
      FeatureKind::Matrix3 => "matrix3".to_string(),
      FeatureKind::Matrix4 => "matrix4".to_string(),
      FeatureKind::Matrix2x3 => "matrix2x3".to_string(),
      FeatureKind::Matrix3x2 => "matrix3x2".to_string(),
      FeatureKind::RowVector2 => "row_vector2".to_string(),
      FeatureKind::RowVector3 => "row_vector3".to_string(),
      FeatureKind::RowVector4 => "row_vector4".to_string(),
      FeatureKind::Vector2 => "vector2".to_string(),
      FeatureKind::Vector3 => "vector3".to_string(),
      FeatureKind::Vector4 => "vector4".to_string(),
      FeatureKind::VectorD => "vectord".to_string(),
      FeatureKind::MatrixD => "matrixd".to_string(),
      FeatureKind::RowVectorD => "row_vectord".to_string(),
      FeatureKind::HorzCat => "matrix_horzcat".to_string(),
      FeatureKind::VertCat => "matrix_vertcat".to_string(),
      FeatureKind::Compiler => "compiler".to_string(),
      FeatureKind::PrettyPrint => "pretty_print".to_string(),
      FeatureKind::Serde => "serde".to_string(),
      FeatureKind::MatMul => "matrix_matmul".to_string(),
      FeatureKind::Transpose => "matrix_transpose".to_string(),
      FeatureKind::Dot => "matrix_dot".to_string(),
      FeatureKind::Cross => "matrix_cross".to_string(),
      FeatureKind::Add => "math_add".to_string(),
      FeatureKind::Sub => "math_sub".to_string(),
      FeatureKind::Mul => "math_mul".to_string(),
      FeatureKind::Div => "math_div".to_string(),
      FeatureKind::Exp => "math_exp".to_string(),
      FeatureKind::Mod => "math_mod".to_string(),
      FeatureKind::Neg => "math_neg".to_string(),
      FeatureKind::OpAssign => "math_opassign".to_string(),
      FeatureKind::LT => "compare_lt".to_string(),
      FeatureKind::LTE => "compare_lte".to_string(),
      FeatureKind::GT => "compare_gt".to_string(),
      FeatureKind::GTE => "compare_gte".to_string(),
      FeatureKind::EQ => "compare_eq".to_string(),
      FeatureKind::NEQ => "compare_neq".to_string(),
      FeatureKind::And => "logic_and".to_string(),
      FeatureKind::Or => "logic_or".to_string(),
      FeatureKind::Xor => "logic_xor".to_string(),
      FeatureKind::Not => "logic_not".to_string(),
      FeatureKind::Convert => "convert".to_string(),
      FeatureKind::Assign => "assign".to_string(),
      FeatureKind::Access => "access".to_string(),
      FeatureKind::Union => "set_union".to_string(),
      FeatureKind::Intersection => "set_intersection".to_string(),
      FeatureKind::Difference => "set_difference".to_string(),
      FeatureKind::Complement => "set_complement".to_string(),
      FeatureKind::Subset => "set_subset".to_string(),
      FeatureKind::Superset => "set_superset".to_string(),
      FeatureKind::ProperSubset => "set_proper_subset".to_string(),
      FeatureKind::ProperSuperset => "set_proper_superset".to_string(),
      FeatureKind::ElementOf => "set_element_of".to_string(),
      FeatureKind::NotElementOf => "set_not_element_of".to_string(),
      FeatureKind::Functions => "functions".to_string(),
      FeatureKind::Formulas => "formulas".to_string(),
      FeatureKind::Custom => "custom".to_string(),
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FeatureFlag {
  Builtin(FeatureKind),
  Custom(u64),
}

impl FeatureFlag {

  pub fn as_string(&self) -> String {
    match self {
      FeatureFlag::Builtin(f) => f.as_string(),
      FeatureFlag::Custom(c) => format!("custom({})", c),
    }
  }
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
  F32, F64, C64, R64, String, Bool, Id, Index, Empty, Any,
  MatrixU8, MatrixU16, MatrixU32, MatrixU64, MatrixU128,
  MatrixI8, MatrixI16, MatrixI32, MatrixI64, MatrixI128,
  MatrixF32, MatrixF64, MatrixC64, MatrixR64, MatrixBool, 
  MatrixString, MatrixIndex,
  EnumTag, Record, Map, Atom, 
  Table, Tuple, Reference, Set, OptionT,
}

impl TypeTag {
  pub fn from_u16(tag: u16) -> Option<Self> {
    match tag {
      1 => Some(TypeTag::U8), 2 => Some(TypeTag::U16), 3 => Some(TypeTag::U32), 4 => Some(TypeTag::U64), 5 => Some(TypeTag::U128),
      6 => Some(TypeTag::I8), 7 => Some(TypeTag::I16), 8 => Some(TypeTag::I32), 9 => Some(TypeTag::I64), 10 => Some(TypeTag::I128),
      11 => Some(TypeTag::F32), 12 => Some(TypeTag::F64), 13 => Some(TypeTag::C64), 14 => Some(TypeTag::R64),
      15 => Some(TypeTag::String), 16 => Some(TypeTag::Bool), 17 => Some(TypeTag::Id), 18 => Some(TypeTag::Index), 19 => Some(TypeTag::Empty), 20 => Some(TypeTag::Any),
      21 => Some(TypeTag::MatrixU8), 22 => Some(TypeTag::MatrixU16), 23 => Some(TypeTag::MatrixU32), 24 => Some(TypeTag::MatrixU64), 25 => Some(TypeTag::MatrixU128),
      26 => Some(TypeTag::MatrixI8), 27 => Some(TypeTag::MatrixI16), 28 => Some(TypeTag::MatrixI32), 29 => Some(TypeTag::MatrixI64), 30 => Some(TypeTag::MatrixI128),
      31 => Some(TypeTag::MatrixF32), 32 => Some(TypeTag::MatrixF64), 33 => Some(TypeTag::MatrixC64), 34 => Some(TypeTag::MatrixR64), 35 => Some(TypeTag::MatrixBool), 
      36 => Some(TypeTag::MatrixString), 37 => Some(TypeTag::MatrixIndex),
      38 => Some(TypeTag::EnumTag), 39 => Some(TypeTag::Record), 40 => Some(TypeTag::Map), 41 => Some(TypeTag::Atom), 
      42 => Some(TypeTag::Table), 43 => Some(TypeTag::Tuple), 44 => Some(TypeTag::Reference), 45 => Some(TypeTag::Set), 46 => Some(TypeTag::OptionT),
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
  pub mutable: bool,
  pub reg: Register,    // register index this symbol maps to
}

impl SymbolEntry {

  pub fn new(id: u64, mutable: bool, reg: Register) -> Self {
    Self { id, mutable, reg }
  }

  pub fn write_to(&self, w: &mut impl Write) -> MResult<()> {
    w.write_u64::<LittleEndian>(self.id)?;
    w.write_u8(if self.mutable { 1 } else { 0 })?;
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

impl std::fmt::Display for OpCode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let s = match self {
      OpCode::ConstLoad => "ConstLoad",
      OpCode::NullOp    => "NullOp",
      OpCode::Unop      => "Unop",
      OpCode::Binop     => "Binop",
      OpCode::Ternop    => "Ternop",
      OpCode::Quadop    => "Quadop",
      OpCode::VarArg    => "VarArg",
      OpCode::Return    => "Return",
    };
    write!(f, "{}", s)
  }
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


