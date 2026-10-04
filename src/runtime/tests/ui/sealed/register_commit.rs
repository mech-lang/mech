use mech_core::{PreparedCellPublicationBatch, ReadyPublicationBatch};

fn main() {
    let prepared = PreparedCellPublicationBatch::new(Vec::new()).unwrap();
    let forged_ready = ReadyPublicationBatch {
        prepared,
        completed: false,
    };
    forged_ready.commit();
}
