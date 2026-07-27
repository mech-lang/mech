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
  ModuleBuildOptions,
  SourceRequest,
  ActorTurn,
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
      ModuleBuildOptions::new(
        env!("CARGO_PKG_VERSION"),
        "mech-current",
        "runtime",
        &[],
        &[],
      ),
    )?
    .expect("expected actor behavior to resolve");

  let state_id = runtime.next_object_id();

  runtime.put_object(ObjectRecord::text(
    state_id,
    "actor-state",
    "count=0",
  ))?;

  let subject = BasicSubject::new("actor:context-host");
  let capability_ids = vec![
    CapabilityId(1),
    CapabilityId(2),
    CapabilityId(3),
  ];

  for (id, name) in [
    (CapabilityId(1), "actor/message/kind"),
    (CapabilityId(2), "actor/message/payload"),
    (CapabilityId(3), "actor/state/id"),
  ] {
    runtime.grant_capability(Arc::new(BasicCapability::new(
      id,
      &subject,
      &BasicResource::new(format!("host:{}", name)),
      [BasicOperation::new("call")],
    )))?;
  }

  let actor = runtime.create_actor(
    "actor:context-host",
    Some(actor_version),
    Some(state_id),
    capability_ids,
  )?;

  let message = runtime.send_message(
    actor,
    "increment",
    b"count by 1".to_vec(),
  )?;

  println!("actor: {}", actor);
  println!("state: {}", state_id);
  println!("message: {}", message);

  let actor_record = runtime
    .get_actor(actor)?
    .expect("actor should exist");
  let queued_message = runtime
    .peek_message(actor)?
    .expect("expected actor message");
  let expected_turn = ActorTurn::new(actor_record, queued_message)?;
  let mut context = runtime.context_for_actor_turn(&expected_turn)?;

  runtime.begin_transaction(&mut context)?;

  let turn = runtime
    .next_actor_turn_with_context(&mut context, actor)?
    .expect("expected actor turn");
  assert_eq!(turn, expected_turn);

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
