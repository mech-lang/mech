use crate::*;

// Symbol Table
// ----------------------------------------------------------------------------

pub type SymbolTableRef = Ref<SymbolTable>;

#[derive(Clone, Debug)]
pub struct SymbolTableSnapshot {
    symbols: HashMap<u64, ValueCell>,
    mutable_variables: HashMap<u64, ValueCell>,
    dictionary: Ref<Dictionary>,
    dictionary_contents: Dictionary,
}

impl PartialEq for SymbolTableSnapshot {
    fn eq(&self, other: &Self) -> bool {
        fn same_cells(left: &HashMap<u64, ValueCell>, right: &HashMap<u64, ValueCell>) -> bool {
            left.len() == right.len()
                && left
                    .iter()
                    .all(|(key, cell)| right.get(key).is_some_and(|other| cell.same_cell(other)))
        }

        same_cells(&self.symbols, &other.symbols)
            && same_cells(&self.mutable_variables, &other.mutable_variables)
            && self.dictionary.same_handle(&other.dictionary)
            && self.dictionary_contents == other.dictionary_contents
    }
}

impl Eq for SymbolTableSnapshot {}

#[derive(Clone, Debug)]
pub struct SymbolTable {
    pub symbols: HashMap<u64, ValueCell>,
    pub mutable_variables: HashMap<u64, ValueCell>,
    pub dictionary: Ref<Dictionary>,
}

impl SymbolTable {
    pub fn snapshot(&self) -> SymbolTableSnapshot {
        SymbolTableSnapshot {
            symbols: self.symbols.clone(),
            mutable_variables: self.mutable_variables.clone(),
            dictionary: self.dictionary.clone(),
            dictionary_contents: self.dictionary.borrow().clone(),
        }
    }

    pub fn preflight_restore(&self, snapshot: &SymbolTableSnapshot) -> MResult<()> {
        snapshot
            .dictionary
            .try_borrow_mut()
            .map(|_| ())
            .map_err(|_| {
                MechError::new(
                    ValueStateBorrowConflict {
                        phase: "restore-before",
                        type_name: core::any::type_name::<Dictionary>(),
                    },
                    None,
                )
                .with_compiler_loc()
            })
    }

    pub fn apply_restore(&mut self, snapshot: &SymbolTableSnapshot) {
        self.symbols = snapshot.symbols.clone();
        self.mutable_variables = snapshot.mutable_variables.clone();
        self.dictionary = snapshot.dictionary.clone();
        *self.dictionary.borrow_mut() = snapshot.dictionary_contents.clone();
    }

    pub fn restore(&mut self, snapshot: SymbolTableSnapshot) {
        self.apply_restore(&snapshot);
    }

    pub fn new() -> SymbolTable {
        Self {
            symbols: HashMap::new(),
            mutable_variables: HashMap::new(),
            dictionary: Ref::new(HashMap::new()),
        }
    }

    pub fn get_symbol_name_by_id(&self, id: u64) -> Option<String> {
        self.dictionary.borrow().get(&id).cloned()
    }

    pub fn get_mutable(&self, key: u64) -> Option<ValueCell> {
        self.mutable_variables.get(&key).cloned()
    }

    pub fn get(&self, key: u64) -> Option<ValueCell> {
        self.symbols.get(&key).cloned()
    }

    pub fn contains(&self, key: u64) -> bool {
        self.symbols.contains_key(&key)
    }

    pub fn insert(&mut self, key: u64, value: LegacyValue, mutable: bool) -> ValueCell {
        self.insert_cell(key, ValueCell::new(value), mutable)
    }

    pub fn insert_cell(&mut self, key: u64, cell: ValueCell, mutable: bool) -> ValueCell {
        self.symbols.insert(key, cell.clone());
        if mutable {
            self.mutable_variables.insert(key, cell.clone());
        } else {
            self.mutable_variables.remove(&key);
        }
        cell.clone()
    }
}

#[cfg(test)]
mod snapshot_tests {
    use super::*;

    #[test]
    fn symbol_table_cells_preserve_identity_and_mutability() {
        let mut table = SymbolTable::new();
        let outer = hash_str("outer");
        let stored = table.insert(outer, LegacyValue::Index(Ref::new(1)), true);

        assert!(stored.same_cell(&table.get(outer).unwrap()));
        assert!(stored.same_cell(&table.get_mutable(outer).unwrap()));

        let immutable = hash_str("immutable");
        let immutable_cell = table.insert(immutable, LegacyValue::Index(Ref::new(2)), false);
        assert!(immutable_cell.same_cell(&table.get(immutable).unwrap()));
        assert!(table.get_mutable(immutable).is_none());
    }

    #[test]
    fn replacing_equal_payloads_replaces_cell_identity() {
        let mut table = SymbolTable::new();
        let key = hash_str("value");
        let first = table.insert(key, LegacyValue::Index(Ref::new(1)), false);
        let second = ValueCell::new(LegacyValue::Index(Ref::new(1)));

        let stored = table.insert_cell(key, second.clone(), false);
        assert_eq!(*first.borrow(), *second.borrow());
        assert!(!first.same_cell(&second));
        assert!(stored.same_cell(&second));
        assert!(table.get(key).unwrap().same_cell(&second));
    }

    #[test]
    fn symbol_table_snapshot_restores_cells_and_dictionary_identity() {
        let mut table = SymbolTable::new();
        let outer = hash_str("outer");
        let temporary = hash_str("temporary");
        let outer_cell = table.insert(outer, LegacyValue::Index(Ref::new(1)), true);
        table
            .dictionary
            .borrow_mut()
            .insert(outer, "outer".to_string());
        let original_dictionary = table.dictionary.clone();
        let original_snapshot = table.snapshot();

        table.insert(outer, LegacyValue::Index(Ref::new(2)), false);
        table.insert(temporary, LegacyValue::Index(Ref::new(3)), false);
        table
            .dictionary
            .borrow_mut()
            .insert(outer, "changed".to_string());
        table
            .dictionary
            .borrow_mut()
            .insert(temporary, "temporary".to_string());
        table.dictionary = Ref::new(Dictionary::new());
        assert_ne!(table.snapshot(), original_snapshot);

        table.restore(original_snapshot.clone());
        assert_eq!(table.snapshot(), original_snapshot);
        assert!(!table.contains(temporary));
        assert!(table.get_mutable(outer).is_some());
        assert!(table.get(outer).unwrap().same_cell(&outer_cell));
        assert!(table.dictionary.same_handle(&original_dictionary));
        assert_eq!(table.get_symbol_name_by_id(outer).as_deref(), Some("outer"));
    }
}

#[cfg(feature = "pretty_print")]
impl PrettyPrint for SymbolTable {
    fn pretty_print(&self) -> String {
        let mut builder = Builder::default();
        let dict_brrw = self.dictionary.borrow();
        for (k, v) in &self.symbols {
            let name = dict_brrw.get(k).unwrap_or(&"??".to_string()).clone();
            let v_brrw = v.borrow();
            builder.push_record(vec![format!(
                "\n{} : {}\n{}\n",
                name,
                v_brrw.kind(),
                v_brrw.pretty_print()
            )])
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
