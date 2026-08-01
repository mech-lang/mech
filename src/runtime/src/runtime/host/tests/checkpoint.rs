use crate::runtime::host::RuntimeHostNativeFunction;
use mech_core::{Ref, Value};
use mech_engine::{MechProgram, MechProgramConfig};

#[test]
fn runtime_host_native_function_output_round_trips_through_program_checkpoint() {
    let mut program = MechProgram::new(MechProgramConfig::default());
    let plan = program.interpreter().plan();
    let value = Ref::new(Value::Empty);
    let value_address = value.addr();
    plan.add_function(Box::new(RuntimeHostNativeFunction {
        name: "test/host".to_string(),
        host_name: "test/host".to_string(),
        arguments: Vec::new(),
        value: value.clone(),
    }));
    let checkpoint = program.checkpoint().unwrap();
    let replacement = Ref::new(Value::Index(Ref::new(99)));
    *value.borrow_mut() = Value::MutableReference(replacement);

    program.restore(checkpoint).unwrap();

    assert_eq!(value.addr(), value_address);
    assert_eq!(*value.borrow(), Value::Empty);
    assert!(program.checkpoint().is_ok());
}
