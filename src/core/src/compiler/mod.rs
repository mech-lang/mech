use crate::*;
use byteorder::{LittleEndian, WriteBytesExt, ReadBytesExt};
use std::io::Cursor;
#[cfg(not(feature = "no_std"))]
use std::collections::HashSet;
#[cfg(feature = "no_std")]
use hashbrown::HashSet;

pub mod sections;
pub mod constants;
pub mod context;
#[cfg(feature = "symbol_table")]
pub mod symbol_table;
#[cfg(feature = "program")]
pub mod program;

pub use self::sections::*;
pub use self::constants::*;
pub use self::context::*;
#[cfg(feature = "symbol_table")]
pub use self::symbol_table::*;
#[cfg(feature = "program")]
pub use self::program::*;

// Program State
// ----------------------------------------------------------------------------

pub type Dictionary = HashMap<u64,String>;
pub type KindTable = HashMap<u64, ValueKind>;
#[cfg(feature = "enum")]
pub type EnumTable = HashMap<u64, MechEnum>;
#[cfg(all(feature = "invariant_define", feature = "symbol_table"))]
pub type InvariantTable = HashMap<u64, (String, ValRef)>;
#[cfg(feature = "invariant_define")]
pub type InvariantExpressionTable = HashMap<u64, String>;
#[cfg(feature = "invariant_define")]
#[derive(Clone, Debug)]
pub struct InvariantEvaluation {
  pub reason: String,
  pub evaluated_kind: String,
  pub actual: String,
  pub expected: String,
}
#[cfg(feature = "invariant_define")]
pub type InvariantEvaluationTable = HashMap<u64, InvariantEvaluation>;
#[cfg(feature = "invariant_define")]
#[derive(Clone, Debug)]
pub struct InvariantViolation {
  pub id: u64,
  pub error: MechError,
}

pub struct ProgramState {
  #[cfg(feature = "symbol_table")]
  pub symbol_table: SymbolTableRef,
  #[cfg(feature = "symbol_table")]
  pub environment: Option<SymbolTableRef>,
  #[cfg(feature = "functions")]
  pub functions: FunctionsRef,
  #[cfg(feature = "functions")]
  pub plan: Plan,
  pub kinds: KindTable,
  #[cfg(feature = "enum")]
  pub enums: EnumTable,
  #[cfg(all(feature = "invariant_define", feature = "symbol_table"))]
  pub invariants: InvariantTable,
  #[cfg(feature = "invariant_define")]
  pub invariant_violations: Vec<InvariantViolation>,
  #[cfg(feature = "invariant_define")]
  pub invariant_expressions: InvariantExpressionTable,
  #[cfg(feature = "invariant_define")]
  pub invariant_evaluations: InvariantEvaluationTable,
  pub dictionary: Ref<Dictionary>,
}

impl Clone for ProgramState {
  fn clone(&self) -> Self {
    ProgramState {
      #[cfg(feature = "symbol_table")]
      symbol_table: self.symbol_table.clone(),
      #[cfg(feature = "symbol_table")]
      environment: self.environment.clone(),
      #[cfg(feature = "functions")]
      functions: self.functions.clone(),
      #[cfg(feature = "functions")]
      plan: self.plan.clone(),
      kinds: self.kinds.clone(),
      #[cfg(feature = "enum")]
      enums: self.enums.clone(),
      #[cfg(all(feature = "invariant_define", feature = "symbol_table"))]
      invariants: self.invariants.clone(),
      #[cfg(feature = "invariant_define")]
      invariant_violations: self.invariant_violations.clone(),
      #[cfg(feature = "invariant_define")]
      invariant_expressions: self.invariant_expressions.clone(),
      #[cfg(feature = "invariant_define")]
      invariant_evaluations: self.invariant_evaluations.clone(),
      dictionary: self.dictionary.clone(),
    }
  }
}

