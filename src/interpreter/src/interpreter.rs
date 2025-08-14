use crate::*;
use std::rc::Rc;
use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};

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
    let mut interp = Interpreter {
      id,
      symbols: new_ref(SymbolTable::new()),
      plan: new_ref(Vec::new()),
      functions: new_ref(Functions::new()),
      out: Value::Empty,
      sub_interpreters: new_ref(HashMap::new()),
      out_values: new_ref(HashMap::new()),
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
    #[cfg(feature = "combinatorics_n_choose-k")]
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
    self.symbols = new_ref(SymbolTable::new());
    self.plan = new_ref(Vec::new());
    self.functions = new_ref(Functions::new());
    self.out = Value::Empty;
    self.out_values = new_ref(HashMap::new());
    self.sub_interpreters = new_ref(HashMap::new());
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
  
}