use mech_runtime::__gate_a_recording::AccountedRecord;

struct HugePayload(Vec<u8>);

impl AccountedRecord for HugePayload {
    fn retained_bytes(&self) -> usize {
        0
    }
}

fn main() {}
