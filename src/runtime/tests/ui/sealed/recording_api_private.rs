use mech_runtime::ledger::RetainedTurnLedger;
use mech_runtime::outbox::RetainedEffectOutbox;
use mech_runtime::turn_record::{AccountedRecord, LedgerSequence, TurnId, TurnRecordHeader};

fn main() {
    assert_ne!(core::mem::size_of::<RetainedTurnLedger<Box<[u8]>>>(), 0);
    assert_ne!(core::mem::size_of::<RetainedEffectOutbox<Box<[u8]>>>(), 0);
    assert_ne!(core::mem::size_of::<LedgerSequence>(), 0);
    assert_ne!(core::mem::size_of::<TurnId>(), 0);
    assert_ne!(core::mem::size_of::<TurnRecordHeader>(), 0);
    fn require_accounting<T: AccountedRecord>() {}
    require_accounting::<Box<[u8]>>();
}
