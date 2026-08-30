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

#[cfg(feature = "pretty_print")]
impl PrettyPrint for SymbolTable {
    fn pretty_print(&self) -> String {
        let mut builder = Builder::default();
        let dict_brrw = self.dictionary.borrow();
        for (k, v) in &self.symbols {
            let name = dict_brrw.get(k).unwrap_or(&"??".to_string()).clone();
            builder.push_record(vec![format!("\n{}\n{:#?}\n", name, v,)])
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
