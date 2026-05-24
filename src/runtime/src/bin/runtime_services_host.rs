use std::fmt::Display;
use std::sync::Arc;

use mech_core::{MResult, Value, Ref};

use mech_runtime::{
  register_actor_context_host_functions,
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
};

fn short(id: impl Display) -> String {
  let id = id.to_string();

  if id.len() <= 12 {
    return id;
  }

  format!("{}…{}", &id[..6], &id[id.len() - 6..])
}

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

  println!("runtime: {}", short(runtime.id()));

  let actor_version = runtime
    .resolve_and_store_module_source(
      SourceRequest::new("actor.behavior"),
      env!("CARGO_PKG_VERSION"),
      "mech-current",
      &[],
      &[],
    )?
    .expect("expected actor behavior to resolve");

  let initial_state = runtime.next_object_id();

  runtime.put_object(ObjectRecord::text(
    initial_state,
    "actor-state",
    "count=0",
  ))?;

  let actor = runtime.create_actor(
    "actor:services-host",
    Some(actor_version),
    Some(initial_state),
    Vec::new(),
  )?;

  let message = runtime.send_message(
    actor,
    "increment",
    b"count by 1".to_vec(),
  )?;

  println!("actor: {}", short(actor));
  println!("initial state: {}", short(initial_state));
  println!("message: {}", short(message));

  let subject = BasicSubject::new("actor:services-host");

  for (id, name) in [
    (1, "actor.message.kind"),
    (2, "actor.message.payload"),
    (3, "actor.state.id"),
    (4, "actor.state.get"),
    (5, "actor.state.put"),
  ] {
    runtime.grant_capability(Arc::new(BasicCapability::new(
      CapabilityId(id),
      &subject,
      &BasicResource::new(format!("host:{}", name)),
      [BasicOperation::new("call")],
    )))?;
  }

  let mut context = RuntimeContextBuilder::new(runtime.id())
    .subject("actor:services-host")
    .actor(actor)
    .build()?;

  runtime.begin_transaction(&mut context)?;

  let turn = runtime
    .next_actor_turn_with_context(&mut context, actor)?
    .expect("expected actor turn");

  context.bind_actor_turn(&turn);

  let kind = runtime.call_host_with_context(
    &mut context,
    HostCall::new("actor.message.kind", Vec::new()),
  )?;

  let payload = runtime.call_host_with_context(
    &mut context,
    HostCall::new("actor.message.payload", Vec::new()),
  )?;

  let state_id = runtime.call_host_with_context(
    &mut context,
    HostCall::new("actor.state.id", Vec::new()),
  )?;

  let old_state = runtime.call_host_with_context(
    &mut context,
    HostCall::new("actor.state.get", Vec::new()),
  )?;

  let new_state = runtime.call_host_with_context(
    &mut context,
    HostCall::new(
      "actor.state.put",
      vec![Value::String(Ref::new("count=1".to_string()))],
    ),
  )?;

  println!();
  println!("host calls:");
  println!("  actor.message.kind    -> {:?}", kind);
  println!("  actor.message.payload -> {:?}", payload);
  println!("  actor.state.id        -> {:?}", state_id);
  println!("  actor.state.get       -> {:?}", old_state);
  println!("  actor.state.put       -> {:?}", new_state);

  assert!(
    runtime.peek_message(actor)?.is_some(),
    "message ack should not be applied before commit",
  );

  let committed = runtime.commit_runtime_transaction(&mut context)?;

  println!();
  println!("transaction committed: {}", short(committed));

  assert!(
    runtime.peek_message(actor)?.is_none(),
    "message ack should be applied after commit",
  );

  let actor_after = runtime
    .get_actor(actor)?
    .expect("actor should exist after commit");

  let updated_state = actor_after
    .state
    .expect("actor should have state after commit");

  assert_ne!(
    updated_state,
    initial_state,
    "actor.state.put should update actor state pointer",
  );

  let updated_state_object = runtime
    .get_object(updated_state)?
    .expect("updated state object should exist after commit");

  assert_eq!(
    updated_state_object.data,
    b"count=1",
    "updated state object should contain written state",
  );

  runtime.shutdown()?;

  println!();
  println!("transactions:");

  let transactions = runtime.list_transactions(None)?;

  for transaction in &transactions {
    let reads: Vec<String> = transaction
      .read_set
      .iter()
      .map(|id| short(*id))
      .collect();

    let writes: Vec<String> = transaction
      .write_set
      .iter()
      .map(|id| short(*id))
      .collect();

    let message_acks: Vec<String> = transaction
      .message_acks
      .iter()
      .map(|id| short(*id))
      .collect();

    let message_sends: Vec<String> = transaction
      .message_sends
      .iter()
      .map(|id| short(*id))
      .collect();

    let task_updates: Vec<String> = transaction
      .task_updates
      .iter()
      .map(|id| short(*id))
      .collect();

    let actor_updates: Vec<String> = transaction
      .actor_updates
      .iter()
      .map(|id| short(*id))
      .collect();

    let events: Vec<String> = transaction
      .events
      .iter()
      .map(|id| short(*id))
      .collect();

    println!("  {}", short(transaction.id));
    println!("    subject:       {}", transaction.subject);
    println!("    reads:         {:?}", reads);
    println!("    writes:        {:?}", writes);
    println!("    message_acks:  {:?}", message_acks);
    println!("    message_sends: {:?}", message_sends);
    println!("    task_updates:  {:?}", task_updates);
    println!("    actor_updates: {:?}", actor_updates);
    println!("    events:        {:?}", events);
  }

  let transaction = transactions
    .iter()
    .find(|transaction| transaction.subject == "actor:services-host")
    .expect("expected actor services transaction");

  assert!(
    transaction.read_set.contains(&initial_state),
    "actor.state.get should record a read of the initial actor state",
  );

  assert!(
    transaction.write_set.contains(&updated_state),
    "actor.state.put should record a write of the updated actor state",
  );

  assert!(
    transaction.message_acks.contains(&message),
    "actor turn should record message ack",
  );

  assert!(
    transaction.actor_updates.contains(&actor),
    "actor.state.put should record actor update",
  );

  println!();
  println!("events:");

  for event in runtime.list_events(None)? {
    println!(
      "  #{:03} {:24} {:?}",
      event.sequence,
      event.name(),
      event.kind,
    );
  }

  Ok(())
}