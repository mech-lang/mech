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
  pub mutable_symbols: HashSet<u64>,
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
      let mutable = self.mutable_symbols.contains(id);
      let entry = SymbolEntry::new(*id, mutable, *reg);
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
      return Err(
        MechError2::new(
          InvalidMagicNumberError,
          None,
        ).with_compiler_loc()
      );
    }

    // Check version number
    if self.header.version != 1 {
      return Err(
        MechError2::new(
          UnsupportedBytecodeVersionError,
          None,
        ).with_compiler_loc()
      );
    }

    // Check mech version
    if self.header.mech_ver != parse_version_to_u16(env!("CARGO_PKG_VERSION")).unwrap() {
      return Err(
        MechError2::new(
          IncompatibleMechVersionError,
          None,
        ).with_compiler_loc()
      );
    }

    Ok(())
  }

  pub fn decode_const_entries(&self) -> MResult<Vec<Value>> {
    let mut out = Vec::with_capacity(self.const_entries.len());
    let blob_len = self.const_blob.len() as u64;

    for const_entry in &self.const_entries {
      // Encoding check
      if const_entry.enc != ConstEncoding::Inline as u8 {
        return Err(
          MechError2::new(
            UnsupportedConstantEncodingError,
            None,
          ).with_compiler_loc()
        );
      }

      // Bounds check #1
      if const_entry.offset.checked_add(const_entry.length).is_none() {
        return Err(
          MechError2::new(
            ConstantEntryOutOfBoundsError,
            None,
          ).with_compiler_loc()
        );
      }

      // Bounds check #2
      let end = const_entry.offset + const_entry.length;
      if end > blob_len {
        return Err(
          MechError2::new(
            ConstantEntryOutOfBoundsError,
            None,
          ).with_compiler_loc()
        );
      }

      // Alignment check
      if !check_alignment(const_entry.offset, const_entry.align) {
        return Err(
          MechError2::new(
            ConstantEntryAlignmentError,
            None,
          ).with_compiler_loc()
        );
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
            return Err(MechError2::new(ConstantWrongSizeError {expected: 1,found: data.len(),type_name: "Bool",},None,).with_compiler_loc());
          }
          let value = data[0] != 0;
          Value::Bool(Ref::new(value))
        }
        #[cfg(feature = "string")]
        TypeTag::String => {
          if data.len() < 4 {
            return Err(MechError2::new(ConstantWrongSizeError {expected: 4, found: data.len(), type_name: "String",}, None,).with_compiler_loc());
          }
          let s = String::from_le(&data);
          Value::String(Ref::new(s))
        }
        #[cfg(feature = "u8")]
        TypeTag::U8 => {
          if data.len() != 1 { 
            return Err(MechError2::new(ConstantWrongSizeError { expected: 1, found: data.len(), type_name: "U8" }, None).with_compiler_loc()); 
          }
          let value = data[0];
          Value::U8(Ref::new(value))
        },
        #[cfg(feature = "u16")]
        TypeTag::U16 => {
          if data.len() != 2 {
            return Err(MechError2::new(ConstantWrongSizeError { expected: 2, found: data.len(), type_name: "U16" }, None).with_compiler_loc());
          }
          let value = u16::from_le_bytes(data.try_into().unwrap());
          Value::U16(Ref::new(value))
        },
        #[cfg(feature = "u32")]
        TypeTag::U32 => {
          if data.len() != 4 {
            return Err(MechError2::new(ConstantWrongSizeError { expected: 4, found: data.len(), type_name: "U32" }, None).with_compiler_loc());
          }
          let value = u32::from_le_bytes(data.try_into().unwrap());
          Value::U32(Ref::new(value))
        },
        #[cfg(feature = "u64")]
        TypeTag::U64 => {
          if data.len() != 8 {
            return Err(MechError2::new(ConstantWrongSizeError { expected: 8, found: data.len(), type_name: "U64" }, None).with_compiler_loc());
          }
          let value = u64::from_le_bytes(data.try_into().unwrap());
          Value::U64(Ref::new(value))
        },
        #[cfg(feature = "u128")]
        TypeTag::U128 => {
          if data.len() != 16 {
            return Err(MechError2::new(ConstantWrongSizeError { expected: 16, found: data.len(), type_name: "U128" }, None).with_compiler_loc());
          }
          let value = u128::from_le_bytes(data.try_into().unwrap());
          Value::U128(Ref::new(value))
        },
        #[cfg(feature = "i8")]
        TypeTag::I8 => {
          if data.len() != 1 {
            return Err(MechError2::new(ConstantWrongSizeError { expected: 1, found: data.len(), type_name: "I8" }, None).with_compiler_loc());
          }
          let value = data[0] as i8;
          Value::I8(Ref::new(value))
        },
        #[cfg(feature = "i16")]
        TypeTag::I16 => {
          if data.len() != 2 {
            return Err(MechError2::new(ConstantWrongSizeError { expected: 2, found: data.len(), type_name: "I16" }, None).with_compiler_loc());
          }
          let value = i16::from_le_bytes(data.try_into().unwrap());
          Value::I16(Ref::new(value))
        },
        #[cfg(feature = "i32")]
        TypeTag::I32 => {
          if data.len() != 4 {
            return Err(MechError2::new(ConstantWrongSizeError { expected: 4, found: data.len(), type_name: "I32" }, None).with_compiler_loc());
          }
          let value = i32::from_le_bytes(data.try_into().unwrap());
          Value::I32(Ref::new(value))
        },
        #[cfg(feature = "i64")]
        TypeTag::I64 => {
          if data.len() != 8 {
            return Err(MechError2::new(ConstantWrongSizeError { expected: 8, found: data.len(), type_name: "I64" }, None).with_compiler_loc());
          }
          let value = i64::from_le_bytes(data.try_into().unwrap());
          Value::I64(Ref::new(value))
        },
        #[cfg(feature = "i128")]
        TypeTag::I128 => {
          if data.len() != 16 {
            return Err(MechError2::new(ConstantWrongSizeError { expected: 16, found: data.len(), type_name: "i128" }, None).with_compiler_loc());
          }
          let value = i128::from_le_bytes(data.try_into().unwrap());
          Value::I128(Ref::new(value))
        },
        #[cfg(feature = "f32")]
        TypeTag::F32 => {
          if data.len() != 4 {
            return Err(MechError2::new(ConstantWrongSizeError { expected: 4, found: data.len(), type_name: "f32" }, None).with_compiler_loc());
          }
          let value = f32::from_le_bytes(data.try_into().unwrap());
          Value::F32(Ref::new(F32::new(value)))
        },
        #[cfg(feature = "f64")]
        TypeTag::F64 => {
          if data.len() != 8 {
            return Err(MechError2::new(ConstantWrongSizeError { expected: 8, found: data.len(), type_name: "f64" }, None).with_compiler_loc());
          }
          let value = f64::from_le_bytes(data.try_into().unwrap());
          Value::F64(Ref::new(F64::new(value)))
        },
        #[cfg(feature = "complex")]
        TypeTag::C64 => {
          if data.len() != 16 {
            return Err(MechError2::new(ConstantWrongSizeError { expected: 16, found: data.len(), type_name: "c64" }, None).with_compiler_loc());
          }
          let real = f64::from_le_bytes(data[0..8].try_into().unwrap());
          let imag = f64::from_le_bytes(data[8..16].try_into().unwrap());
          Value::C64(Ref::new(C64::new(real, imag)))
        },
        #[cfg(feature = "rational")]
        TypeTag::R64 => {
          if data.len() != 16 {
            return Err(MechError2::new(ConstantWrongSizeError { expected: 16, found: data.len(), type_name: "r64" }, None).with_compiler_loc());
          }
          let numer = i64::from_le_bytes(data[0..8].try_into().unwrap());
          let denom = i64::from_le_bytes(data[8..16].try_into().unwrap());
          Value::R64(Ref::new(R64::new(numer, denom)))
        },
        #[cfg(all(feature = "matrix", feature = "string"))]
        TypeTag::MatrixString => {
          if data.len() < 8 {
            return Err(MechError2::new(ConstantTooShortError { type_name: "[string]" }, None).with_compiler_loc());
          }
          let matrix = Matrix::<String>::from_le(&data);
          Value::MatrixString(matrix)
        }
        #[cfg(all(feature = "matrix", feature = "bool"))]
        TypeTag::MatrixBool => {
          if data.len() < 1 {
            return Err(MechError2::new(ConstantTooShortError { type_name: "[bool]" }, None).with_compiler_loc());
          }
          let matrix = Matrix::<bool>::from_le(&data);
          Value::MatrixBool(matrix)
        }
        #[cfg(all(feature = "matrix", feature = "u8"))]
        TypeTag::MatrixU8 => {
          if data.len() < 1 {
            return Err(MechError2::new(ConstantTooShortError { type_name: "[u8]" }, None).with_compiler_loc());
          }
          let matrix = Matrix::<u8>::from_le(&data);
          Value::MatrixU8(matrix)
        }
        #[cfg(all(feature = "matrix", feature = "i8"))]
        TypeTag::MatrixI8 => {
          if data.len() < 1 {
            return Err(MechError2::new(ConstantTooShortError { type_name: "[i8]" }, None).with_compiler_loc());
          }
          let matrix = Matrix::<i8>::from_le(&data);
          Value::MatrixI8(matrix)
        }
        #[cfg(all(feature = "matrix", feature = "f32"))]
        TypeTag::MatrixF32 => {
          if data.len() < 4 {
            return Err(MechError2::new(ConstantTooShortError { type_name: "[f32]" }, None).with_compiler_loc());
          }
          let matrix = Matrix::<F32>::from_le(&data);
          Value::MatrixF32(matrix)
        }
        #[cfg(all(feature = "matrix", feature = "f64"))]
        TypeTag::MatrixF64 => {
          if data.len() < 8 {
            return Err(MechError2::new(ConstantTooShortError { type_name: "[f64]" }, None).with_compiler_loc());
          }
          let matrix = Matrix::<F64>::from_le(&data);
          Value::MatrixF64(matrix)
        }
        #[cfg(all(feature = "matrix", feature = "u16"))]
        TypeTag::MatrixU16 => {
          if data.len() < 2 {
            return Err(MechError2::new(ConstantTooShortError { type_name: "[u16]" }, None).with_compiler_loc());
          }
          let matrix = Matrix::<u16>::from_le(&data);
          Value::MatrixU16(matrix)
        }
        #[cfg(all(feature = "matrix", feature = "u32"))]
        TypeTag::MatrixU32 => {
          if data.len() < 4 {
            return Err(MechError2::new(ConstantTooShortError { type_name: "[u32]" }, None).with_compiler_loc());
          }
          let matrix = Matrix::<u32>::from_le(&data);
          Value::MatrixU32(matrix)
        }
        #[cfg(all(feature = "matrix", feature = "u64"))]
        TypeTag::MatrixU64 => {
          if data.len() < 8 {
            return Err(MechError2::new(ConstantTooShortError { type_name: "[u64]" }, None).with_compiler_loc());
          }
          let matrix = Matrix::<u64>::from_le(&data);
          Value::MatrixU64(matrix)
        }
        #[cfg(all(feature = "matrix", feature = "u128"))]
        TypeTag::MatrixU128 => {
          if data.len() < 16 {
            return Err(MechError2::new(ConstantTooShortError { type_name: "[u128]" }, None).with_compiler_loc());
          }
          let matrix = Matrix::<u128>::from_le(&data);
          Value::MatrixU128(matrix)
        }
        #[cfg(all(feature = "matrix", feature = "i16"))]
        TypeTag::MatrixI16 => {
          if data.len() < 2 {
            return Err(MechError2::new(ConstantTooShortError { type_name: "[i16]" }, None).with_compiler_loc());
          }
          let matrix = Matrix::<i16>::from_le(&data);
          Value::MatrixI16(matrix)
        }
        #[cfg(all(feature = "matrix", feature = "i32"))]
        TypeTag::MatrixI32 => {
          if data.len() < 4 {
            return Err(MechError2::new(ConstantTooShortError { type_name: "[i32]" }, None).with_compiler_loc());
          }
          let matrix = Matrix::<i32>::from_le(&data);
          Value::MatrixI32(matrix)
        }
        #[cfg(all(feature = "matrix", feature = "i64"))]
        TypeTag::MatrixI64 => {
          if data.len() < 8 {
            return Err(MechError2::new(ConstantTooShortError { type_name: "[i64]" }, None).with_compiler_loc());
          }
          let matrix = Matrix::<i64>::from_le(&data);
          Value::MatrixI64(matrix)
        },
        #[cfg(all(feature = "matrix", feature = "i128"))]
        TypeTag::MatrixI128 => {
          if data.len() < 8 {
            return Err(MechError2::new(ConstantTooShortError { type_name: "[i128]" }, None).with_compiler_loc());
          }
          let matrix = Matrix::<i128>::from_le(&data);
          Value::MatrixI128(matrix)
        },
        #[cfg(all(feature = "matrix", feature = "c64"))]
        TypeTag::MatrixC64 => {
          if data.len() < 8 {
            return Err(MechError2::new(ConstantTooShortError { type_name: "[c64]" }, None).with_compiler_loc());
          }
          let matrix = Matrix::<C64>::from_le(&data);
          Value::MatrixC64(matrix)
        },
        #[cfg(all(feature = "matrix", feature = "r64"))]
        TypeTag::MatrixR64 => {
          if data.len() < 8 {
            return Err(MechError2::new(ConstantTooShortError { type_name: "[r64]" }, None).with_compiler_loc());
          }
          let matrix = Matrix::<R64>::from_le(&data);
          Value::MatrixR64(matrix)
        },
        #[cfg(feature = "matrix")]
        TypeTag::MatrixIndex => {
          if data.len() < 8 {
            return Err(MechError2::new(ConstantTooShortError { type_name: "[ix]" }, None).with_compiler_loc());
          }
          let matrix = Matrix::<usize>::from_le(&data);
          Value::MatrixIndex(matrix)
        },
        TypeTag::Index => {
          if data.len() != 8 {
            return Err(MechError2::new(ConstantWrongSizeError { expected: 8, found: data.len(), type_name: "Index" }, None).with_compiler_loc());
          }
          let value = u64::from_le_bytes(data.try_into().unwrap()) as usize;
          Value::Index(Ref::new(value))
        },
        #[cfg(feature = "set")]
        TypeTag::Set => {
          if data.len() < 4 {
            return Err(MechError2::new(ConstantTooShortError { type_name: "set" }, None).with_compiler_loc());
          }
          let set = MechSet::from_le(&data);
          Value::Set(Ref::new(set))
        },
        #[cfg(feature = "table")]
        TypeTag::Table => {
          if data.len() < 8 {
            return Err(MechError2::new(ConstantTooShortError { type_name: "table" }, None).with_compiler_loc());
          }
          let table = MechTable::from_le(&data);
          Value::Table(Ref::new(table))
        }
        _ => {
          return Err(
            MechError2::new(
              UnsupportedConstantTypeError { type_tag: ty.tag },
              None,
            )
            .with_compiler_loc()
          );
        }    
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
  if !header.validate_magic(b"MECH") {
    return Err(
      MechError2::new(InvalidMagicNumberError, None)
        .with_compiler_loc()
    );
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
        return Err(
        MechError2::new(UnknownConstantTypeError { tag }, None)
          .with_compiler_loc()
        );
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
  let mut mutable_symbols = HashSet::new();
  if header.symbols_off != 0 && header.symbols_len > 0 {
    r.seek(SeekFrom::Start(header.symbols_off))?;
    let mut symbols_bytes = vec![0u8; header.symbols_len as usize];
    r.read_exact(&mut symbols_bytes)?;
    let mut cur = Cursor::new(&symbols_bytes[..]);
    for _ in 0..(header.symbols_len / 12) {
      let id = cur.read_u64::<LittleEndian>()?;
      let mutable = cur.read_u8()? != 0;
      let reg = cur.read_u32::<LittleEndian>()?;
      symbols.insert(id, reg);
      if mutable {
        mutable_symbols.insert(id);
      }
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
      let name = String::from_utf8(name_bytes).map_err(|_| 
          MechError2::new(InvalidUtf8InDictError, None)
            .with_compiler_loc()
      )?;
      dictionary.insert(id, name);
    }
  }

  // decode instructions
  let instrs = decode_instructions(Cursor::new(&instr_bytes[..]))?;
  
  Ok(ParsedProgram { header, features, types, const_entries, const_blob, instr_bytes, symbols, mutable_symbols, instrs, dictionary })
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
      .map_err(|e| MechError2::new(ConstEntryWriteIoError { source: format!("{}", e) }, None).with_compiler_loc())?;
    // enc, align, flags, reserved (u8 each)
    w.write_u8(self.enc)
      .map_err(|e| MechError2::new(ConstEntryWriteIoError { source: format!("{}", e) }, None).with_compiler_loc())?;
    w.write_u8(self.align)
      .map_err(|e| MechError2::new(ConstEntryWriteIoError { source: format!("{}", e) }, None).with_compiler_loc())?;
    w.write_u8(self.flags)
      .map_err(|e| MechError2::new(ConstEntryWriteIoError { source: format!("{}", e) }, None).with_compiler_loc())?;
    w.write_u8(self.reserved)
      .map_err(|e| MechError2::new(ConstEntryWriteIoError { source: format!("{}", e) }, None).with_compiler_loc())?;
    // offset (u64)
    w.write_u64::<LittleEndian>(self.offset)
      .map_err(|e| MechError2::new(ConstEntryWriteIoError { source: format!("{}", e) }, None).with_compiler_loc())?;
    // length (u64)
    w.write_u64::<LittleEndian>(self.length)
      .map_err(|e| MechError2::new(ConstEntryWriteIoError { source: format!("{}", e) }, None).with_compiler_loc())?;
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
    return Err(MechError2::new(
      FileTooShortError { total_len, expected_len: 4 },
      None
    ).with_compiler_loc());
  }

  r.seek(SeekFrom::Start(total_len - 4))?;
  let expected_crc = r.read_u32::<LittleEndian>()?;

  r.seek(SeekFrom::Start(0))?;
  let payload_len = (total_len - 4) as usize;
  let mut buf = vec![0u8; payload_len];
  r.read_exact(&mut buf)?;

  let file_crc = crc32fast::hash(&buf);
  if file_crc != expected_crc {
    Err(MechError2::new(
      CrcMismatchError { expected: expected_crc, found: file_crc },
      None
    ).with_compiler_loc())
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
      return Err(MechError2::new(
        TruncatedInstructionError,
        None
      ).with_compiler_loc());
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
      Some(unknown) => {
        return Err(MechError2::new(
          UnknownOpcodeError { opcode: unknown },
          None
        ).with_compiler_loc());
      }
      None => {
        return Err(MechError2::new(
          InvalidOpcodeError { opcode: opcode_byte },
          None
        ).with_compiler_loc());
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


#[derive(Debug, Clone)]
pub struct UnsupportedBytecodeVersionError;
impl MechErrorKind2 for UnsupportedBytecodeVersionError {
  fn name(&self) -> &str { "UnsupportedBytecodeVersion" }
  fn message(&self) -> String { "Unsupported bytecode version".to_string() }
}

#[derive(Debug, Clone)]
pub struct IncompatibleMechVersionError;
impl MechErrorKind2 for IncompatibleMechVersionError {
  fn name(&self) -> &str { "IncompatibleMechVersion" }
  fn message(&self) -> String { "Incompatible Mech version".to_string() }
}

#[derive(Debug, Clone)]
pub struct UnsupportedConstantEncodingError;
impl MechErrorKind2 for UnsupportedConstantEncodingError {
  fn name(&self) -> &str { "UnsupportedConstantEncoding" }
  fn message(&self) -> String { "Unsupported constant encoding".to_string() }
}

#[derive(Debug, Clone)]
pub struct ConstantEntryOutOfBoundsError;
impl MechErrorKind2 for ConstantEntryOutOfBoundsError {
  fn name(&self) -> &str { "ConstantEntryOutOfBounds" }
  fn message(&self) -> String { "Constant entry out of bounds".to_string() }
}

#[derive(Debug, Clone)]
pub struct ConstantEntryAlignmentError;
impl MechErrorKind2 for ConstantEntryAlignmentError {
  fn name(&self) -> &str { "ConstantEntryAlignmentError" }
  fn message(&self) -> String { "Constant entry alignment error".to_string() }
}

#[derive(Debug, Clone)]
pub struct ConstantWrongSizeError {
  pub expected: usize,
  pub found: usize,
  pub type_name: &'static str,
}
impl MechErrorKind2 for ConstantWrongSizeError {
  fn name(&self) -> &str { "ConstantWrongSize" }
  fn message(&self) -> String {
    format!(
      "{} constant wrong size: expected {}, found {}",
      self.type_name, self.expected, self.found
    )
  }
}

#[derive(Debug, Clone)]
pub struct ConstantTooShortError {
  pub type_name: &'static str,
}
impl MechErrorKind2 for ConstantTooShortError {
  fn name(&self) -> &str { "ConstantTooShort" }
  fn message(&self) -> String {
    format!("{} constant too short", self.type_name)
  }
}

#[derive(Debug, Clone)]
pub struct UnsupportedConstantTypeError {
  pub type_tag: TypeTag,
}
impl MechErrorKind2 for UnsupportedConstantTypeError {
  fn name(&self) -> &str { "UnsupportedConstantType" }

  fn message(&self) -> String {
    format!("Unsupported constant type {:?}", self.type_tag)
  }
}

#[derive(Debug, Clone)]
pub struct CrcMismatchError {
  pub expected: u32,
  pub found: u32,
}
impl MechErrorKind2 for CrcMismatchError {
  fn name(&self) -> &str { "CrcMismatch" }

  fn message(&self) -> String {
    format!("CRC mismatch: expected {}, found {}", self.expected, self.found)
  }
}

#[derive(Debug, Clone)]
pub struct TruncatedInstructionError;
impl MechErrorKind2 for TruncatedInstructionError {
  fn name(&self) -> &str { "TruncatedInstruction" }
  fn message(&self) -> String { "Truncated instruction: cannot read full opcode or operands".to_string() }
}

#[derive(Debug, Clone)]
pub struct UnknownOpcodeError {
  pub opcode: OpCode,
}
impl MechErrorKind2 for UnknownOpcodeError {
  fn name(&self) -> &str { "UnknownOpcode" }
  fn message(&self) -> String { format!("Unknown opcode: {}", self.opcode) }
}

#[derive(Debug, Clone)]
pub struct FileTooShortError {
  pub total_len: u64,
  pub expected_len: u64,
}
impl MechErrorKind2 for FileTooShortError {
  fn name(&self) -> &str { "FileTooShort" }
  fn message(&self) -> String {
    format!(
      "File too short: expected at least {}, got {}",
      self.expected_len, self.total_len
    )
  }
}

#[derive(Debug, Clone)]
pub struct InvalidOpcodeError {
  pub opcode: u8,
}
impl MechErrorKind2 for InvalidOpcodeError {
  fn name(&self) -> &str { "InvalidOpcode" }
  fn message(&self) -> String { format!("Invalid opcode byte: {}", self.opcode) }
}

#[derive(Debug, Clone)]
pub struct UnknownConstantTypeError {
  pub tag: u16,
}
impl MechErrorKind2 for UnknownConstantTypeError {
  fn name(&self) -> &str { "UnknownConstantType" }

  fn message(&self) -> String {
    format!("Unknown constant type: {}", self.tag)
  }
}

#[derive(Debug, Clone)]
pub struct InvalidUtf8InDictError;
impl MechErrorKind2 for InvalidUtf8InDictError {
  fn name(&self) -> &str { "InvalidUtf8InDict" }
  fn message(&self) -> String { "Invalid UTF-8 in dictionary entry".to_string() }
}

#[derive(Debug, Clone)]
pub struct ConstEntryWriteIoError {
  pub source: String,
}
impl MechErrorKind2 for ConstEntryWriteIoError {
  fn name(&self) -> &str { "ConstEntryWriteIoError" }
  fn message(&self) -> String { format!("Failed to write constant entry: {}", self.source) }
}