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
  InMemoryHostRegistry,
  InMemorySourceResolver,
  ObjectRecord,
  RuntimeBuilder,
  SourceRequest,
};

fn short_text(text: &str) -> String {
  if text.len() <= 18 {
    return text.to_string();
  }

  format!("{}…{}", &text[..8], &text[text.len() - 8..])
}

fn short(id: impl Display) -> String {
  short_text(&id.to_string())
}

fn main() -> MResult<()> {
  let mut host_registry = InMemoryHostRegistry::new();

  register_actor_context_host_functions(&mut host_registry)?;

  let mut source_resolver = InMemorySourceResolver::new();

  // This should be parsed by the normal Mech parser.
  //
  // The runtime should register native function compilers for:
  //
  //   actor/message/payload
  //   actor/state/get
  //   actor/state/put
  //
  // so these calls resolve through the interpreter's normal function registry.
  source_resolver.insert_string(
    "actor.behavior",
    r#"
      actor/message/payload()
      actor/state/get()
      actor/state/put("{\"count\":1}")
    "#,
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

  runtime.put_object(ObjectRecord::json(
    initial_state,
    "actor-state",
    r#"{"count":0}"#,
  ))?;

  let actor = runtime.create_actor(
    "actor:native-function-counter",
    Some(actor_version),
    Some(initial_state),
    Vec::new(),
  )?;

  let message = runtime.send_message(
    actor,
    "increment",
    b"count by 1".to_vec(),
  )?;

  println!("actor:         {}", short(actor));
  println!("initial state: {}", short(initial_state));
  println!("message:       {}", short(message));

  let subject = BasicSubject::new("actor:native-function-counter");

  for (id, name) in [
    (1, "actor/message/kind"),
    (2, "actor/message/payload"),
    (3, "actor/state/id"),
    (4, "actor/state/get"),
    (5, "actor/state/put"),
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
    let reads: Vec<String> = outcome
      .access
      .reads
      .iter()
      .map(|id| short(*id))
      .collect();

    let writes: Vec<String> = outcome
      .access
      .writes
      .iter()
      .map(|id| short(*id))
      .collect();

    println!("  actor:       {:?}", outcome.actor.map(short));
    println!("  task:        {:?}", outcome.task.map(short));
    println!("  transaction: {:?}", outcome.transaction.map(short));
    println!("  reads:       {:?}", reads);
    println!("  writes:      {:?}", writes);
    println!("  events:      {}", outcome.events.len());
  }

  assert_eq!(
    outcomes.len(),
    1,
    "expected one scheduled actor turn outcome",
  );

  assert_eq!(
    outcomes[0].actor,
    Some(actor),
    "expected scheduled outcome to belong to actor",
  );

  assert!(
    runtime.peek_message(actor)?.is_none(),
    "actor message should be acked after scheduled actor turn commits",
  );

  let actor_after = runtime
    .get_actor(actor)?
    .expect("actor should exist after scheduled actor turn");

  let updated_state = actor_after
    .state
    .expect("actor should have updated state after scheduled actor turn");

  assert_ne!(
    updated_state,
    initial_state,
    "native-function behavior should update actor state pointer",
  );

  let updated_state_object = runtime
    .get_object(updated_state)?
    .expect("updated state object should exist after commit");

  assert_eq!(
    updated_state_object.data,
    br#"{"count":1}"#,
    "updated state object should contain written state",
  );

  println!();
  println!("state update:");
  println!("  old state: {}", short(initial_state));
  println!("  new state: {}", short(updated_state));
  println!(
    "  data:      {:?}",
    String::from_utf8_lossy(&updated_state_object.data),
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

    println!("  {}", short(transaction.id));
    println!("    subject:       {}", transaction.subject);
    println!("    reads:         {:?}", reads);
    println!("    writes:        {:?}", writes);
    println!("    message_acks:  {:?}", message_acks);
    println!("    message_sends: {:?}", message_sends);
    println!("    task_updates:  {:?}", task_updates);
    println!("    actor_updates: {:?}", actor_updates);
    println!("    events:        {}", transaction.events.len());
  }

  let transaction = transactions
    .iter()
    .find(|transaction| {
      transaction.subject == "actor:native-function-counter"
    })
    .expect("expected native-function actor transaction");

  assert!(
    transaction.read_set.contains(&initial_state),
    "actor/state/get should record old state read",
  );

  assert!(
    transaction.write_set.contains(&updated_state),
    "actor/state/put should record new state write",
  );

  assert!(
    transaction.message_acks.contains(&message),
    "scheduled actor turn should record message ack",
  );

  assert!(
    transaction.actor_updates.contains(&actor),
    "actor/state/put should record actor update",
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