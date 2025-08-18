use crate::*;
use std::rc::Rc;
use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::io::{Cursor, Read, Write};
use byteorder::{LittleEndian, WriteBytesExt, ReadBytesExt};

// Interpreter 
// ----------------------------------------------------------------------------

pub struct Interpreter {
  pub id: u64,
  symbols: SymbolTableRef,
  pub code: Vec<MechSourceCode>,
  plan: Plan,
  functions: FunctionsRef,
  pub out: Value,
  pub out_values: Ref<HashMap<u64, Value>>,
  pub sub_interpreters: Ref<HashMap<u64, Box<Interpreter>>>,
}

impl Interpreter {
  pub fn new(id: u64) -> Interpreter {
    let mut interp = Interpreter {
      id,
      symbols: Ref::new(SymbolTable::new()),
      plan: Plan::new(),
      functions: Ref::new(Functions::new()),
      out: Value::Empty,
      sub_interpreters: Ref::new(HashMap::new()),
      out_values: Ref::new(HashMap::new()),
      code: Vec::new(),
    };
    interp.load_stdkinds();
    interp.load_stdlib();
    interp
  }

  pub fn load_stdkinds(&mut self) {
    let mut fxns = self.functions.borrow_mut();
    
    // Preload scalar kinds
    #[cfg(feature = "u8")]
    fxns.kinds.insert(hash_str("u8"),ValueKind::U8);
    #[cfg(feature = "u16")]
    fxns.kinds.insert(hash_str("u16"),ValueKind::U16);
    #[cfg(feature = "u32")]
    fxns.kinds.insert(hash_str("u32"),ValueKind::U32);
    #[cfg(feature = "u64")]
    fxns.kinds.insert(hash_str("u64"),ValueKind::U64);
    #[cfg(feature = "u128")]
    fxns.kinds.insert(hash_str("u128"),ValueKind::U128);
    #[cfg(feature = "i8")]
    fxns.kinds.insert(hash_str("i8"),ValueKind::I8);
    #[cfg(feature = "i16")]
    fxns.kinds.insert(hash_str("i16"),ValueKind::I16);
    #[cfg(feature = "i32")]
    fxns.kinds.insert(hash_str("i32"),ValueKind::I32);
    #[cfg(feature = "i64")]
    fxns.kinds.insert(hash_str("i64"),ValueKind::I64);
    #[cfg(feature = "i128")]
    fxns.kinds.insert(hash_str("i128"),ValueKind::I128);
    #[cfg(feature = "f32")]
    fxns.kinds.insert(hash_str("f32"),ValueKind::F32);
    #[cfg(feature = "f64")]
    fxns.kinds.insert(hash_str("f64"),ValueKind::F64);
    #[cfg(feature = "c64")]
    fxns.kinds.insert(hash_str("c64"),ValueKind::ComplexNumber);
    #[cfg(feature = "r64")]
    fxns.kinds.insert(hash_str("r64"),ValueKind::RationalNumber);
    #[cfg(feature = "string")]
    fxns.kinds.insert(hash_str("string"),ValueKind::String);
    #[cfg(feature = "bool")]
    fxns.kinds.insert(hash_str("bool"),ValueKind::Bool);
  }

