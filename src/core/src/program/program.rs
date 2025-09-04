use crate::*;
use super::*;
use byteorder::{LittleEndian, WriteBytesExt, ReadBytesExt};
use std::io::Write;
use std::io::{self, SeekFrom, Seek, Cursor};
#[cfg(not(feature = "no_std"))]
use std::fs::File;
#[cfg(not(feature = "no_std"))]
use std::path::Path;
#[cfg(feature = "matrix")]
use crate::matrix::Matrix;

 macro_rules! extract_matrix {
  ($type_tag:ident, $value_type:ty, $bytes:expr, $data:ident) => {
    {
      // first 8 bytes are rows (u32) and cols (u32)
      if $data.len() < 8 {
        return Err(MechError {
          file: file!().to_string(),
          tokens: vec![],
          msg: "Matrix const entry must be at least 8 bytes".to_string(),
          id: line!(),
          kind: MechErrorKind::GenericError("Matrix const entry must be at least 8 bytes".to_string()),
        });
      }
      let rows = u32::from_le_bytes($data[0..4].try_into().unwrap()) as usize;
      let cols = u32::from_le_bytes($data[4..8].try_into().unwrap()) as usize;
      let mut elements = Vec::with_capacity(rows * cols);
      for i in 0..(rows * cols) {
        let start = 8 + i * $bytes;
        let end = start + $bytes;
        let val = <$value_type>::from_le_bytes($data[start..end].try_into().unwrap());
        elements.push(val);
      }
      Value::$type_tag(Matrix::from_vec(elements, rows, cols))
    }
  };
}

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

 pub fn to_bytes(&self) -> MResult<Vec<u8>> {
    let mut buf = Cursor::new(Vec::<u8>::new());

    // 1. Header
    self.header.write_to(&mut buf)?;

    // 2. Features
    buf.write_u32::<LittleEndian>(self.features.len() as u32)?;
    for f in &self.features {
      buf.write_u64::<LittleEndian>(*f)?;
    }

    // 3. Types
    self.types.write_to(&mut buf)?;

    // 4. Const entries
    for entry in &self.const_entries {
      entry.write_to(&mut buf)?;
    }

    // 5. Const blob
    if !self.const_blob.is_empty() {
      buf.write_all(&self.const_blob)?;
    }

    // 6. Symbols
    for (id, reg) in &self.symbols {
      let entry = SymbolEntry::new(*id, *reg);
      entry.write_to(&mut buf)?;
    }

    // 7. Instructions
    for ins in &self.instrs {
      ins.write_to(&mut buf)?;
    }

    // 8. Dictionary
    for (id, name) in &self.dictionary {
      let dict_entry = DictEntry::new(*id, name);
      dict_entry.write_to(&mut buf)?;
    }

    // 9. CRC32 trailer
    let bytes_so_far = buf.get_ref().as_slice();
    let checksum = crc32fast::hash(bytes_so_far);
    buf.write_u32::<LittleEndian>(checksum)?;

    Ok(buf.into_inner())
  }

  pub fn from_bytes(bytes: &[u8]) -> MResult<ParsedProgram> {
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

    for const_entry in &self.const_entries {
      // Only support Inline encoding for now
      if const_entry.enc != ConstEncoding::Inline as u8 {
        return Err(MechError {file: file!().to_string(),tokens: vec![],msg: "Unsupported constant encoding".to_string(),id: line!(),kind: MechErrorKind::GenericError("Unsupported constant encoding".to_string())});
      }

      // Bounds check
      if const_entry.offset.checked_add(const_entry.length).is_none() {
          return Err(MechError {file: file!().to_string(),tokens: vec![],msg: "Constant entry out of bounds".to_string(),id: line!(),kind: MechErrorKind::GenericError("Constant entry out of bounds".to_string())});
      }
      let end = const_entry.offset + const_entry.length;
      if end > blob_len {
        return Err(MechError {file: file!().to_string(),tokens: vec![],msg: "Constant entry out of bounds".to_string(),id: line!(),kind: MechErrorKind::GenericError("Constant entry out of bounds".to_string())});
      }

      // Alignment check (if your alignment semantics differ, change this)
      if !check_alignment(const_entry.offset, const_entry.align) {
        return Err(MechError {file: file!().to_string(),tokens: vec![],msg: "Constant entry alignment error".to_string(),id: line!(),kind: MechErrorKind::GenericError("Constant entry alignment error".to_string())});
      }

      // Copy bytes out (we clone into Vec<u8> to own data)
      let start = const_entry.offset as usize;
      let len = const_entry.length as usize;
      let data = self.const_blob[start .. start + len].to_vec();

      // get the type from the id
      let ty = &self.types.entries[const_entry.type_id as usize];

      let val: Value = match ty.tag {
        #[cfg(feature = "bool")]
        TypeTag::Bool => {
          if data.len() != 1 {
            return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "Bool const entry must be 1 byte".to_string(), id: line!(), kind: MechErrorKind::GenericError("Bool const entry must be 1 byte".to_string())});
          }
          let value = data[0] != 0;
          Value::Bool(Ref::new(value))
        },
        #[cfg(feature = "string")]
        TypeTag::String => {
          // string is utf-8 bytes
          match String::from_utf8(data) {
            Ok(s) => Value::String(Ref::new(s)),
            Err(_) => return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "Invalid UTF-8 in string constant".to_string(), id: line!(), kind: MechErrorKind::GenericError("Invalid UTF-8 in string constant".to_string())}),
          }
        },
        #[cfg(feature = "u8")]
        TypeTag::U8 => {
          if data.len() != 1 {
            return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "U8 const entry must be 1 byte".to_string(), id: line!(), kind: MechErrorKind::GenericError("U8 const entry must be 1 byte".to_string())});
          }
          let value = data[0];
          Value::U8(Ref::new(value))
        },
        #[cfg(feature = "u16")]
        TypeTag::U16 => {
          if data.len() != 2 {
            return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "U16 const entry must be 2 bytes".to_string(), id: line!(), kind: MechErrorKind::GenericError("U16 const entry must be 2 bytes".to_string())});
          }
          let value = u16::from_le_bytes(data.try_into().unwrap());
          Value::U16(Ref::new(value))
        },
        #[cfg(feature = "u32")]
        TypeTag::U32 => {
          if data.len() != 4 {
            return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "U32 const entry must be 4 bytes".to_string(), id: line!(), kind: MechErrorKind::GenericError("U32 const entry must be 4 bytes".to_string())});
          }
          let value = u32::from_le_bytes(data.try_into().unwrap());
          Value::U32(Ref::new(value))
        },
        #[cfg(feature = "u64")]
        TypeTag::U64 => {
          if data.len() != 8 {
            return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "U64 const entry must be 8 bytes".to_string(), id: line!(), kind: MechErrorKind::GenericError("U64 const entry must be 8 bytes".to_string())});
          }
          let value = u64::from_le_bytes(data.try_into().unwrap());
          Value::U64(Ref::new(value))
        },
        #[cfg(feature = "u128")]
        TypeTag::U128 => {
          if data.len() != 16 {
            return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "U128 const entry must be 16 bytes".to_string(), id: line!(), kind: MechErrorKind::GenericError("U128 const entry must be 16 bytes".to_string())});
          }
          let value = u128::from_le_bytes(data.try_into().unwrap());
          Value::U128(Ref::new(value))
        },
        #[cfg(feature = "i8")]
        TypeTag::I8 => {
          if data.len() != 1 {
            return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "I8 const entry must be 1 byte".to_string(), id: line!(), kind: MechErrorKind::GenericError("I8 const entry must be 1 byte".to_string())});
          }
          let value = data[0] as i8;
          Value::I8(Ref::new(value))
        },
        #[cfg(feature = "i16")]
        TypeTag::I16 => {
          if data.len() != 2 {
            return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "I16 const entry must be 2 bytes".to_string(), id: line!(), kind: MechErrorKind::GenericError("I16 const entry must be 2 bytes".to_string())});
          }
          let value = i16::from_le_bytes(data.try_into().unwrap());
          Value::I16(Ref::new(value))
        },
        #[cfg(feature = "i32")]
        TypeTag::I32 => {
          if data.len() != 4 {
            return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "I32 const entry must be 4 bytes".to_string(), id: line!(), kind: MechErrorKind::GenericError("I32 const entry must be 4 bytes".to_string())});
          }
          let value = i32::from_le_bytes(data.try_into().unwrap());
          Value::I32(Ref::new(value))
        },
        #[cfg(feature = "i64")]
        TypeTag::I64 => {
          if data.len() != 8 {
            return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "I64 const entry must be 8 bytes".to_string(), id: line!(), kind: MechErrorKind::GenericError("I64 const entry must be 8 bytes".to_string())});
          }
          let value = i64::from_le_bytes(data.try_into().unwrap());
          Value::I64(Ref::new(value))
        }
        #[cfg(feature = "i128")]
        TypeTag::I128 => {
          if data.len() != 16 {
            return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "I128 const entry must be 16 bytes".to_string(), id: line!(), kind: MechErrorKind::GenericError("I128 const entry must be 16 bytes".to_string())});
          }
          let value = i128::from_le_bytes(data.try_into().unwrap());
          Value::I128(Ref::new(value))
        },
        #[cfg(feature = "f32")]
        TypeTag::F32 => {
          if data.len() != 4 {
            return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "F32 const entry must be 4 bytes".to_string(), id: line!(), kind: MechErrorKind::GenericError("F32 const entry must be 4 bytes".to_string())});
          }
          let value = f32::from_le_bytes(data.try_into().unwrap());
          Value::F32(Ref::new(F32::new(value)))
        },
        #[cfg(feature = "f64")]
        TypeTag::F64 => {
          if data.len() != 8 {
            return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "F64 const entry must be 8 bytes".to_string(), id: line!(), kind: MechErrorKind::GenericError("F64 const entry must be 8 bytes".to_string())});
          }
          let value = f64::from_le_bytes(data.try_into().unwrap());
          Value::F64(Ref::new(F64::new(value)))
        },
        #[cfg(feature = "complex")]
        TypeTag::C64 => {
          if data.len() != 16 {
            return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "C64 const entry must be 8 bytes real + 8 bytes imag".to_string(), id: line!(), kind: MechErrorKind::GenericError("C64 const entry must be 8 bytes real + 8 bytes imag".to_string())});
          }
          let real = f64::from_le_bytes(data[0..8].try_into().unwrap());
          let imag = f64::from_le_bytes(data[8..16].try_into().unwrap());
          Value::ComplexNumber(Ref::new(ComplexNumber::new(real, imag)))
        },
        #[cfg(feature = "rational")]
        TypeTag::R64 => {
          if data.len() != 16 {
            return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "R64 const entry must be 8 bytes numerator + 8 bytes denominator".to_string(), id: line!(), kind: MechErrorKind::GenericError("R64 const entry must be 8 bytes numerator + 8 bytes denominator".to_string())});
          }
          let numer = i64::from_le_bytes(data[0..8].try_into().unwrap());
          let denom = i64::from_le_bytes(data[8..16].try_into().unwrap());
          Value::RationalNumber(Ref::new(RationalNumber::new(numer, denom)))
        },
        #[cfg(feature = "matrix")]
        TypeTag::MatrixU8 => {
          if data.len() < 8 {
            return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "Matrix const entry must be at least 8 bytes".to_string(), id: line!(), kind: MechErrorKind::GenericError("Matrix const entry must be at least 8 bytes".to_string())});
          }
          let rows = u32::from_le_bytes(data[0..4].try_into().unwrap()) as usize;
          let cols = u32::from_le_bytes(data[4..8].try_into().unwrap()) as usize;
          let mut elements = Vec::with_capacity(rows * cols);
          for i in 0..(rows * cols) {
            let start = 8 + i * 1;
            let end = start + 1;
            let val = data[start..end].try_into().unwrap();
            elements.push(u8::from_le_bytes(val));
          }
          Value::MatrixU8(Matrix::from_vec(elements, rows, cols))
        }
        #[cfg(feature = "matrix")]
        TypeTag::MatrixI8 => {
          if data.len() < 8 {
            return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "Matrix const entry must be at least 8 bytes".to_string(), id: line!(), kind: MechErrorKind::GenericError("Matrix const entry must be at least 8 bytes".to_string())});
          }
          let rows = u32::from_le_bytes(data[0..4].try_into().unwrap()) as usize;
          let cols = u32::from_le_bytes(data[4..8].try_into().unwrap()) as usize;
          let mut elements = Vec::with_capacity(rows * cols);
          for i in 0..(rows * cols) {
            let start = 8 + i * 1;
            let end = start + 1;
            let val = data[start..end].try_into().unwrap();
            elements.push(i8::from_le_bytes(val));
          }
          Value::MatrixI8(Matrix::from_vec(elements, rows, cols))
        }
        #[cfg(feature = "matrix")]
        TypeTag::MatrixF32 => {
          if data.len() < 8 {
            return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "Matrix const entry must be at least 8 bytes".to_string(), id: line!(), kind: MechErrorKind::GenericError("Matrix const entry must be at least 8 bytes".to_string())});
          }
          let rows = u32::from_le_bytes(data[0..4].try_into().unwrap()) as usize;
          let cols = u32::from_le_bytes(data[4..8].try_into().unwrap()) as usize;
          let mut elements = Vec::with_capacity(rows * cols);
          for i in 0..(rows * cols) {
            let start = 8 + i * 4;
            let end = start + 4;
            let val = data[start..end].try_into().unwrap();
            let fval = f32::from_le_bytes(val);
            elements.push(F32::new(fval));
          }
          Value::MatrixF32(Matrix::from_vec(elements, rows, cols))
        }
        #[cfg(feature = "matrix")]
        TypeTag::MatrixF64 => {
          // first 8 bytes are rows (u32) and cols (u32)
          if data.len() < 8 {
            return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "Matrix const entry must be at least 8 bytes".to_string(), id: line!(), kind: MechErrorKind::GenericError("Matrix const entry must be at least 8 bytes".to_string())});
          }
          let rows = u32::from_le_bytes(data[0..4].try_into().unwrap()) as usize;
          let cols = u32::from_le_bytes(data[4..8].try_into().unwrap()) as usize;
          let mut elements = Vec::with_capacity(rows * cols);
          for i in 0..(rows * cols) {
            let start = 8 + i * 8;
            let end = start + 8;
            let val = f64::from_le_bytes(data[start..end].try_into().unwrap());
            elements.push(F64::new(val));
          }
          Value::MatrixF64(Matrix::from_vec(elements, rows, cols))
        }
        TypeTag::MatrixU16 => {extract_matrix!(MatrixU16, u16, 2, data)},
        TypeTag::MatrixU32 => {extract_matrix!(MatrixU32, u32, 4, data)},
        TypeTag::MatrixU64 => {extract_matrix!(MatrixU64, u64, 8, data)},
        TypeTag::MatrixU128 => {extract_matrix!(MatrixU128, u128, 16, data)},
        TypeTag::MatrixI16 => {extract_matrix!(MatrixI16, i16, 2, data)},
        TypeTag::MatrixI32 => {extract_matrix!(MatrixI32, i32, 4, data)},
        TypeTag::MatrixI64 => {extract_matrix!(MatrixI64, i64, 8, data)},
        TypeTag::MatrixI128 => {extract_matrix!(MatrixI128, i128, 16, data)},
        TypeTag::MatrixC64 => {extract_matrix!(MatrixComplexNumber, ComplexNumber, 16, data)},
        TypeTag::MatrixR64 => {extract_matrix!(MatrixRationalNumber, RationalNumber, 16, data)},
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

