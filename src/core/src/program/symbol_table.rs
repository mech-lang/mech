use crate::*;

// Symbol Table
// ----------------------------------------------------------------------------

pub type SymbolTableRef= Ref<SymbolTable>;

#[derive(Clone, Debug)]
pub struct SymbolTable {
  pub symbols: HashMap<u64,ValRef>,
  pub mutable_variables: HashMap<u64,ValRef>,
  pub dictionary: Ref<Dictionary>,
  pub reverse_lookup: HashMap<*const Ref<Value>, u64>,
}

impl SymbolTable {

  pub fn new() -> SymbolTable {
    Self {
      symbols: HashMap::new(),
      mutable_variables: HashMap::new(),
      dictionary: Ref::new(HashMap::new()),
      reverse_lookup: HashMap::new(),
    }
  }

  pub fn get_symbol_name_by_id(&self, id: u64) -> Option<String> {
    self.dictionary.borrow().get(&id).cloned()
  }

  pub fn get_mutable(&self, key: u64) -> Option<ValRef> {
    self.mutable_variables.get(&key).cloned()
  }

  pub fn get(&self, key: u64) -> Option<ValRef> {
    self.symbols.get(&key).cloned()
  }

  pub fn contains(&self, key: u64) -> bool {
    self.symbols.contains_key(&key)
  }

  pub fn value_cell(&self, id: u64) -> Option<ValRef> {
    self.symbols.get(&id).cloned()
  }

  pub fn update_existing(&self, id: u64, value: Value) -> MResult<bool> {
    let Some(cell) = self.symbols.get(&id) else {
      return Err(MechError::new(SymbolTableUpdateMissing { id }, None));
    };
    {
      let current = cell.borrow().clone();
      if let Value::MutableReference(reference) = current {
        *reference.borrow_mut() = value;
        return Ok(true);
      }
    }
    *cell.borrow_mut() = value;
    Ok(true)
  }

  pub fn insert(&mut self, key: u64, value: Value, mutable: bool) -> ValRef {
    let cell = Ref::new(value);
    self.reverse_lookup.insert(&cell, key);
    let old = self.symbols.insert(key,cell.clone());
    if mutable {
      self.mutable_variables.insert(key,cell.clone());
    }
    cell.clone()
  }

}

#[cfg(feature = "pretty_print")]
impl PrettyPrint for SymbolTable {
  fn pretty_print(&self) -> String {
    let mut builder = Builder::default();
    let dict_brrw = self.dictionary.borrow();
    for (k,v) in &self.symbols {
      let name = dict_brrw.get(k).unwrap_or(&"??".to_string()).clone();
      let v_brrw = v.borrow();
      builder.push_record(vec![format!("\n{} : {}\n{}\n",name, v_brrw.kind(), v_brrw.pretty_print())])
    }
    if self.symbols.is_empty() {
      builder.push_record(vec!["".to_string()]);
    }
    let mut table = builder.build();
    let table_style = Style::empty()
    .top(' ')
    .left(' ')
    .right(' ')
    .bottom(' ')
    .vertical(' ')
    .horizontal('·')
    .intersection_bottom(' ')
    .corner_top_left(' ')
    .corner_top_right(' ')
    .corner_bottom_left(' ')
    .corner_bottom_right(' ');
    table.with(table_style);
    format!("{table}")
  }
}
#[derive(Debug, Clone)]
pub struct SymbolTableUpdateMissing { pub id: u64 }
impl MechErrorKind for SymbolTableUpdateMissing {
  fn name(&self) -> &str { "SymbolTableUpdateMissing" }
  fn message(&self) -> String { format!("symbol table has no existing value cell for id {}", self.id) }
}