  pub fn load_stdlib(&mut self) {

    let mut fxns = self.functions.borrow_mut();

    // Preload combinatorics functions
    #[cfg(feature = "combinatorics_n_choose_k")]
    fxns.function_compilers.insert(hash_str("combinatorics/n-choose-k"), Box::new(CombinatoricsNChooseK{}));

    // Preload stats functions
    #[cfg(feature = "stats_sum")]
    fxns.function_compilers.insert(hash_str("stats/sum/row"), Box::new(StatsSumRow{}));
    #[cfg(feature = "stats_sum")]
    fxns.function_compilers.insert(hash_str("stats/sum/column"), Box::new(StatsSumColumn{}));

    // Preload math functions
    #[cfg(feature = "math_sin")]
    fxns.function_compilers.insert(hash_str("math/sin"),Box::new(MathSin{}));
    #[cfg(feature = "math_cos")]
    fxns.function_compilers.insert(hash_str("math/cos"),Box::new(MathCos{}));
    #[cfg(feature = "math_atan2")]
    fxns.function_compilers.insert(hash_str("math/atan2"),Box::new(MathAtan2{}));
    #[cfg(feature = "math_atan")]
    fxns.function_compilers.insert(hash_str("math/atan"),Box::new(MathAtan{}));
    #[cfg(feature = "math_acos")]
    fxns.function_compilers.insert(hash_str("math/acos"),Box::new(MathAcos{}));
    #[cfg(feature = "math_acosh")]
    fxns.function_compilers.insert(hash_str("math/acosh"),Box::new(MathAcosh{}));
    #[cfg(feature = "math_acot")]
    fxns.function_compilers.insert(hash_str("math/acot"),Box::new(MathAcot{}));
    #[cfg(feature = "math_acsc")]
    fxns.function_compilers.insert(hash_str("math/acsc"),Box::new(MathAcsc{}));
    #[cfg(feature = "math_asec")]
    fxns.function_compilers.insert(hash_str("math/asec"),Box::new(MathAsec{}));
    #[cfg(feature = "math_asin")]
    fxns.function_compilers.insert(hash_str("math/asin"),Box::new(MathAsin{}));
    #[cfg(feature = "math_sinh")]
    fxns.function_compilers.insert(hash_str("math/sinh"),Box::new(MathSinh{}));
    #[cfg(feature = "math_cosh")]
    fxns.function_compilers.insert(hash_str("math/cosh"),Box::new(MathCosh{}));
    #[cfg(feature = "math_tanh")]
    fxns.function_compilers.insert(hash_str("math/tanh"),Box::new(MathTanh{}));
    #[cfg(feature = "math_atanh")]
    fxns.function_compilers.insert(hash_str("math/atanh"),Box::new(MathAtanh{}));
    #[cfg(feature = "math_cot")]
    fxns.function_compilers.insert(hash_str("math/cot"),Box::new(MathCot{}));
    #[cfg(feature = "math_csc")]
    fxns.function_compilers.insert(hash_str("math/csc"),Box::new(MathCsc{}));
    #[cfg(feature = "math_sec")]
    fxns.function_compilers.insert(hash_str("math/sec"),Box::new(MathSec{}));
    #[cfg(feature = "math_tan")]
    fxns.function_compilers.insert(hash_str("math/tan"),Box::new(MathTan{}));

    // Preload io functions
    #[cfg(feature = "io_print")]
    fxns.function_compilers.insert(hash_str("io/print"), Box::new(IoPrint{}));
    #[cfg(feature = "io_println")]
    fxns.function_compilers.insert(hash_str("io/println"), Box::new(IoPrintln{}));
  }

  pub fn plan(&self) -> Plan {
    self.plan.clone()
  }

  pub fn clear(&mut self) {
    self.symbols = Ref::new(SymbolTable::new());
    self.plan = Plan::new();
    self.functions = Ref::new(Functions::new());
    self.out = Value::Empty;
    self.out_values = Ref::new(HashMap::new());
    self.code = Vec::new();
    self.sub_interpreters = Ref::new(HashMap::new());
    self.load_stdkinds();
    self.load_stdlib();
  }

  pub fn get_symbol(&self, id: u64) -> Option<Ref<Value>> {
    let symbols_brrw = self.symbols.borrow();
    symbols_brrw.get(id)
  }

  pub fn symbols(&self) -> SymbolTableRef {
    self.symbols.clone()
  }

  pub fn functions(&self) -> FunctionsRef {
    self.functions.clone()
  }

  pub fn add_plan_step(&self, step: Box<dyn MechFunction>) {
    let mut plan_brrw = self.plan.borrow_mut();
    plan_brrw.push(step);
  }

  #[cfg(feature = "functions")]
  pub fn insert_function(&self, fxn: FunctionDefinition) {
    let mut fxns_brrw = self.functions.borrow_mut();
    fxns_brrw.functions.insert(fxn.id, fxn);
  }

  #[cfg(feature = "pretty_print")]
  pub fn pretty_print_symbols(&self) -> String {
    let symbol_table = self.symbols.borrow();
    symbol_table.pretty_print()
  }

  pub fn dictionary(&self) -> Ref<Dictionary> {
    let symbols_ref = self.symbols.borrow();
    symbols_ref.dictionary.clone()
  }

