use crate::*;
use std::rc::Rc;
use std::collections::HashMap;

// Interpreter 
// ----------------------------------------------------------------------------

pub struct Interpreter {
  pub id: u64,
  symbols: SymbolTableRef,
  plan: Plan,
  functions: FunctionsRef,
  pub out: Value,
  pub out_values: Ref<HashMap<u64, Value>>,
  pub sub_interpreters: Ref<HashMap<u64, Box<Interpreter>>>,
}

impl Interpreter {
  pub fn new(id: u64) -> Interpreter {
    
    // Preload functions
    let mut fxns = Functions::new();

    // Preload combinatorics functions
    fxns.function_compilers.insert(hash_str("combinatorics/n_choose_k"),Box::new(CombinatoricsNChooseK{}));

    // Preload stats functions
    fxns.function_compilers.insert(hash_str("stats/sum/row"),Box::new(StatsSumRow{}));
    fxns.function_compilers.insert(hash_str("stats/sum/column"),Box::new(StatsSumColumn{}));
    
    // Preload math functions
    fxns.function_compilers.insert(hash_str("math/sin"),Box::new(MathSin{}));
    fxns.function_compilers.insert(hash_str("math/cos"),Box::new(MathCos{}));
    fxns.function_compilers.insert(hash_str("math/atan2"),Box::new(MathAtan2{}));
    fxns.function_compilers.insert(hash_str("math/atan"),Box::new(MathAtan{}));
    fxns.function_compilers.insert(hash_str("math/acos"),Box::new(MathAcos{}));
    fxns.function_compilers.insert(hash_str("math/acosh"),Box::new(MathAcosh{}));
    fxns.function_compilers.insert(hash_str("math/acot"),Box::new(MathAcot{}));
    fxns.function_compilers.insert(hash_str("math/acsc"),Box::new(MathAcsc{}));
    fxns.function_compilers.insert(hash_str("math/asec"),Box::new(MathAsec{}));
    fxns.function_compilers.insert(hash_str("math/asin"),Box::new(MathAsin{}));
    fxns.function_compilers.insert(hash_str("math/sinh"),Box::new(MathSinh{}));
    fxns.function_compilers.insert(hash_str("math/cosh"),Box::new(MathCosh{}));
    fxns.function_compilers.insert(hash_str("math/tanh"),Box::new(MathTanh{}));
    //fxns.function_compilers.insert(hash_str("math/atanh"),Box::new(MathAtanh{}));
    fxns.function_compilers.insert(hash_str("math/cot"),Box::new(MathCot{}));
    fxns.function_compilers.insert(hash_str("math/csc"),Box::new(MathCsc{}));
    fxns.function_compilers.insert(hash_str("math/sec"),Box::new(MathSec{}));
    fxns.function_compilers.insert(hash_str("math/tan"),Box::new(MathTan{}));

    // Preload io functions
    fxns.function_compilers.insert(hash_str("io/print"),Box::new(IoPrint{}));
    fxns.function_compilers.insert(hash_str("io/println"),Box::new(IoPrintln{}));

    // Preload kinds
    fxns.kinds.insert(hash_str("u8"),ValueKind::U8);
    fxns.kinds.insert(hash_str("u16"),ValueKind::U16);
    fxns.kinds.insert(hash_str("u32"),ValueKind::U32);
    fxns.kinds.insert(hash_str("u64"),ValueKind::U64);
    fxns.kinds.insert(hash_str("u128"),ValueKind::U128);
    fxns.kinds.insert(hash_str("i8"),ValueKind::I8);
    fxns.kinds.insert(hash_str("i16"),ValueKind::I16);
    fxns.kinds.insert(hash_str("i32"),ValueKind::I32);
    fxns.kinds.insert(hash_str("i64"),ValueKind::I64);
    fxns.kinds.insert(hash_str("i128"),ValueKind::I128);
    fxns.kinds.insert(hash_str("f32"),ValueKind::F32);
    fxns.kinds.insert(hash_str("f64"),ValueKind::F64);
    fxns.kinds.insert(hash_str("string"),ValueKind::String);
    fxns.kinds.insert(hash_str("bool"),ValueKind::Bool);

    Interpreter {
      id,
      symbols: new_ref(SymbolTable::new()),
      plan: new_ref(Vec::new()),
      functions: new_ref(fxns),
      out: Value::Empty,
      sub_interpreters: new_ref(HashMap::new()),
      out_values: new_ref(HashMap::new()),
    }
  }

  pub fn plan(&self) -> Plan {
    self.plan.clone()
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

  pub fn insert_function(&self, fxn: FunctionDefinition) {
    let mut fxns_brrw = self.functions.borrow_mut();
    fxns_brrw.functions.insert(fxn.id, fxn);
  }

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
    let result = program(tree, &self);
    if let Some(last_step) = self.plan.borrow().last() {
      self.out = last_step.out().clone();
    } else {
      self.out = Value::Empty;
    }
    result
  }
}

//-----------------------------------------------------------------------------

pub fn program(program: &Program, p: &Interpreter) -> MResult<Value> {
  body(&program.body, p)
}