impl ProgramState {
  pub fn new() -> ProgramState {
    ProgramState {
      #[cfg(feature = "symbol_table")]
      symbol_table: Ref::new(SymbolTable::new()),
      #[cfg(feature = "symbol_table")]
      environment: None,
      #[cfg(feature = "functions")]
      functions: Ref::new(Functions::new()),
      #[cfg(feature = "functions")]
      plan: Plan::new(),
      kinds: KindTable::new(),
      #[cfg(feature = "enum")]
      enums: EnumTable::new(),
      #[cfg(all(feature = "invariant_define", feature = "symbol_table"))]
      invariants: InvariantTable::new(),
      #[cfg(feature = "invariant_define")]
      invariant_violations: vec![],
      #[cfg(feature = "invariant_define")]
      invariant_expressions: InvariantExpressionTable::new(),
      #[cfg(feature = "invariant_define")]
      invariant_evaluations: InvariantEvaluationTable::new(),
      dictionary: Ref::new(Dictionary::new()),
    }
  }

  #[cfg(feature = "pretty_print")]
  pub fn pretty_print(&self) -> String {
    let mut output = String::new();
    output.push_str("Program State:\n");
    #[cfg(feature = "symbol_table")]
    {
      output.push_str("Symbol Table:\n");
      output.push_str(&self.symbol_table.borrow().pretty_print());
    }
    #[cfg(feature = "functions")]
    {
      output.push_str(&self.functions.borrow().pretty_print());
    }
    #[cfg(feature = "functions")]
    {
      output.push_str("Execution Plan:\n");
      for (i, step) in self.plan.borrow().iter().enumerate() {
        output.push_str(&format!("  Step {}: {}\n", i, step.to_string()));
      }
    }
    output
  }

  #[cfg(feature = "symbol_table")]
  pub fn get_symbol(&self, id: u64) -> Option<Ref<Value>> {
    let syms = self.symbol_table.borrow();
    syms.get(id)
  }

  #[cfg(feature = "symbol_table")]
  pub fn get_mutable_symbol(&self, id: u64) -> Option<ValRef> {
    let syms = self.symbol_table.borrow();
    syms.get_mutable(id)
  }

  #[cfg(feature = "symbol_table")]
  pub fn contains_symbol(&self, id: u64) -> bool {
    if let Some(env) = &self.environment {
      let env_brrw = env.borrow();
      if env_brrw.contains(id) {
        true
      } else {
        let syms = self.symbol_table.borrow();
        syms.contains(id)
      }
    } else {
      let syms = self.symbol_table.borrow();
      syms.contains(id)
    }
  }

  #[cfg(feature = "symbol_table")]
  pub fn get_environment(&self) -> Option<SymbolTableRef> {
    self.environment.clone()
  }

  /// Look up symbol in environment first, then in global symbol table
  #[cfg(feature = "symbol_table")]
  pub fn get_env_symbol(&self, id: u64) -> Option<Ref<Value>> {
    if let Some(env) = &self.environment {
      let env_brrw = env.borrow();
      match env_brrw.get(id) {
        Some(val) => Some(val),
        None => {
          let sym_brrw = self.symbol_table.borrow();
          sym_brrw.get(id)
        }
      }
    } else {
      None
    }
  }
      
  #[cfg(feature = "functions")]
  pub fn add_plan_step(&self, step: Box<dyn MechFunction>) {
    let mut plan_brrw = self.plan.borrow_mut();
    plan_brrw.push(step);
  }

  #[cfg(feature = "functions")]
  pub fn insert_function(&self, fxn: FunctionDescriptor) {
    let mut fxns_brrw = self.functions.borrow_mut();
    let id = hash_str(&fxn.name);
    fxns_brrw.functions.insert(id.clone(), fxn.ptr);
    self.dictionary.borrow_mut().insert(id, fxn.name.to_string());
  }

  #[cfg(feature = "symbol_table")]
  pub fn save_symbol(&self, id: u64, name: String, value: Value, mutable: bool) -> ValRef {
    let mut symbols_brrw = self.symbol_table.borrow_mut();
    let val_ref = symbols_brrw.insert(id,value,mutable);
    let mut dict_brrw = symbols_brrw.dictionary.borrow_mut();
    dict_brrw.insert(id,name);
    val_ref
  }

