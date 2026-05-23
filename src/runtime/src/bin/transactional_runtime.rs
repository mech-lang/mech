use mech_core::MResult;

use mech_runtime::{
  InMemorySourceResolver,
  MessageRecord,
  RuntimeBuilder,
  RuntimeContextBuilder,
};

fn main() -> MResult<()> {
  let source_resolver = InMemorySourceResolver::new();

  let mut runtime = RuntimeBuilder::new()
    .source_resolver(source_resolver)
    .build()?;

  println!("runtime: {}", runtime.id());

  // ---------------------------------------------------------------------------
  // Setup: create an actor and send it two durable messages.
  // ---------------------------------------------------------------------------

  let actor = runtime.create_actor(
    "actor:transactional-mailbox",
    None,
    None,
    Vec::new(),
  )?;

  println!("actor: {}", actor);

  let first_message = runtime.send_message(
    actor,
    "first",
    b"keep me after abort".to_vec(),
  )?;

  let second_message = runtime.send_message(
    actor,
    "second",
    b"remove me after commit".to_vec(),
  )?;

  println!("first message: {}", first_message);
  println!("second message: {}", second_message);

  let initial_peek = runtime
    .peek_message(actor)?
    .expect("expected first message in mailbox");

  assert_eq!(initial_peek.id, first_message);
  assert_eq!(initial_peek.kind, "first");

  println!(
    "initial mailbox front: id={} kind={} payload={:?}",
    initial_peek.id,
    initial_peek.kind,
    String::from_utf8_lossy(&initial_peek.payload),
  );

  // ---------------------------------------------------------------------------
  // Abort path.
  //
  // We pop inside a transaction. The runtime should stage an ack, not remove the
  // message from the durable mailbox. Aborting should discard the staged ack.
  // ---------------------------------------------------------------------------

  let mut abort_context = RuntimeContextBuilder::new(runtime.id())
    .subject("actor:transactional-mailbox")
    .actor(actor)
    .build()?;

  let abort_transaction = runtime.begin_transaction(&mut abort_context)?;

  println!("abort transaction started: {}", abort_transaction);

  let popped_inside_abort = runtime
    .pop_message_with_context(&mut abort_context, actor)?
    .expect("expected first message inside abort transaction");

  assert_eq!(popped_inside_abort.id, first_message);

  println!(
    "popped inside abort transaction: id={} kind={} payload={:?}",
    popped_inside_abort.id,
    popped_inside_abort.kind,
    String::from_utf8_lossy(&popped_inside_abort.payload),
  );

  runtime.abort_runtime_transaction(
    &mut abort_context,
    "rollback staged mailbox ack",
  )?;

  let after_abort_peek = runtime
    .peek_message(actor)?
    .expect("message should remain after abort");

  assert_eq!(after_abort_peek.id, first_message);

  println!(
    "after abort mailbox front is still: id={} kind={} payload={:?}",
    after_abort_peek.id,
    after_abort_peek.kind,
    String::from_utf8_lossy(&after_abort_peek.payload),
  );

  // ---------------------------------------------------------------------------
  // Commit path for the first message.
  //
  // This time the staged ack should be applied at commit.
  // ---------------------------------------------------------------------------

  let mut commit_context = RuntimeContextBuilder::new(runtime.id())
    .subject("actor:transactional-mailbox")
    .actor(actor)
    .build()?;

  let commit_transaction = runtime.begin_transaction(&mut commit_context)?;

  println!("commit transaction started: {}", commit_transaction);

  let popped_inside_commit = runtime
    .pop_message_with_context(&mut commit_context, actor)?
    .expect("expected first message inside commit transaction");

  assert_eq!(popped_inside_commit.id, first_message);

  println!(
    "popped inside commit transaction: id={} kind={} payload={:?}",
    popped_inside_commit.id,
    popped_inside_commit.kind,
    String::from_utf8_lossy(&popped_inside_commit.payload),
  );

  let committed = runtime.commit_runtime_transaction(&mut commit_context)?;

  println!("commit transaction committed: {}", committed);

  let after_commit_peek = runtime
    .peek_message(actor)?
    .expect("expected second message after first ack commit");

  assert_eq!(after_commit_peek.id, second_message);
  assert_eq!(after_commit_peek.kind, "second");

  println!(
    "after first commit mailbox front advanced to: id={} kind={} payload={:?}",
    after_commit_peek.id,
    after_commit_peek.kind,
    String::from_utf8_lossy(&after_commit_peek.payload),
  );

  // ---------------------------------------------------------------------------
  // Commit path for the second message.
  //
  // This proves the mailbox can be drained through staged acks.
  // ---------------------------------------------------------------------------

  let mut final_context = RuntimeContextBuilder::new(runtime.id())
    .subject("actor:transactional-mailbox")
    .actor(actor)
    .build()?;

  let final_transaction = runtime.begin_transaction(&mut final_context)?;

  println!("final transaction started: {}", final_transaction);

  let popped_second = runtime
    .pop_message_with_context(&mut final_context, actor)?
    .expect("expected second message inside final transaction");

  assert_eq!(popped_second.id, second_message);

  println!(
    "popped second message: id={} kind={} payload={:?}",
    popped_second.id,
    popped_second.kind,
    String::from_utf8_lossy(&popped_second.payload),
  );

  let final_committed = runtime.commit_runtime_transaction(&mut final_context)?;

  println!("final transaction committed: {}", final_committed);

  assert!(
    runtime.peek_message(actor)?.is_none(),
    "mailbox should be empty after both staged acks commit",
  );

  println!("mailbox is empty after committed staged acks");

  // ---------------------------------------------------------------------------
  // Also prove staged enqueues are invisible until commit.
  // ---------------------------------------------------------------------------

  let mut enqueue_abort_context = RuntimeContextBuilder::new(runtime.id())
    .subject("actor:transactional-mailbox")
    .actor(actor)
    .build()?;

  let enqueue_abort_transaction =
    runtime.begin_transaction(&mut enqueue_abort_context)?;

  println!(
    "staged enqueue abort transaction started: {}",
    enqueue_abort_transaction,
  );

  let staged_aborted_message = runtime.send_message_with_context(
    &mut enqueue_abort_context,
    actor,
    "staged-abort",
    b"discard me".to_vec(),
  )?;

  println!(
    "staged enqueue inside abort transaction: {}",
    staged_aborted_message,
  );

  assert!(
    runtime.peek_message(actor)?.is_none(),
    "staged enqueue should not be visible outside transaction before abort",
  );

  let staged_inside_abort = runtime
    .peek_message_with_context(&mut enqueue_abort_context, actor)?
    .expect("staged enqueue should be visible inside transaction");

  assert_eq!(staged_inside_abort.id, staged_aborted_message);

  runtime.abort_runtime_transaction(
    &mut enqueue_abort_context,
    "discard staged enqueue",
  )?;

  assert!(
    runtime.peek_message(actor)?.is_none(),
    "staged enqueue should be discarded after abort",
  );

  println!(
    "staged enqueue discarded after abort: {}",
    staged_aborted_message,
  );

  let mut enqueue_commit_context = RuntimeContextBuilder::new(runtime.id())
    .subject("actor:transactional-mailbox")
    .actor(actor)
    .build()?;

  let enqueue_commit_transaction =
    runtime.begin_transaction(&mut enqueue_commit_context)?;

  println!(
    "staged enqueue commit transaction started: {}",
    enqueue_commit_transaction,
  );

  let staged_committed_message = runtime.send_message_with_context(
    &mut enqueue_commit_context,
    actor,
    "staged-commit",
    b"persist me".to_vec(),
  )?;

  println!(
    "staged enqueue inside commit transaction: {}",
    staged_committed_message,
  );

  assert!(
    runtime.peek_message(actor)?.is_none(),
    "staged enqueue should not be visible outside transaction before commit",
  );

  let staged_inside_commit = runtime
    .peek_message_with_context(&mut enqueue_commit_context, actor)?
    .expect("staged enqueue should be visible inside transaction");

  assert_eq!(staged_inside_commit.id, staged_committed_message);

  runtime.commit_runtime_transaction(&mut enqueue_commit_context)?;

  let committed_staged_message = runtime
    .peek_message(actor)?
    .expect("staged enqueue should be visible outside transaction after commit");

  assert_eq!(committed_staged_message.id, staged_committed_message);

  println!(
    "staged enqueue persisted after commit: id={} kind={} payload={:?}",
    committed_staged_message.id,
    committed_staged_message.kind,
    String::from_utf8_lossy(&committed_staged_message.payload),
  );

  // ---------------------------------------------------------------------------
  // Shutdown and inspect the event/transaction streams.
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