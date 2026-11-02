use std::collections::VecDeque;

use crate::{
    ActorId, CapabilityId, EventId, IdGenerator, MessageId, NodeId, ObjectId, RuntimeId, TaskId,
    TransactionId,
};

#[derive(Debug)]
pub(crate) struct ScriptedIdGenerator {
    next: u128,
}

impl ScriptedIdGenerator {
    pub(crate) fn new(next: u128) -> Self {
        Self { next }
    }

    fn next_id(&mut self) -> u128 {
        let id = self.next;
        self.next = self.next.saturating_add(1);
        id
    }
}

impl IdGenerator for ScriptedIdGenerator {
    fn runtime_id(&mut self) -> RuntimeId {
        RuntimeId(self.next_id())
    }

    fn object_id(&mut self) -> ObjectId {
        ObjectId(self.next_id())
    }

    fn actor_id(&mut self) -> ActorId {
        ActorId(self.next_id())
    }

    fn task_id(&mut self) -> TaskId {
        TaskId(self.next_id())
    }

    fn capability_id(&mut self) -> CapabilityId {
        CapabilityId(self.next_id())
    }

    fn transaction_id(&mut self) -> TransactionId {
        TransactionId(self.next_id())
    }

    fn event_id(&mut self) -> EventId {
        EventId(self.next_id())
    }

    fn node_id(&mut self) -> NodeId {
        NodeId(self.next_id())
    }

    fn message_id(&mut self) -> MessageId {
        MessageId(self.next_id())
    }
}

#[derive(Debug)]
pub(crate) struct ScriptedEventIdGenerator {
    fallback: ScriptedIdGenerator,
    event_ids: VecDeque<EventId>,
}

impl ScriptedEventIdGenerator {
    pub(crate) fn new(next: u128, event_ids: impl IntoIterator<Item = EventId>) -> Self {
        Self {
            fallback: ScriptedIdGenerator::new(next),
            event_ids: event_ids.into_iter().collect(),
        }
    }
}

impl IdGenerator for ScriptedEventIdGenerator {
    fn runtime_id(&mut self) -> RuntimeId {
        RuntimeId(self.fallback.next_id())
    }

    fn object_id(&mut self) -> ObjectId {
        ObjectId(self.fallback.next_id())
    }

    fn actor_id(&mut self) -> ActorId {
        ActorId(self.fallback.next_id())
    }

    fn task_id(&mut self) -> TaskId {
        TaskId(self.fallback.next_id())
    }

    fn capability_id(&mut self) -> CapabilityId {
        CapabilityId(self.fallback.next_id())
    }

    fn transaction_id(&mut self) -> TransactionId {
        TransactionId(self.fallback.next_id())
    }

    fn event_id(&mut self) -> EventId {
        self.event_ids
            .pop_front()
            .unwrap_or_else(|| EventId(self.fallback.next_id()))
    }

    fn node_id(&mut self) -> NodeId {
        NodeId(self.fallback.next_id())
    }

    fn message_id(&mut self) -> MessageId {
        MessageId(self.fallback.next_id())
    }
}
