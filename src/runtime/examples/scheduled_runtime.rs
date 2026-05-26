use mech_core::MResult;

use mech_runtime::{
  InMemorySourceResolver,
  RuntimeBuilder,
  ModuleBuildOptions,
  SourceRequest,
};

fn main() -> MResult<()> {
  let mut source_resolver = InMemorySourceResolver::new();

  source_resolver.insert_string(
    "main",
    "x := 1",
  )?;

  source_resolver.insert_string(
    "actor.behavior",
    "y := 2",
  )?;

  let mut runtime = RuntimeBuilder::new()
    .source_resolver(source_resolver)
    .build()?;

  println!("runtime: {}", runtime.id());

  // ---------------------------------------------------------------------------
  // Resolve executable source into durable module versions.
  // ---------------------------------------------------------------------------

  let main_version = runtime
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
    .expect("expected `main` to resolve");

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
    .expect("expected `actor.behavior` to resolve");

  println!("main module version: {}", main_version);
  println!("actor module version: {}", actor_version);

  // ---------------------------------------------------------------------------
  // Scheduled task path.
  //
  // Expected transaction effect:
  //
  //   task_updates=[task]
  // ---------------------------------------------------------------------------

  let task = runtime.start_task(
    "task:transactional-main",
    Some(main_version),
    Vec::new(),
  )?;

  println!("task: {}", task);

  runtime.enqueue_task(task)?;

  let task_outcomes = runtime.run_tick()?;

  println!("task tick outcomes:");

  for outcome in &task_outcomes {
    println!("  {:?}", outcome);
  }

  assert_eq!(
    task_outcomes.len(),
    1,
    "expected one scheduled task outcome",
  );

  assert_eq!(
    task_outcomes[0].task,
    Some(task),
    "expected outcome to belong to scheduled task",
  );

  // ---------------------------------------------------------------------------
  // Scheduled actor path.
  //
  // Expected transaction effect:
  //
  //   message_acks=[message]
  // ---------------------------------------------------------------------------

  let actor = runtime.create_actor(
    "actor:transactional-worker",
    Some(actor_version),
    None,
    Vec::new(),
  )?;

  println!("actor: {}", actor);

  let message = runtime.send_message(
    actor,
    "ping",
    b"hello actor".to_vec(),
  )?;

  println!("actor message: {}", message);

  runtime.enqueue_actor(actor)?;

  let actor_outcomes = runtime.run_tick()?;

  println!("actor tick outcomes:");

  for outcome in &actor_outcomes {
    println!("  {:?}", outcome);
  }

  assert_eq!(
    actor_outcomes.len(),
    1,
    "expected one scheduled actor outcome",
  );

  assert_eq!(
    actor_outcomes[0].actor,
    Some(actor),
    "expected outcome to belong to scheduled actor",
  );

  assert!(
    runtime.peek_message(actor)?.is_none(),
    "actor message should be acknowledged after committed actor turn",
  );

  // ---------------------------------------------------------------------------
  // Validate durable transaction records.
  // ---------------------------------------------------------------------------

  let transactions = runtime.list_transactions(None)?;

  println!();
  println!("transactions:");

  for transaction in &transactions {
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

  let task_transaction = transactions
    .iter()
    .find(|transaction| transaction.subject == "task:transactional-main")
    .expect("expected task transaction");

  assert!(
    task_transaction.task_updates.contains(&task),
    "scheduled task transaction should record task update",
  );

  let actor_transaction = transactions
    .iter()
    .find(|transaction| transaction.subject == "actor:transactional-worker")
    .expect("expected actor transaction");

  assert!(
    actor_transaction.message_acks.contains(&message),
    "scheduled actor transaction should record message ack",
  );

  // ---------------------------------------------------------------------------
  // Shutdown and inspect event stream.
  // ---------------------------------------------------------------------------

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

  Ok(())
}