  #[cfg(feature = "symbol_table")]
  pub fn save_env_symbol(&self, id: u64, name: String, value: Value, mutable: bool) -> ValRef {
    if let Some(env) = &self.environment {
      let mut env_brrw = env.borrow_mut();
      let val_ref = env_brrw.insert(id,value,mutable);
      let mut dict_brrw = env_brrw.dictionary.borrow_mut();
      dict_brrw.insert(id,name);
      val_ref
    } else {
      panic!("No environment to save variable into");
    }
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

#[derive(Debug, Clone)]
pub struct InvalidMagicNumberError;

impl MechErrorKind for InvalidMagicNumberError {
  fn name(&self) -> &str { "InvalidMagicNumber" }
  fn message(&self) -> String { "Invalid magic number".to_string() }
}


pub type Register = u32;

pub fn encode_value_kind(ts: &mut TypeSection, vk: &ValueKind) -> (TypeTag, Vec<u8>) {
  let mut b = Vec::new();
  let tag = match vk {
    ValueKind::Kind(kind) => {
      let kind_id = ts.get_or_intern(kind);
      b.write_u32::<LittleEndian>(kind_id).unwrap();
      TypeTag::Kind
    },
    ValueKind::U8 => TypeTag::U8, ValueKind::U16 => TypeTag::U16, ValueKind::U32 => TypeTag::U32,
    ValueKind::U64 => TypeTag::U64, ValueKind::U128 => TypeTag::U128,
    ValueKind::I8 => TypeTag::I8, ValueKind::I16 => TypeTag::I16, ValueKind::I32 => TypeTag::I32,
    ValueKind::I64 => TypeTag::I64, ValueKind::I128 => TypeTag::I128,
    ValueKind::F32 => TypeTag::F32, ValueKind::F64 => TypeTag::F64,
    ValueKind::C64 => TypeTag::C64,
    ValueKind::R64 => TypeTag::R64,
    ValueKind::String => TypeTag::String,
    ValueKind::Bool => TypeTag::Bool,
    ValueKind::Id => TypeTag::Id,
    ValueKind::Index => TypeTag::Index,
    ValueKind::Empty => TypeTag::Empty,
    ValueKind::Any => TypeTag::Any,
    ValueKind::None => TypeTag::None,

    ValueKind::Matrix(elem, dims) => {
      let elem_id = ts.get_or_intern(elem);
      b.write_u32::<LittleEndian>(elem_id).unwrap();
      b.write_u32::<LittleEndian>(dims.len() as u32).unwrap();
      for &d in dims { b.write_u32::<LittleEndian>(d as u32).unwrap(); }
      match &**elem {
        ValueKind::U8 => TypeTag::MatrixU8,
        ValueKind::U16 => TypeTag::MatrixU16,
        ValueKind::U32 => TypeTag::MatrixU32,
        ValueKind::U64 => TypeTag::MatrixU64,
        ValueKind::U128 => TypeTag::MatrixU128,
        ValueKind::I8 => TypeTag::MatrixI8,
        ValueKind::I16 => TypeTag::MatrixI16,
        ValueKind::I32 => TypeTag::MatrixI32,
        ValueKind::I64 => TypeTag::MatrixI64,
        ValueKind::I128 => TypeTag::MatrixI128,
        ValueKind::F32 => TypeTag::MatrixF32,
        ValueKind::F64 => TypeTag::MatrixF64,
        ValueKind::C64 => TypeTag::MatrixC64,
        ValueKind::R64 => TypeTag::MatrixR64,
        ValueKind::String => TypeTag::MatrixString,
        ValueKind::Bool => TypeTag::MatrixBool,
        ValueKind::Index => TypeTag::MatrixIndex,
        _ => panic!("Unsupported matrix element type {:?}", elem),
      }
    }

    ValueKind::Enum(id, name) => {
      b.write_u64::<LittleEndian>(*id).unwrap();
      let name_bytes = name.as_bytes();
      b.write_u32::<LittleEndian>(name_bytes.len() as u32).unwrap();
      b.extend_from_slice(name_bytes);
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

    ValueKind::Atom(id, name) => {
      b.write_u64::<LittleEndian>(*id).unwrap();
      let name_bytes = name.as_bytes();
      b.write_u32::<LittleEndian>(name_bytes.len() as u32).unwrap();
      b.extend_from_slice(name_bytes);
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
