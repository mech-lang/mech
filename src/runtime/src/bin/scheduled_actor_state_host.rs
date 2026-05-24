use std::fmt::Display;
use std::sync::Arc;

use mech_core::MResult;

use mech_runtime::{
  register_actor_context_host_functions,
  BasicCapability,
  BasicCapabilityKernel,
  BasicOperation,
  BasicResource,
  BasicSubject,
  CapabilityId,
  HostCallActorBehaviorDriver,
  InMemoryHostRegistry,
  InMemorySourceResolver,
  ObjectRecord,
  RuntimeBuilder,
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
    .actor_behavior_driver(HostCallActorBehaviorDriver::new("count=1"))
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
    "actor:scheduled-services-host",
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

  let subject = BasicSubject::new("actor:scheduled-services-host");

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

  runtime.enqueue_actor(actor)?;

  let outcomes = runtime.run_tick()?;

  println!();
  println!("tick outcomes:");

  for outcome in &outcomes {
    println!("  {:?}", outcome);
  }

  assert_eq!(outcomes.len(), 1, "expected one actor turn outcome");
  assert_eq!(outcomes[0].actor, Some(actor));

  assert!(
    runtime.peek_message(actor)?.is_none(),
    "actor message should be acked after scheduled actor turn commits",
  );

  let actor_after = runtime
    .get_actor(actor)?
    .expect("actor should exist after scheduled turn");

  let updated_state = actor_after
    .state
    .expect("actor should have state after scheduled turn");

  assert_ne!(
    updated_state,
    initial_state,
    "scheduled actor behavior should update actor state pointer",
  );

  let updated_state_object = runtime
    .get_object(updated_state)?
    .expect("updated actor state object should exist");

  assert_eq!(
    updated_state_object.data,
    b"count=1",
    "updated actor state object should contain written state",
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

    let actor_updates: Vec<String> = transaction
      .actor_updates
      .iter()
      .map(|id| short(*id))
      .collect();

    println!("  {}", short(transaction.id));
    println!("    subject:       {}", transaction.subject);
    println!("    reads:         {:?}", reads);
    println!("    writes:        {:?}", writes);
    println!("    message_acks:  {:?}", message_acks);
    println!("    actor_updates: {:?}", actor_updates);
    println!("    events:        {}", transaction.events.len());
  }

  let transaction = transactions
    .iter()
    .find(|transaction| {
      transaction.subject == "actor:scheduled-services-host"
    })
    .expect("expected scheduled actor transaction");

  assert!(
    transaction.read_set.contains(&initial_state),
    "actor.state.get should record initial state read",
  );

  assert!(
    transaction.write_set.contains(&updated_state),
    "actor.state.put should record updated state write",
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