// Load Program
// ----------------------------------------------------------------------------

#[cfg(not(feature = "no_std"))]
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

pub fn load_program_from_bytes(bytes: &[u8]) -> MResult<ParsedProgram> {
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


impl ParsedConstEntry {
  pub fn write_to<W: Write>(&self, w: &mut W) -> MResult<()> {
    // type_id (u32)
    w.write_u32::<LittleEndian>(self.type_id)
      .map_err(|e| MechError {
        file: file!().to_string(),
        tokens: vec![],
        msg: e.to_string(),
        id: line!(),
        kind: MechErrorKind::GenericError(e.to_string()),
      })?;
    // enc, align, flags, reserved (u8 each)
    w.write_u8(self.enc).map_err(|e| MechError {
      file: file!().to_string(),
      tokens: vec![],
      msg: e.to_string(),
      id: line!(),
      kind: MechErrorKind::GenericError(e.to_string()),
    })?;
    w.write_u8(self.align).map_err(|e| MechError {
      file: file!().to_string(),
      tokens: vec![],
      msg: e.to_string(),
      id: line!(),
      kind: MechErrorKind::GenericError(e.to_string()),
    })?;
    w.write_u8(self.flags).map_err(|e| MechError {
      file: file!().to_string(),
      tokens: vec![],
      msg: e.to_string(),
      id: line!(),
      kind: MechErrorKind::GenericError(e.to_string()),
    })?;
    w.write_u8(self.reserved).map_err(|e| MechError {
      file: file!().to_string(),
      tokens: vec![],
      msg: e.to_string(),
      id: line!(),
      kind: MechErrorKind::GenericError(e.to_string()),
    })?;
    // offset (u64)
    w.write_u64::<LittleEndian>(self.offset).map_err(|e| MechError {
      file: file!().to_string(),
      tokens: vec![],
      msg: e.to_string(),
      id: line!(),
      kind: MechErrorKind::GenericError(e.to_string()),
    })?;
    // length (u64)
    w.write_u64::<LittleEndian>(self.length).map_err(|e| MechError {
      file: file!().to_string(),
      tokens: vec![],
      msg: e.to_string(),
      id: line!(),
      kind: MechErrorKind::GenericError(e.to_string()),
    })?;

    Ok(())
  }
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
  NullOp {fxn_id: u64, dst: u32 },
  UnOp { fxn_id: u64, dst: u32, src: u32 },
  BinOp { fxn_id: u64, dst: u32, lhs: u32, rhs: u32 },
  TernOp { fxn_id: u64, dst: u32, a: u32, b: u32, c: u32 },
  QuadOp { fxn_id: u64, dst: u32, a: u32, b: u32, c: u32, d: u32 },
  VarArg { fxn_id: u64, dst: u32, args: Vec<u32> },
  Ret { src: u32 },
  Unknown { opcode: u8, rest: Vec<u8> }, // unknown opcode or dynamic form
}

impl DecodedInstr {
 pub fn from_u8(num: u8) -> Option<DecodedInstr> {
    match OpCode::from_u8(num) {
      Some(OpCode::ConstLoad) => Some(DecodedInstr::ConstLoad { dst: 0, const_id: 0 }),
      Some(OpCode::NullOp) => Some(DecodedInstr::NullOp { fxn_id: 0, dst: 0 }),
      Some(OpCode::Unop) => Some(DecodedInstr::UnOp { fxn_id: 0, dst: 0, src: 0 }),
      Some(OpCode::Binop) => Some(DecodedInstr::BinOp { fxn_id: 0, dst: 0, lhs: 0, rhs: 0 }),
      Some(OpCode::Ternop) => Some(DecodedInstr::TernOp { fxn_id: 0, dst: 0, a: 0, b: 0, c: 0 }),
      Some(OpCode::Quadop) => Some(DecodedInstr::QuadOp { fxn_id: 0, dst: 0, a: 0, b: 0, c: 0, d: 0 }),
      Some(OpCode::VarArg) => Some(DecodedInstr::VarArg { fxn_id: 0, dst: 0, args: vec![] }),
      Some(OpCode::Return) => Some(DecodedInstr::Ret { src: 0 }),
      _ => None,
    }
  }
}

fn decode_instructions(mut cur: Cursor<&[u8]>) -> MResult<Vec<DecodedInstr>> {
  let mut out = Vec::new();
  while (cur.position() as usize) < cur.get_ref().len() {
    // read opcode (u64)
    let pos_before = cur.position();
    // if remaining < 8, can't read opcode
    let rem = cur.get_ref().len() - pos_before as usize;
    if rem < 8 {
      return Err(MechError {
        file: file!().to_string(),
        tokens: vec![],
        msg: "Truncated instruction: cannot read opcode".to_string(),
        id: line!(),
        kind: MechErrorKind::GenericError("Truncated instruction".to_string()),
      });
    }
    let opcode_byte = cur.read_u8()?;
    match OpCode::from_u8(opcode_byte) {
      Some(OpCode::ConstLoad) => {
        // need 4+4 bytes
        let dst = cur.read_u32::<LittleEndian>()?;
        let const_id = cur.read_u32::<LittleEndian>()?;
        out.push(DecodedInstr::ConstLoad { dst, const_id });
      }
      Some(OpCode::Return) => {
        let src = cur.read_u32::<LittleEndian>()?;
        out.push(DecodedInstr::Ret { src });
      }
      Some(OpCode::NullOp) => {
        // need 8+4 bytes
        let fxn_id = cur.read_u64::<LittleEndian>()?;
        let dst = cur.read_u32::<LittleEndian>()?;
        out.push(DecodedInstr::NullOp { fxn_id: fxn_id, dst });
      }
      Some(OpCode::Unop) => {
        // need 8+4+4 bytes
        let fxn_id = cur.read_u64::<LittleEndian>()?;
        let dst = cur.read_u32::<LittleEndian>()?;
        let src = cur.read_u32::<LittleEndian>()?;
        out.push(DecodedInstr::UnOp { fxn_id: fxn_id, dst, src });
      }
      Some(OpCode::Binop) => {
        // need 8+4+4+4 bytes
        let fxn_id = cur.read_u64::<LittleEndian>()?;
        let dst = cur.read_u32::<LittleEndian>()?;
        let lhs = cur.read_u32::<LittleEndian>()?;
        let rhs = cur.read_u32::<LittleEndian>()?;
        out.push(DecodedInstr::BinOp { fxn_id: fxn_id, dst, lhs, rhs });
      }
      Some(OpCode::Ternop) => {
        // need 8+4+4+4+4 bytes
        let fxn_id = cur.read_u64::<LittleEndian>()?;
        let dst = cur.read_u32::<LittleEndian>()?;
        let a = cur.read_u32::<LittleEndian>()?;
        let b = cur.read_u32::<LittleEndian>()?;
        let c = cur.read_u32::<LittleEndian>()?;
        out.push(DecodedInstr::TernOp { fxn_id: fxn_id, dst, a, b, c });
      }
      Some(OpCode::Quadop) => {
        // need 8+4+4+4+4+4 bytes
        let fxn_id = cur.read_u64::<LittleEndian>()?;
        let dst = cur.read_u32::<LittleEndian>()?;
        let a = cur.read_u32::<LittleEndian>()?;
        let b = cur.read_u32::<LittleEndian>()?;
        let c = cur.read_u32::<LittleEndian>()?;
        let d = cur.read_u32::<LittleEndian>()?;
        out.push(DecodedInstr::QuadOp { fxn_id: fxn_id, dst, a, b, c, d });
      }
      Some(OpCode::VarArg) => {
        // need at least 8+4+4 bytes
        let fxn_id = cur.read_u64::<LittleEndian>()?;
        let dst = cur.read_u32::<LittleEndian>()?;
        let arg_count = cur.read_u32::<LittleEndian>()? as usize;
        let mut args = Vec::with_capacity(arg_count);
        for _ in 0..arg_count {
          let a = cur.read_u32::<LittleEndian>()?;
          args.push(a);
        }
        out.push(DecodedInstr::VarArg { fxn_id: fxn_id, dst, args });
      }
      unknown => {
        return Err(MechError {
          file: file!().to_string(),
          tokens: vec![],
          msg: format!("Unknown opcode: {:?}", unknown),
          id: line!(),
          kind: MechErrorKind::GenericError("Unknown opcode".to_string()),
        });
      }
    }
  }
  Ok(out)
}

impl DecodedInstr {
  pub fn write_to<W: Write>(&self, w: &mut W) -> MResult<()> {
    match self {
      DecodedInstr::ConstLoad { dst, const_id } => {
        w.write_u8(OpCode::ConstLoad as u8)?;
        w.write_u32::<LittleEndian>(*dst)?;
        w.write_u32::<LittleEndian>(*const_id)?;
      }
      DecodedInstr::NullOp { fxn_id, dst } => {
        w.write_u8(OpCode::NullOp as u8)?;
        w.write_u64::<LittleEndian>(*fxn_id)?;
        w.write_u32::<LittleEndian>(*dst)?;
      }
      DecodedInstr::UnOp { fxn_id, dst, src } => {
        w.write_u8(OpCode::Unop as u8)?;
        w.write_u64::<LittleEndian>(*fxn_id)?;
        w.write_u32::<LittleEndian>(*dst)?;
        w.write_u32::<LittleEndian>(*src)?;
      }
      DecodedInstr::BinOp { fxn_id, dst, lhs, rhs } => {
        w.write_u8(OpCode::Binop as u8)?;
        w.write_u64::<LittleEndian>(*fxn_id)?;
        w.write_u32::<LittleEndian>(*dst)?;
        w.write_u32::<LittleEndian>(*lhs)?;
        w.write_u32::<LittleEndian>(*rhs)?;
      }
      DecodedInstr::TernOp { fxn_id, dst, a, b, c } => {
        w.write_u8(OpCode::Ternop as u8)?;
        w.write_u64::<LittleEndian>(*fxn_id)?;
        w.write_u32::<LittleEndian>(*dst)?;
        w.write_u32::<LittleEndian>(*a)?;
        w.write_u32::<LittleEndian>(*b)?;
        w.write_u32::<LittleEndian>(*c)?;
      }
      DecodedInstr::QuadOp { fxn_id, dst, a, b, c, d } => {
        w.write_u8(OpCode::Quadop as u8)?;
        w.write_u64::<LittleEndian>(*fxn_id)?;
        w.write_u32::<LittleEndian>(*dst)?;
        w.write_u32::<LittleEndian>(*a)?;
        w.write_u32::<LittleEndian>(*b)?;
        w.write_u32::<LittleEndian>(*c)?;
        w.write_u32::<LittleEndian>(*d)?;
      }
      DecodedInstr::VarArg { fxn_id, dst, args } => {
        w.write_u8(OpCode::VarArg as u8)?;
        w.write_u64::<LittleEndian>(*fxn_id)?;
        w.write_u32::<LittleEndian>(*dst)?;
        w.write_u32::<LittleEndian>(args.len() as u32)?;
        for a in args {
          w.write_u32::<LittleEndian>(*a)?;
        }
      }
      DecodedInstr::Ret { src } => {
        w.write_u8(OpCode::Return as u8)?;
        w.write_u32::<LittleEndian>(*src)?;
      }
      DecodedInstr::Unknown { opcode, rest } => {
        w.write_u8(*opcode)?;
        w.write_all(rest)?;
      }
    }
    Ok(())
  }
}