  pub fn step(&mut self, steps: u64) -> &Value {
    let plan_brrw = self.plan.borrow();
    let mut result = Value::Empty;
    for i in 0..steps {
      for fxn in plan_brrw.iter() {
        fxn.solve();
      }
    }
    self.out = plan_brrw.last().unwrap().out().clone();
    &self.out
  }

  pub fn interpret(&mut self, tree: &Program) -> MResult<Value> {
    self.code.push(MechSourceCode::Tree(tree.clone()));
    catch_unwind(AssertUnwindSafe(|| {
      let result = program(tree, &self);
      if let Some(last_step) = self.plan.borrow().last() {
        self.out = last_step.out().clone();
      } else {
        self.out = Value::Empty;
      }
      result
    }))
    .map_err(|err| {
      let kind = {
        if let Some(raw_msg) = err.downcast_ref::<&'static str>() {
          if raw_msg.contains("Index out of bounds") {
            MechErrorKind::IndexOutOfBounds
          } else if raw_msg.contains("attempt to subtract with overflow") {
            MechErrorKind::IndexOutOfBounds
          } else {
            MechErrorKind::GenericError(raw_msg.to_string())
          }
        } else {
          MechErrorKind::GenericError("Unknown panic".to_string())
        }
      };
      MechError {
        file: file!().to_string(),
        tokens: vec![],
        msg: "Interpreter panicked".to_string(),
        id: line!(),
        kind
      }
    })?
  }

  pub fn compile(&self) -> MResult<Vec<u8>> {
    let plan_brrw = self.plan.borrow();
    let mut ctx = CompileCtx::new();
    for step in plan_brrw.iter() {
      step.compile(&mut ctx)?;
    }
    let header_size = ByteCodeHeader::HEADER_SIZE as u64;
    let feat_bytes_len: u64 = 4 + (ctx.features.len() as u64) * 8;
    let const_tbl_len: u64 = (ctx.const_entries.len() as u64) * ConstEntry::byte_len();
    let const_blob_len: u64 = ctx.const_blob.len() as u64;
    let instr_bytes_len: u64 = ctx.instrs.iter().map(|i| i.byte_len()).sum();

    let mut offset = header_size;                           // bytes in header
    let feature_off = offset; offset += feat_bytes_len;     // offset to feature section
    let const_tbl_off = offset; offset += const_tbl_len;    // offset to constant table
    let const_blob_off = offset; offset += const_blob_len;  // offset to constant blob
    let instr_off = offset;                                 // offset to instruction stream
    offset += instr_bytes_len;                              
    let feat_off = feature_off;                             // offset to feature section              
    let feat_len = feat_bytes_len;                          // bytes in feature section                
    
    let file_len_before_trailer = offset;
    let trailer_len = 4u64;
    let full_file_len = file_len_before_trailer + trailer_len;

    // The header!
    let header = ByteCodeHeader {
      magic: *b"MECH",
      version: 1,             
      mech_ver: parse_version_to_u16(env!("CARGO_PKG_VERSION")).unwrap(),
      flags: 0,
      reg_count: ctx.next_reg,
      instr_count: ctx.instrs.len() as u32,
      feature_count: ctx.features.len() as u32,
      feature_off,

      const_count: ctx.const_entries.len() as u32,
      const_tbl_off,
      const_tbl_len,
      const_blob_off,
      const_blob_len,

      instr_off,
      instr_len: instr_bytes_len,

      feat_off,
      feat_len,

      reserved: 0,
    };
    
    let mut buf = Cursor::new(Vec::<u8>::with_capacity(full_file_len as usize));

    // 1. Write the header
    header.write_to(&mut buf)?;

    // 2. Write the feature section
    buf.write_u32::<LittleEndian>(ctx.features.len() as u32)?;
    for f in &ctx.features {
      buf.write_u64::<LittleEndian>(f.as_u64())?;
    }

    // 3. write const table entries
    for entry in &ctx.const_entries {
      entry.write_to(&mut buf)?;
    }

    // 4. write const blob
    if !ctx.const_blob.is_empty() {
      buf.write_all(&ctx.const_blob)?;
    }

    // 5. write instructions. This is where the action is!
    for ins in &ctx.instrs {
      ins.write_to(&mut buf)?;
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