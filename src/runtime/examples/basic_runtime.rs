use std::sync::Arc;

use mech_core::{MResult, Value};
use mech_runtime::{
    BasicCapability, BasicCapabilityKernel, BasicOperation, BasicResource, BasicSubject,
    CapabilityId, DeterministicHostFunction, HostCall, InMemoryHostRegistry,
    InMemorySourceResolver, ModuleBuildOptions, RuntimeBuilder, RuntimeConfig, SourceRequest,
    TaskRecord,
};

fn main() -> MResult<()> {
    let mut source_resolver = InMemorySourceResolver::new();
    source_resolver.insert_string("main", "x := 1")?;

    let mut host_registry = InMemoryHostRegistry::new();
    host_registry.insert(DeterministicHostFunction::new(
        "host.empty",
        |_context, _args| Ok(Value::Empty),
        |_context, _args| Ok(Value::Empty),
    ))?;

    let mut runtime = RuntimeBuilder::new()
        .config(RuntimeConfig::default())
        .source_resolver(source_resolver)
        .host_registry(host_registry)
        .capability_kernel(BasicCapabilityKernel::new())
        .build()?;

    println!("runtime: {}", runtime.id());

    let run_result = runtime.run_string("x := 1")?;
    println!("run_string result: {:?}", run_result);

    let version = runtime
        .resolve_and_store_module_source(
            SourceRequest::new("main"),
            ModuleBuildOptions::new(
                env!("CARGO_PKG_VERSION"),
                "mech-current",
                "runtime",
                &[],
                &[],
            ),
        )?
        .expect("expected in-memory source to resolve");

    println!("stored module version: {}", version);

    let actor = runtime.create_actor("actor:example", Some(version), None, Vec::new())?;

    println!("actor: {}", actor);

    let message = runtime.send_message(actor, "ping", b"hello".to_vec())?;

    println!("message: {}", message);

    let popped = runtime
        .pop_message(actor)?
        .expect("expected message in actor mailbox");

    println!(
        "popped message kind={} payload={:?}",
        popped.kind,
        String::from_utf8_lossy(&popped.payload),
    );

    let subject = BasicSubject::new("task:host-example");
    let resource = BasicResource::new("host:host.empty");

    let capability = BasicCapability::new(
        CapabilityId(1),
        &subject,
        &resource,
        [BasicOperation::new("call")],
    );

    runtime.grant_capability(Arc::new(capability))?;

    let task = TaskRecord::new(runtime.next_task_id(), "task:host-example")
        .with_capabilities(vec![CapabilityId(1)]);
    let mut host_context = runtime.context_for_task(&task)?;

    let host_result = runtime
        .call_host_with_context(&mut host_context, HostCall::new("host.empty", Vec::new()))?;

    println!("host call result: {:?}", host_result);

    println!("events:");

    for event in runtime.list_events(None)? {
        println!("  #{:03} {} {:?}", event.sequence, event.name(), event.kind,);
    }

    runtime.shutdown()?;

    Ok(())
}
