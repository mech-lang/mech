use crate::*;
use std::io::Cursor;
#[cfg(not(feature = "no_std"))]
use std::collections::HashSet;
#[cfg(feature = "no_std")]
use hashbrown::HashSet;

#[cfg(any(feature = "compiler", feature = "program"))]
pub mod compiler;
#[cfg(feature = "symbol_table")]
pub mod symbol_table;
#[cfg(feature = "program")]
pub mod program;

#[cfg(any(feature = "compiler", feature = "program"))]
pub use self::compiler::*;
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

pub struct ProgramState {
  #[cfg(feature = "symbol_table")]
  pub symbol_table: SymbolTableRef,
  #[cfg(feature = "functions")]
  pub functions: FunctionsRef,
  #[cfg(feature = "functions")]
  pub plan: Plan,
  pub kinds: KindTable,
  #[cfg(feature = "enum")]
  pub enums: EnumTable,
  pub dictionary: Ref<Dictionary>,
}

impl Clone for ProgramState {
  fn clone(&self) -> Self {
    ProgramState {
      #[cfg(feature = "symbol_table")]
      symbol_table: self.symbol_table.clone(),
      #[cfg(feature = "functions")]
      functions: self.functions.clone(),
      #[cfg(feature = "functions")]
      plan: self.plan.clone(),
      kinds: self.kinds.clone(),
      #[cfg(feature = "enum")]
      enums: self.enums.clone(),
      dictionary: self.dictionary.clone(),
    }
  }
}

impl ProgramState {
  pub fn new() -> ProgramState {
    ProgramState {
      #[cfg(feature = "symbol_table")]
      symbol_table: Ref::new(SymbolTable::new()),
      #[cfg(feature = "functions")]
      functions: Ref::new(Functions::new()),
      #[cfg(feature = "functions")]
      plan: Plan::new(),
      kinds: KindTable::new(),
      #[cfg(feature = "enum")]
      enums: EnumTable::new(),
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

impl MechErrorKind2 for InvalidMagicNumberError {
  fn name(&self) -> &str { "InvalidMagicNumber" }
  fn message(&self) -> String { "Invalid magic number".to_string() }
}