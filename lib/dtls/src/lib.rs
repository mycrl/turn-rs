
pub struct Message {
    kind: u8,
    version: u16,
    epoch: u16,
    sequence_number: u64,
    payload: Payload
}
