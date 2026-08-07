use mech_runtime::ledger::RetainedTurnLedger;
use mech_runtime::outbox::RetainedEffectOutbox;
use mech_runtime::turn_record::{AccountedRecord, LedgerSequence, TurnId, TurnRecordHeader};

fn main() {
    let _ = core::mem::size_of::<RetainedTurnLedger<Box<[u8]>>>();
    let _ = core::mem::size_of::<RetainedEffectOutbox<Box<[u8]>>>();
    let _ = core::mem::size_of::<LedgerSequence>();
    let _ = core::mem::size_of::<TurnId>();
    let _ = core::mem::size_of::<TurnRecordHeader>();
    fn require_accounting<T: AccountedRecord>() {}
    require_accounting::<Box<[u8]>>();
}
