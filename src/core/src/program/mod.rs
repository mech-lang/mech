use crate::*;

#[cfg(feature = "compiler")]
pub mod compiler;
#[cfg(feature = "symbol_table")]
pub mod symbol_table;

#[cfg(feature = "compiler")]
pub use self::compiler::*;
#[cfg(feature = "symbol_table")]
pub use self::symbol_table::*;

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
  pub fn insert_function(&self, fxn: FunctionDefinition) {
    let mut fxns_brrw = self.functions.borrow_mut();
    fxns_brrw.functions.insert(fxn.id, fxn);
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