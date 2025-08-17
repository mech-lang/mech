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