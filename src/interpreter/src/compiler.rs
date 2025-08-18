use crate::*;
use byteorder::{LittleEndian, WriteBytesExt, ReadBytesExt};
use std::io::{self, Write, Read};

// Byetecode Compiler
// ============================================================================

// Format:
// 1. Header
// 2. Type Section
// 3. Feature Section
// 4. Constant Table
// 5. Constant Blob
// 6. Instruction Stream

// 1. Header
// ----------------------------------------------------------------------------

#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ByteCodeHeader {
  pub magic:        [u8; 4],       // e.g., b"MECH"
  pub version:        u8,          // bytecode format version
  pub mech_ver:       u16,         // Mech language version
  pub flags:          u16,         // reserved/feature bit
  pub reg_count:      u32,         // total virtual registers used
  pub instr_count:    u32,         // number of instructions
  pub feature_count:  u32,         // number of feature flags  
  pub feature_off:    u64,         // offset to feature flags (array of u64)

  pub const_count:    u32,         // number of constants (entries
  pub const_tbl_off:  u64,         // offset to constant table (array of entries)
  pub const_tbl_len:  u64,         // bytes in constant table area (entries only)
  pub const_blob_off: u64,         // offset to raw constant blob data
  pub const_blob_len: u64,         // bytes in blob (payloads
                               
  pub instr_off:      u64,         // offset to instruction stream
  pub instr_len:      u64,         // bytes of instruction stream
            
  pub feat_off:       u64,         // offset to feature section
  pub feat_len:       u64,         // bytes of feature section

  pub checksum:       u32,         // optional (CRC32/xxHash), or 0 if unused
  pub reserved:       u32,         // pad/alignment
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

