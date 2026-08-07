use mech_runtime::__gate_a_recording::{
    AccountedRecord, OwnedTurnRecordQueue, RecordEstimate, prepare_queue, reserve_queue,
};

fn main() {
    let first = OwnedTurnRecordQueue::<Box<[u8]>>::new(1, 4).unwrap();
    let second = OwnedTurnRecordQueue::<Box<[u8]>>::new(1, 4).unwrap();
    let record = vec![1_u8, 2, 3, 4].into_boxed_slice();
    let permit = reserve_queue(
        &first,
        RecordEstimate {
            records: 1,
            bytes: record.retained_bytes(),
        },
    )
    .unwrap();
    let prepared = prepare_queue(&first, permit, record).unwrap();

    second.append(prepared);
}
