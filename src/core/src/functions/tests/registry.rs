use super::super::{
    FunctionArgs, FunctionCompilerDescriptor, FunctionDescriptor, GuardFunctionSafety,
    MechFunction, ModuleItemDescriptor, NativeFunctionCompiler, StaticNativeFunctionCompiler,
};
use super::support::PureStaticTestCompiler;
#[cfg(feature = "f64")]
use super::support::scalar;
use crate::{MResult, Value};

fn assert_send_sync<T: Send + Sync>() {}

struct DefaultTestCompiler;

impl NativeFunctionCompiler for DefaultTestCompiler {
    fn compile(&self, _arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
        unreachable!("safety metadata test must not compile the function")
    }
}

static PURE_STATIC_TEST_COMPILER: PureStaticTestCompiler = PureStaticTestCompiler;

#[test]
fn native_compiler_descriptors_are_send_and_sync_without_manual_promises() {
    assert_send_sync::<FunctionDescriptor>();
    assert_send_sync::<FunctionCompilerDescriptor>();
    assert_send_sync::<ModuleItemDescriptor>();
    assert_send_sync::<StaticNativeFunctionCompiler>();
}

#[test]
fn native_compiler_guard_safety_defaults_to_unsupported() {
    let compiler = DefaultTestCompiler;

    assert_eq!(compiler.guard_safety(), GuardFunctionSafety::Unsupported);
}

#[test]
fn static_native_compiler_preserves_guard_safety_metadata() {
    let compiler = StaticNativeFunctionCompiler::new(&PURE_STATIC_TEST_COMPILER);

    assert_eq!(compiler.guard_safety(), GuardFunctionSafety::PureStatic);
}

#[cfg(feature = "f64")]
#[test]
fn function_args_returns_only_inputs() {
    let (out, _) = scalar(0.0);
    let (a, _) = scalar(1.0);
    let (b, _) = scalar(2.0);
    let (c, _) = scalar(3.0);
    let (d, _) = scalar(4.0);

    assert_eq!(
        FunctionArgs::Nullary(out.clone()).input_values(),
        Vec::<Value>::new()
    );
    assert_eq!(
        FunctionArgs::Unary(out.clone(), a.clone()).input_values(),
        vec![a.clone()]
    );
    assert_eq!(
        FunctionArgs::Binary(out.clone(), a.clone(), b.clone()).input_values(),
        vec![a.clone(), b.clone()],
    );
    assert_eq!(
        FunctionArgs::Ternary(out.clone(), a.clone(), b.clone(), c.clone()).input_values(),
        vec![a.clone(), b.clone(), c.clone()],
    );
    assert_eq!(
        FunctionArgs::Quaternary(out.clone(), a.clone(), b.clone(), c.clone(), d.clone(),)
            .input_values(),
        vec![a.clone(), b.clone(), c.clone(), d.clone()],
    );
    assert_eq!(
        FunctionArgs::Variadic(out, vec![a.clone(), b.clone(), c.clone(), d.clone()])
            .input_values(),
        vec![a, b, c, d],
    );
}
