use crate::*;

// Symbol Table
// ----------------------------------------------------------------------------

pub type SymbolTableRef= Ref<SymbolTable>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SymbolTableSnapshot {
  symbols: HashMap<u64, ValRef>,
  mutable_variables: HashMap<u64, ValRef>,
  dictionary: Dictionary,
  reverse_lookup: HashMap<*const Ref<Value>, u64>,
}

#[derive(Clone, Debug)]
pub struct SymbolTable {
  pub symbols: HashMap<u64,ValRef>,
  pub mutable_variables: HashMap<u64,ValRef>,
  pub dictionary: Ref<Dictionary>,
  pub reverse_lookup: HashMap<*const Ref<Value>, u64>,
}

impl SymbolTable {
  pub fn snapshot(&self) -> SymbolTableSnapshot {
    SymbolTableSnapshot {
      symbols: self.symbols.clone(),
      mutable_variables: self.mutable_variables.clone(),
      dictionary: self.dictionary.borrow().clone(),
      reverse_lookup: self.reverse_lookup.clone(),
    }
  }

  pub fn restore(&mut self, snapshot: SymbolTableSnapshot) {
    self.symbols = snapshot.symbols;
    self.mutable_variables = snapshot.mutable_variables;
    *self.dictionary.borrow_mut() = snapshot.dictionary;
    self.reverse_lookup = snapshot.reverse_lookup;
  }


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

#[cfg(test)]
mod snapshot_tests {
  use super::*;

  #[test]
  fn symbol_table_snapshot_restores_all_indexes_and_identity() {
    let mut table = SymbolTable::new();
    let outer = hash_str("outer");
    let temporary = hash_str("temporary");
    let outer_ref = table.insert(outer, Value::Index(Ref::new(1)), true);
    table.dictionary.borrow_mut().insert(outer, "outer".to_string());
    let outer_addr = outer_ref.addr();
    let original_snapshot = table.snapshot();

    table.insert(outer, Value::Index(Ref::new(2)), false);
    table.insert(temporary, Value::Index(Ref::new(3)), false);
    table.dictionary.borrow_mut().insert(outer, "changed".to_string());
    table.dictionary.borrow_mut().insert(temporary, "temporary".to_string());
    assert_ne!(table.snapshot(), original_snapshot);

    table.restore(original_snapshot.clone());
    assert_eq!(table.snapshot(), original_snapshot);
    assert!(!table.contains(temporary));
    assert!(table.get_mutable(outer).is_some());
    assert_eq!(table.get(outer).unwrap().addr(), outer_addr);
    assert_eq!(table.get_symbol_name_by_id(outer).as_deref(), Some("outer"));
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
