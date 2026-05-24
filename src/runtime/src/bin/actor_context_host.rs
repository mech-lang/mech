use std::sync::Arc;

use mech_core::MResult;

use mech_runtime::{
  BasicCapability,
  BasicCapabilityKernel,
  BasicOperation,
  BasicResource,
  BasicSubject,
  CapabilityId,
  HostCall,
  InMemoryHostRegistry,
  InMemorySourceResolver,
  ObjectRecord,
  RuntimeBuilder,
  RuntimeContextBuilder,
  SourceRequest,
  register_actor_context_host_functions,
};

fn main() -> MResult<()> {
  let mut host_registry = InMemoryHostRegistry::new();

  register_actor_context_host_functions(&mut host_registry)?;

  let mut source_resolver = InMemorySourceResolver::new();

  source_resolver.insert_string(
    "actor.behavior",
    "y := 2",
  )?;

  let mut runtime = RuntimeBuilder::new()
    .source_resolver(source_resolver)
    .host_registry(host_registry)
    .capability_kernel(BasicCapabilityKernel::new())
    .build()?;

  println!("runtime: {}", runtime.id());

  let actor_version = runtime
    .resolve_and_store_module_source(
      SourceRequest::new("actor.behavior"),
      env!("CARGO_PKG_VERSION"),
      "mech-current",
      &[],
      &[],
    )?
    .expect("expected actor behavior to resolve");

  let state_id = runtime.next_object_id();

  runtime.put_object(ObjectRecord::text(
    state_id,
    "actor-state",
    "count=0",
  ))?;

  let actor = runtime.create_actor(
    "actor:context-host",
    Some(actor_version),
    Some(state_id),
    Vec::new(),
  )?;

  let message = runtime.send_message(
    actor,
    "increment",
    b"count by 1".to_vec(),
  )?;

  println!("actor: {}", actor);
  println!("state: {}", state_id);
  println!("message: {}", message);

  let subject = BasicSubject::new("actor:context-host");

  let capability_kind = BasicCapability::new(
    CapabilityId(1),
    &subject,
    &BasicResource::new("host:actor/message/kind"),
    [BasicOperation::new("call")],
  );

  let capability_payload = BasicCapability::new(
    CapabilityId(2),
    &subject,
    &BasicResource::new("host:actor/message/payload"),
    [BasicOperation::new("call")],
  );

  let capability_state = BasicCapability::new(
    CapabilityId(3),
    &subject,
    &BasicResource::new("host:actor/state/id"),
    [BasicOperation::new("call")],
  );

  runtime.grant_capability(Arc::new(capability_kind))?;
  runtime.grant_capability(Arc::new(capability_payload))?;
  runtime.grant_capability(Arc::new(capability_state))?;

  let mut context = RuntimeContextBuilder::new(runtime.id())
    .subject("actor:context-host")
    .actor(actor)
    .build()?;

  runtime.begin_transaction(&mut context)?;

  let turn = runtime
    .next_actor_turn_with_context(&mut context, actor)?
    .expect("expected actor turn");

  context.bind_actor_turn(&turn);

  let kind = runtime.call_host_with_context(
    &mut context,
    HostCall::new("actor/message/kind", Vec::new()),
  )?;

  let payload = runtime.call_host_with_context(
    &mut context,
    HostCall::new("actor/message/payload", Vec::new()),
  )?;

  let state = runtime.call_host_with_context(
    &mut context,
    HostCall::new("actor/state/id", Vec::new()),
  )?;

  println!("actor/message/kind -> {:?}", kind);
  println!("actor/message/payload -> {:?}", payload);
  println!("actor/state/id -> {:?}", state);

  runtime.commit_runtime_transaction(&mut context)?;

  runtime.shutdown()?;

  println!();
  println!("events:");

  for event in runtime.list_events(None)? {
    println!(
      "  #{:03} {} {:?}",
      event.sequence,
      event.name(),
      event.kind,
    );
  }

  println!();
  println!("transactions:");

  for transaction in runtime.list_transactions(None)? {
    println!(
      "  {} subject={} reads={:?} writes={:?} message_acks={:?} message_sends={:?} task_updates={:?} actor_updates={:?} events={:?}",
      transaction.id,
      transaction.subject,
      transaction.read_set,
      transaction.write_set,
      transaction.message_acks,
      transaction.message_sends,
      transaction.task_updates,
      transaction.actor_updates,
      transaction.events,
    );
  }

  Ok(())
}