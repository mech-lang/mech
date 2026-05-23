use std::sync::Arc;

use mech_core::{MResult, Value};

use mech_runtime::{
  BasicCapability,
  BasicCapabilityKernel,
  BasicOperation,
  BasicResource,
  BasicSubject,
  CapabilityId,
  ClosureHostFunction,
  HostCall,
  InMemoryHostRegistry,
  InMemorySourceResolver,
  ObjectRecord,
  RuntimeBuilder,
  RuntimeContextBuilder,
  RuntimeTurnOutcome,
  ScheduledWork,
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

  let mut host_registry = InMemoryHostRegistry::new();

  host_registry.insert(ClosureHostFunction::new(
    "host.empty",
    |_ctx, _args| Ok(Value::Empty),
  ))?;

  let mut runtime = RuntimeBuilder::new()
    .source_resolver(source_resolver)
    .host_registry(host_registry)
    .capability_kernel(BasicCapabilityKernel::new())
    .build()?;

  println!("runtime: {}", runtime.id());

  // ---------------------------------------------------------------------------
  // Resolve and store executable source as module versions.
  // ---------------------------------------------------------------------------

  let main_version = runtime
    .resolve_and_store_module_source(
      SourceRequest::new("main"),
      env!("CARGO_PKG_VERSION"),
      "mech-current",
      &[],
      &[],
    )?
    .expect("expected `main` to resolve");

  let actor_version = runtime
    .resolve_and_store_module_source(
      SourceRequest::new("actor.behavior"),
      env!("CARGO_PKG_VERSION"),
      "mech-current",
      &[],
      &[],
    )?
    .expect("expected `actor.behavior` to resolve");

  println!("main module version: {}", main_version);
  println!("actor module version: {}", actor_version);

  // ---------------------------------------------------------------------------
  // Task scheduling path.
  // ---------------------------------------------------------------------------

  let task = runtime.start_task(
    "task:transactional-main",
    Some(main_version),
    Vec::new(),
  )?;

  println!("task: {}", task);

  runtime.enqueue_task(task)?;

  let tick = runtime.collect_tick()?;

  println!("tick selected {} item(s)", tick.len());

  for work in tick.work {
    println!("running scheduled work: {}", work.label());

    match runtime.run_scheduled_work(work) {
      Ok(outcome) => {
        println!("scheduled work completed: {:?}", outcome);
      }
      Err(error) => {
        println!("scheduled work failed: {:?}", error);
      }
    }
  }

  // ---------------------------------------------------------------------------
  // Actor scheduling path.
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

  let actor_tick = runtime.collect_tick()?;

  println!("actor tick selected {} item(s)", actor_tick.len());

  for work in actor_tick.work {
    println!("running scheduled work: {}", work.label());

    match runtime.run_scheduled_work(work) {
      Ok(outcome) => {
        println!("actor work completed: {:?}", outcome);
      }
      Err(error) => {
        println!("actor work failed: {:?}", error);
      }
    }
  }

  // ---------------------------------------------------------------------------
  // Manual staged transaction path for object writes.
  // ---------------------------------------------------------------------------

  let mut context = RuntimeContextBuilder::new(runtime.id())
    .subject("task:manual-transaction")
    .build()?;

  let transaction = runtime.begin_transaction(&mut context)?;

  println!("manual transaction started: {}", transaction);

  let object_id = runtime.next_object_id();

  let object = ObjectRecord::text(
    object_id,
    "note",
    "hello from a staged transactional context",
  );

  let staged_object = runtime.put_object_with_context(
    &mut context,
    object,
  )?;

  println!("staged object: {}", staged_object);

  assert!(
    runtime.get_object(object_id)?.is_none(),
    "object should not be visible outside the transaction before commit",
  );

  let visible_inside_transaction = runtime
    .get_object_with_context(&mut context, object_id)?
    .expect("object should be visible inside the active transaction");

  println!(
    "staged object visible inside transaction: kind={} encoding={} data={:?}",
    visible_inside_transaction.kind,
    visible_inside_transaction.encoding,
    String::from_utf8_lossy(&visible_inside_transaction.data),
  );

  let committed = runtime.commit_runtime_transaction(&mut context)?;

  println!("manual transaction committed: {}", committed);

  assert!(
    runtime.get_object(object_id)?.is_some(),
    "object should be visible outside the transaction after commit",
  );

  let committed_object = runtime
    .get_object(object_id)?
    .expect("object should exist after commit");

  println!(
    "committed object visible outside transaction: kind={} encoding={} data={:?}",
    committed_object.kind,
    committed_object.encoding,
    String::from_utf8_lossy(&committed_object.data),
  );

  // ---------------------------------------------------------------------------
  // Aborted staged transaction path.
  // ---------------------------------------------------------------------------

  let mut abort_context = RuntimeContextBuilder::new(runtime.id())
    .subject("task:aborted-transaction")
    .build()?;

  let aborted_transaction = runtime.begin_transaction(&mut abort_context)?;

  println!("abort transaction started: {}", aborted_transaction);

  let aborted_object_id = runtime.next_object_id();

  runtime.put_object_with_context(
    &mut abort_context,
    ObjectRecord::text(
      aborted_object_id,
      "note",
      "this should be discarded",
    ),
  )?;

  assert!(
    runtime.get_object(aborted_object_id)?.is_none(),
    "aborted object should not be visible outside the transaction before abort",
  );

  assert!(
    runtime
      .get_object_with_context(&mut abort_context, aborted_object_id)?
      .is_some(),
    "aborted object should be visible inside the transaction before abort",
  );

  runtime.abort_runtime_transaction(
    &mut abort_context,
    "discard staged object",
  )?;

  assert!(
    runtime.get_object(aborted_object_id)?.is_none(),
    "aborted object should not be visible after abort",
  );

  println!("abort transaction discarded object: {}", aborted_object_id);

  // ---------------------------------------------------------------------------
  // Host call path.
  // ---------------------------------------------------------------------------

  let subject = BasicSubject::new("task:host-example");
  let resource = BasicResource::new("host:host.empty");

  let capability = BasicCapability::new(
    CapabilityId(1),
    &subject,
    &resource,
    [BasicOperation::new("call")],
  );

  runtime.grant_capability(Arc::new(capability))?;

  let mut host_context = RuntimeContextBuilder::new(runtime.id())
    .subject("task:host-example")
    .build()?;

  let host_result = runtime.call_host_with_context(
    &mut host_context,
    HostCall::new("host.empty", Vec::new()),
  )?;

  println!("host call result: {:?}", host_result);

  // ---------------------------------------------------------------------------
  // Direct scheduler completion API.
  //
  // This is just a direct API smoke test. It does not execute work.
  // ---------------------------------------------------------------------------

  let synthetic_work = ScheduledWork::task(runtime.next_task_id());

  runtime.enqueue_work(synthetic_work)?;

  let synthetic_outcome = RuntimeTurnOutcome::new();

  runtime.complete_scheduled_work(
    synthetic_work,
    synthetic_outcome,
  )?;

  // ---------------------------------------------------------------------------
  // Shutdown and inspect durable event/transaction streams.
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

  println!();
  println!("transactions:");

  for transaction in runtime.list_transactions(None)? {
    println!(
      "  {} subject={} reads={:?} writes={:?} events={:?}",
      transaction.id,
      transaction.subject,
      transaction.read_set,
      transaction.write_set,
      transaction.events,
    );
  }

  Ok(())
}