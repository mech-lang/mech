use crate::*;
use core::ops::Range;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProgramComputeRegion {
    pub name: String,
    pub placement: ComputePlacement,
    pub plan_nodes: Range<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComputeRegionNameConflictError {
    pub name: String,
}

impl MechErrorKind for ComputeRegionNameConflictError {
    fn name(&self) -> &str {
        "ComputeRegionNameConflict"
    }

    fn message(&self) -> String {
        format!("Compute region `{}` is defined more than once.", self.name)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmptyComputeRegionError {
    pub name: String,
}

impl MechErrorKind for EmptyComputeRegionError {
    fn name(&self) -> &str {
        "EmptyComputeRegion"
    }

    fn message(&self) -> String {
        format!(
            "Compute region `{}` did not produce any executable plan nodes.",
            self.name,
        )
    }
}

pub struct ProgramState {
    #[cfg(feature = "symbol_table")]
    pub symbol_table: SymbolTableRef,
    #[cfg(feature = "symbol_table")]
    pub environment: Option<SymbolTableRef>,
    #[cfg(feature = "functions")]
    pub function_environment: FunctionEnvironment,
    #[cfg(feature = "functions")]
    pub function_extensions: FunctionExtensions,
    #[cfg(feature = "functions")]
    pub user_functions: UserFunctionTable,
    #[cfg(feature = "functions")]
    pub plan: Plan,
    #[cfg(feature = "functions")]
    pub compute_regions: Vec<ProgramComputeRegion>,
    pub kinds: KindTable,
    #[cfg(feature = "enum")]
    pub enums: EnumTable,
    #[cfg(feature = "invariant_define")]
    pub integrity_constraints: IntegrityConstraintTable,
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
            function_environment: self.function_environment.clone(),
            #[cfg(feature = "functions")]
            function_extensions: self.function_extensions.clone(),
            #[cfg(feature = "functions")]
            user_functions: self.user_functions.clone(),
            #[cfg(feature = "functions")]
            plan: self.plan.clone(),
            #[cfg(feature = "functions")]
            compute_regions: self.compute_regions.clone(),
            kinds: self.kinds.clone(),
            #[cfg(feature = "enum")]
            enums: self.enums.clone(),
            #[cfg(feature = "invariant_define")]
            integrity_constraints: self.integrity_constraints.clone(),
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
            function_environment: FunctionEnvironment::default(),
            #[cfg(feature = "functions")]
            function_extensions: FunctionExtensions::default(),
            #[cfg(feature = "functions")]
            user_functions: UserFunctionTable::default(),
            #[cfg(feature = "functions")]
            plan: Plan::new(),
            #[cfg(feature = "functions")]
            compute_regions: Vec::new(),
            kinds: KindTable::default(),
            #[cfg(feature = "enum")]
            enums: EnumTable::new(),
            #[cfg(feature = "invariant_define")]
            integrity_constraints: IntegrityConstraintTable::new(),
            dictionary: Ref::new(Dictionary::default()),
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
            output.push_str("Execution Plan:\n");
            for (i, step) in self.plan.borrow().iter().enumerate() {
                output.push_str(&format!("  Step {}: {}\n", i, step.to_string()));
            }
        }
        output
    }

    #[cfg(feature = "symbol_table")]
    pub fn get_symbol(&self, id: u64) -> Option<Ref<LegacyValue>> {
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

    /// Look up symbol in environment first, then in global symbol table.
    #[cfg(feature = "symbol_table")]
    pub fn get_env_symbol(&self, id: u64) -> Option<Ref<LegacyValue>> {
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

    #[cfg(feature = "symbol_table")]
    pub fn save_symbol(&self, id: u64, name: String, value: LegacyValue, mutable: bool) -> ValRef {
        let mut symbols_brrw = self.symbol_table.borrow_mut();
        let val_ref = symbols_brrw.insert(id, value, mutable);
        let mut dict_brrw = symbols_brrw.dictionary.borrow_mut();
        dict_brrw.insert(id, name);
        val_ref
    }

    #[cfg(feature = "symbol_table")]
    pub fn save_env_symbol(
        &self,
        id: u64,
        name: String,
        value: LegacyValue,
        mutable: bool,
    ) -> ValRef {
        if let Some(env) = &self.environment {
            let mut env_brrw = env.borrow_mut();
            let val_ref = env_brrw.insert(id, value, mutable);
            let mut dict_brrw = env_brrw.dictionary.borrow_mut();
            dict_brrw.insert(id, name);
            val_ref
        } else {
            panic!("No environment to save variable into");
        }
    }
}
