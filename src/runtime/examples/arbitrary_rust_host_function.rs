use std::fmt::Display;
use std::sync::Arc;

use mech_core::{MResult, Ref, Value};

use mech_runtime::{
    BasicCapability, BasicCapabilityKernel, BasicOperation, BasicResource, BasicSubject,
    CapabilityId, DeterministicHostFunction, RuntimeBuilder, TaskRecord,
};

use mech_runtime::arg::host_arg_string;

fn short_text(text: &str) -> String {
    if text.len() <= 18 {
        return text.to_string();
    }

    format!("{}…{}", &text[..8], &text[text.len() - 8..])
}

fn short(id: impl Display) -> String {
    short_text(&id.to_string())
}

fn fmt_value(value: &Value) -> String {
    match value {
        Value::String(text) => {
            format!("String({:?})", short_text(&text.borrow()))
        }
        other => format!("{:?}", other),
    }
}

fn main() -> MResult<()> {
    let mut runtime = RuntimeBuilder::new()
        .capability_kernel(BasicCapabilityKernel::new())
        .host_function(DeterministicHostFunction::new(
            "demo/echo",
            |_context, _args| Ok(Value::String(Ref::new(String::new()))),
            |_context, args| {
                let text = host_arg_string("demo/echo", &args, 0)?;
                Ok(Value::String(Ref::new(format!("rust echoed: {}", text,))))
            },
        ))?
        .host_function(DeterministicHostFunction::new(
            "demo/join",
            |_context, _args| Ok(Value::String(Ref::new(String::new()))),
            |_context, args| {
                let left = host_arg_string("demo/join", &args, 0)?;
                let right = host_arg_string("demo/join", &args, 1)?;
                Ok(Value::String(Ref::new(format!("{} + {}", left, right,))))
            },
        ))?
        .build()?;

    println!("runtime: {}", short(runtime.id()));

    let subject = BasicSubject::new("program:arbitrary-rust-host");

    for (id, name) in [(1, "demo/echo"), (2, "demo/join")] {
        runtime.grant_capability(Arc::new(BasicCapability::new(
            CapabilityId(id),
            &subject,
            &BasicResource::new(format!("host:{}", name)),
            [BasicOperation::new("call")],
        )))?;
    }

    let task = TaskRecord::new(runtime.next_task_id(), "program:arbitrary-rust-host")
        .with_capabilities(vec![CapabilityId(1), CapabilityId(2)]);
    let mut context = runtime.context_for_task(&task)?;

    let value = runtime.run_string_with_context(
        &mut context,
        r#"
      demo/echo("in rust")
    "#,
    )?;

    println!("echo result: {}", fmt_value(&value.to_value()));

    match value.into_value() {
        Value::String(text) => {
            assert_eq!(&*text.borrow(), "rust echoed: in rust");
        }
        other => {
            panic!("expected string result, got {:?}", other);
        }
    }

    let task = TaskRecord::new(runtime.next_task_id(), "program:arbitrary-rust-host")
        .with_capabilities(vec![CapabilityId(1), CapabilityId(2)]);
    let mut context = runtime.context_for_task(&task)?;

    let value = runtime.run_string_with_context(
        &mut context,
        r#"
      demo/join("left", "right")
    "#,
    )?;

    println!("join result: {}", fmt_value(&value.to_value()));

    match value.into_value() {
        Value::String(text) => {
            assert_eq!(&*text.borrow(), "left + right");
        }
        other => {
            panic!("expected string result, got {:?}", other);
        }
    }

    runtime.shutdown()?;

    println!();
    println!("events:");

    for event in runtime.list_events(None)? {
        println!(
            "  #{:03} {:24} {}",
            event.sequence,
            event.name(),
            format!("{:?}", &event.kind),
        );
    }

    Ok(())
}
