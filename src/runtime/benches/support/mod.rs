use std::sync::{Arc, OnceLock};

use mech_core::{
    FunctionCatalog, FunctionCatalogBuilder, FunctionSpecializer, MResult, MechFunction,
    MechFunctionImpl, Ref, Value,
};
use mech_runtime::RuntimeBuilder;

#[cfg(feature = "compiler")]
use mech_core::{BytecodeCompilerContext, MechFunctionCompiler, Register};

#[allow(dead_code)]
pub fn intrinsic_source_catalog() -> Arc<FunctionCatalog> {
    static CATALOG: OnceLock<Arc<FunctionCatalog>> = OnceLock::new();

    Arc::clone(CATALOG.get_or_init(|| {
        let mut builder = FunctionCatalogBuilder::new();
        mech_engine::install_intrinsic_runtime(&mut builder)
            .expect("engine intrinsic runtime fragment must be valid");
        mech_engine::install_intrinsic_source(&mut builder)
            .expect("engine intrinsic source fragment must be valid");
        Arc::new(
            builder
                .build()
                .expect("engine intrinsic source catalog must be valid"),
        )
    }))
}

#[derive(Debug)]
struct BenchmarkAddFunction {
    lhs: Ref<f64>,
    rhs: Ref<f64>,
    out: Ref<f64>,
}

impl MechFunctionImpl for BenchmarkAddFunction {
    fn solve_result(&self) -> MResult<()> {
        *self.out.borrow_mut() = *self.lhs.borrow() + *self.rhs.borrow();
        Ok(())
    }

    fn out(&self) -> Value {
        Value::F64(self.out.clone())
    }

    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        Ok(self.reactive_output_values())
    }

    fn to_string(&self) -> String {
        "BenchmarkAddFunction".to_string()
    }
}

#[cfg(feature = "compiler")]
impl MechFunctionCompiler for BenchmarkAddFunction {
    fn compile(&self, _: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Ok(0)
    }
}

struct BenchmarkAddSpecializer;

impl FunctionSpecializer for BenchmarkAddSpecializer {
    fn specialize(&self, arguments: &[Value]) -> MResult<Box<dyn MechFunction>> {
        let [Value::F64(lhs), Value::F64(rhs)] = arguments else {
            panic!("benchmark math/add expects two f64 arguments");
        };
        Ok(Box::new(BenchmarkAddFunction {
            lhs: lhs.clone(),
            rhs: rhs.clone(),
            out: Ref::new(*lhs.borrow() + *rhs.borrow()),
        }))
    }
}

fn benchmark_source_catalog() -> Arc<FunctionCatalog> {
    static CATALOG: OnceLock<Arc<FunctionCatalog>> = OnceLock::new();

    Arc::clone(CATALOG.get_or_init(|| {
        let mut builder = FunctionCatalogBuilder::new();
        mech_engine::install_intrinsic_runtime(&mut builder)
            .expect("engine intrinsic runtime fragment must be valid");
        mech_engine::install_intrinsic_source(&mut builder)
            .expect("engine intrinsic source fragment must be valid");
        builder
            .insert_specializer("math/add", Arc::new(BenchmarkAddSpecializer))
            .expect("benchmark math/add catalog entry must be valid");
        Arc::new(
            builder
                .build()
                .expect("benchmark source catalog must be valid"),
        )
    }))
}

pub fn source_runtime_builder() -> RuntimeBuilder {
    RuntimeBuilder::new().function_catalog(benchmark_source_catalog())
}

#[allow(dead_code)]
pub fn source_catalog() -> Arc<FunctionCatalog> {
    benchmark_source_catalog()
}
