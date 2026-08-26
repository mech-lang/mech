use mech_runtime::turn_record::sealed::Sealed;

struct HugePayload(Vec<u8>);

impl Sealed for HugePayload {}

fn main() {}
