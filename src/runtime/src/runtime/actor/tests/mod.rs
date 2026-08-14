mod limits;
mod transactional_mailbox;

use crate::{ActorId, ActorRecord, MechRuntime};

fn runtime_with_actor_and_messages(payloads: &[&[u8]]) -> MechRuntime {
    let mut runtime = MechRuntime::builder().build().unwrap();
    runtime
        .put_actor(ActorRecord::new(ActorId(1), "actor:1"))
        .unwrap();

    for payload in payloads {
        runtime
            .send_message(ActorId(1), "ping", payload.to_vec())
            .unwrap();
    }

    runtime
}
